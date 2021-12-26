use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

pub(crate) struct Repository {
    inner: git2::Repository,
}

impl Repository {
    pub(crate) fn from_current_dir() -> Result<Self> {
        let inner = git2::Repository::discover(env::current_dir()?)?;
        if inner.is_bare() {
            bail!("Cannot be used in a bare repository");
        }
        Ok(Self { inner })
    }

    pub(crate) fn workdir(&self) -> &Path {
        self.inner.workdir().unwrap() // None in case of bare repo
    }

    pub(crate) fn get_sidecar(&self, path: impl AsRef<Path>, extension: &str) -> Result<PathBuf> {
        let relpath = path.as_ref().strip_prefix(self.workdir())?;
        let name = relpath.to_string_lossy().replace('/', "!");

        let dir = self.inner.path().join("git-agenix");
        fs::create_dir_all(&dir)?;

        let mut rv = dir.join(name);
        rv.set_extension(extension);
        Ok(rv)
    }

    pub(crate) fn get_file_contents(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let relpath = path.as_ref().strip_prefix(self.workdir())?;
        let entry = self.inner.head()?.peel_to_tree()?.get_path(relpath)?;
        let contents = entry.to_object(&self.inner)?;

        Ok(contents.as_blob().unwrap().content().into())
    }
}
