use std::{
    env, io,
    path::{Path, PathBuf},
    process,
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

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
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

    fn remove_config_section(&self, key: &str) -> Result<()>;
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
            return Err(Error::NotExist(value.into()));
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
        if self.get_config(key).is_ok() {
            return Err(Error::AlreadyExists(key.into()));
        }
        let mut cfg = self.inner.config()?;
        cfg.set_str(key, value)?;
        Ok(())
    }

    fn remove_config_section(&self, key: &str) -> Result<()> {
        // Unfortunately there is no `git config --remove-section <section>` equivalent in libgit2
        let mut command = process::Command::new("git");
        command
            .current_dir(self.workdir())
            .arg("config")
            .arg("--remove-section")
            .arg(key);
        let output = command.output()?;

        if !output.status.success() {
            log::error!(
                "Failed to execute command. This may not be an issue; command='{:?}' status='{}', stdout={:?}, stderr={:?}",
                command,
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return Err(Error::NotExist(key.into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use anyhow::Result;
    use assert_fs::prelude::*;
    use assert_fs::TempDir;
    use assert_matches::assert_matches;
    use duct::cmd;
    use rstest::fixture;
    use rstest::rstest;

    use super::*;

    #[fixture]
    fn tempdir() -> TempDir {
        TempDir::new().unwrap()
    }

    struct Repo {
        inner: LibGit2Repository,
        dir: TempDir,
    }

    impl Deref for Repo {
        type Target = LibGit2Repository;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    #[fixture]
    fn git_repo(tempdir: TempDir) -> Repo {
        cmd!("git", "init").dir(tempdir.path()).run().unwrap();
        let repo = LibGit2Repository::from_dir(tempdir.path().to_path_buf()).unwrap();
        Repo {
            inner: repo,
            dir: tempdir,
        }
    }

    #[rstest]
    fn test_repo_can_be_loaded(git_repo: Repo) -> Result<()> {
        assert_eq!(git_repo.workdir(), git_repo.dir.path());
        assert_eq!(git_repo.path(), git_repo.dir.join(".git"));
        Ok(())
    }

    #[rstest]
    fn test_repo_required(tempdir: TempDir) -> Result<()> {
        let repo = LibGit2Repository::from_dir(tempdir.path().to_path_buf()).err();
        assert_matches!(repo, Some(Error::Other(_)));
        Ok(())
    }

    #[rstest]
    fn test_bare_repo_is_error(tempdir: TempDir) -> Result<()> {
        cmd!("git", "init", "--bare").dir(tempdir.path()).run()?;
        let repo = LibGit2Repository::from_dir(tempdir.path().to_path_buf()).err();
        assert_matches!(repo, Some(Error::Other(_)));
        Ok(())
    }

    #[rstest]
    fn test_get_file_contents(git_repo: Repo) -> Result<()> {
        let path = PathBuf::from("subdir/file.txt");
        let file_contents = "file contents";

        let repo_file = git_repo.dir.child(&path);
        repo_file.touch()?;
        repo_file.write_str(file_contents)?;
        cmd!("git", "add", &path).dir(git_repo.dir.path()).run()?;
        cmd!("git", "commit", "-m", "adding file")
            .dir(git_repo.dir.path())
            .run()?;

        assert_eq!(
            git_repo.get_file_contents(&git_repo.dir.join(&path))?,
            file_contents.as_bytes()
        );

        repo_file.write_str("additional_contents")?;
        assert_eq!(
            git_repo.get_file_contents(&git_repo.dir.join(&path))?,
            file_contents.as_bytes()
        );

        assert!(git_repo
            .get_file_contents(git_repo.dir.path().parent().unwrap())
            .is_err());

        Ok(())
    }

    #[rstest]
    fn test_config(git_repo: Repo) -> Result<()> {
        // At first there are no entries under the "foo" section
        assert_eq!(git_repo.list_config("foo")?, [] as [String; 0]);
        assert!(!git_repo.contains_config("foo.bar", "foobar"));

        assert_matches!(git_repo.set_config("invalidkey", "foobar"), Err(_));

        // Both of these are added to the config
        git_repo.set_config("foo.bar", "foobar")?;
        git_repo.add_config("foo.bar", "snafu")?;

        // Set won't work with multivalue entries
        assert_matches!(
            git_repo.set_config("foo.bar", "FOOBAR"),
            Err(Error::AlreadyExists(key)) if key == "foo.bar"
        );

        assert!(git_repo.contains_config("foo.bar", "foobar"));
        assert!(git_repo.contains_config("foo.bar", "snafu"));

        assert_eq!(git_repo.list_config("foo")?, ["foobar", "snafu"]);

        // Returns the last set config
        assert_eq!(git_repo.get_config("foo.bar")?, "snafu");

        // Duplicate value cannot be added
        assert_matches!(
            git_repo.add_config("foo.bar", "foobar"),
            Err(Error::AlreadyExists(value)) if value == "foobar"
        );

        git_repo.remove_config("foo.bar", "foobar")?;

        // Absent value cannot be removed
        assert_matches!(
            git_repo.remove_config("foo.bar", "foobar"), Err(Error::NotExist(value)) if value == "foobar"
        );

        assert_matches!(git_repo.remove_config_section("foo"), Ok(()));
        assert_eq!(git_repo.list_config("foo")?, [] as [String; 0]);

        Ok(())
    }
}
