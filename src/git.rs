use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{:?} already exists.", .0)]
    AlreadyExists(String),
    #[error("{:?} doesn't exist.", .0)]
    NotExist(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<git2::Error> for Error {
    fn from(err: git2::Error) -> Self {
        Self::Other(anyhow!(err))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) trait Repository {
    fn workdir(&self) -> &Path;

    fn path(&self) -> &Path;

    fn get_file_contents(&self, path: &Path) -> Result<Vec<u8>>;

    fn add_config(&self, key: &str, value: &str) -> Result<()>;

    fn contains_config(&self, key: &str, value: &str) -> bool;

    fn remove_config(&self, key: &str, value: &str) -> Result<()>;

    fn list_config(&self, key: &str) -> Result<Vec<String>>;

    fn get_config(&self, key: &str) -> Result<String>;

    fn set_config(&self, key: &str, value: &str) -> Result<()>;
}

pub(crate) struct LibGit2Repository {
    inner: git2::Repository,
}

impl LibGit2Repository {
    pub(crate) fn from_current_dir() -> Result<Self> {
        Self::from_dir(env::current_dir().context("Cannot determine current directory")?)
    }

    pub(crate) fn from_dir(path: PathBuf) -> Result<Self> {
        let inner = git2::Repository::discover(&path)
            .with_context(|| format!("'{}' Not a git repository", path.display()))?;
        if inner.is_bare() {
            return Err(anyhow!("Bare repositories are unsupported {}", path.display(),).into());
        }
        Ok(Self { inner })
    }
}

impl Repository for LibGit2Repository {
    fn workdir(&self) -> &Path {
        self.inner.workdir().unwrap() // None in case of bare repo
    }

    fn path(&self) -> &Path {
        self.inner.path()
    }

    fn get_file_contents(&self, path: &Path) -> Result<Vec<u8>> {
        let relpath = path.strip_prefix(self.workdir()).with_context(|| {
            format!(
                "Path {} is outside of git repository {}",
                path.display(),
                self.workdir().display()
            )
        })?;
        let entry = self
            .inner
            .head()
            .context("Couldn not determine repository head")?
            .peel_to_tree()?
            .get_path(relpath)
            .with_context(|| format!("Path {} is not found in HEAD", relpath.display()))?;
        let contents = entry.to_object(&self.inner)?;

        Ok(contents.as_blob().unwrap().content().into())
    }

    fn add_config(&self, key: &str, value: &str) -> Result<()> {
        if self.contains_config(key, value) {
            return Err(Error::AlreadyExists(value.into()));
        }

        let mut cfg = self.inner.config()?;

        cfg.set_multivar(key, "^$", value)?;

        Ok(())
    }

    fn contains_config(&self, key: &str, value: &str) -> bool {
        let entries = self.list_config(key).unwrap_or_default();
        entries.iter().any(|e| e == value)
    }

    fn remove_config(&self, key: &str, value: &str) -> Result<()> {
        if !self.contains_config(key, value) {
            return Err(Error::NotExist(key.into()));
        }

        let mut cfg = self.inner.config()?;
        let pattern = format!("^{}$", regex::escape(value));
        cfg.remove_multivar(key, &pattern)?;

        Ok(())
    }

    fn list_config(&self, key: &str) -> Result<Vec<String>> {
        let cfg = self.inner.config()?;
        let entries = cfg
            .entries(Some(key))?
            .filter_map(|e| e.ok().and_then(|e| e.value().map(|e| e.to_owned())))
            .collect();

        Ok(entries)
    }

    fn get_config(&self, key: &str) -> Result<String> {
        let cfg = self.inner.config()?;
        cfg.get_string(key)
            .map_err(|_e| Error::NotExist(key.into()))
    }

    fn set_config(&self, key: &str, value: &str) -> Result<()> {
        let mut cfg = self.inner.config()?;
        self.get_config(key)
            .and_then(|_| Err(Error::AlreadyExists(key.into())))
            .or_else(|_| {
                cfg.set_str(key, value)?;
                Ok(())
            })
    }
}

#[cfg(test)]
mod tests {
    use assert_fs::prelude::*;
    use assert_fs::TempDir;
    use assert_matches::assert_matches;
    use duct::cmd;

    use super::*;

    fn setup_git_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        cmd!("git", "init").dir(dir.path()).run().unwrap();
        dir
    }

    #[test]
    fn test_repo_can_be_loaded() {
        let repo_dir = setup_git_repo();
        let repo = LibGit2Repository::from_dir(repo_dir.path().to_path_buf());

        assert!(repo.is_ok());
        let repo = repo.unwrap();

        assert_eq!(repo.workdir(), repo_dir.path());
    }

    #[test]
    fn test_repo_required() {
        let dir = TempDir::new().unwrap();
        let repo = LibGit2Repository::from_dir(dir.path().to_path_buf());
        assert!(matches!(repo, Err(Error::Other(_))));
    }

    #[test]
    fn test_bare_repo_is_error() {
        let dir = TempDir::new().unwrap();
        cmd!("git", "init", "--bare").dir(dir.path()).run().unwrap();
        let repo = LibGit2Repository::from_dir(dir.path().to_path_buf());
        assert!(matches!(repo, Err(Error::Other(_))));
    }

    #[test]
    fn test_get_file_contents() {
        let repo_dir = setup_git_repo();

        let path = PathBuf::from("subdir/file.txt");
        let file_contents = "file contents";

        let repo_file = repo_dir.child(&path);
        repo_file.touch().unwrap();
        repo_file.write_str(file_contents).unwrap();
        cmd!("git", "add", &path)
            .dir(repo_dir.path())
            .run()
            .unwrap();
        cmd!("git", "commit", "-m", "adding file")
            .dir(repo_dir.path())
            .run()
            .unwrap();

        let repo = LibGit2Repository::from_dir(repo_dir.path().to_path_buf()).unwrap();
        assert_matches!(repo.get_file_contents(&repo_dir.join(&path)), Ok(contents) if contents == file_contents.as_bytes());

        repo_file.write_str("additional_contents").unwrap();
        assert_matches!(repo.get_file_contents(&repo_dir.join(&path)), Ok(contents) if contents == file_contents.as_bytes());
    }
}
