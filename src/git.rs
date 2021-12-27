use std::{
    env,
    path::{Path},
};

use anyhow::{bail, Result};

pub(crate) trait Repository {
    fn workdir(&self) -> &Path;

    fn path(&self) -> &Path;

    fn get_file_contents(&self, path: &Path) -> Result<Vec<u8>>;

    fn add_config(&self, key: &str, value: &str) -> Result<bool>;

    fn contains_config(&self, key: &str, value: &str) -> bool;

    fn remove_config(
        &self,
        key: &str,
        value: &str,
    ) -> Result<bool>;
    fn list_config(&self, key: &str) -> Result<Vec<String>>;

    fn get_config(&self, key: &str) -> Option<ConfigValue>;

    fn set_config(&self, key: &str, value: ConfigValue) -> Result<bool>;

}

pub(crate) struct LibGit2Repository {
    inner: git2::Repository,
}

pub(crate) enum ConfigValue {
    Bool(bool),
    String(String),
}

impl From<bool> for ConfigValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<String> for ConfigValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl LibGit2Repository {
    pub(crate) fn from_current_dir() -> Result<Self> {
        let inner = git2::Repository::discover(env::current_dir()?)?;
        if inner.is_bare() {
            bail!("Cannot be used in a bare repository");
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

    fn add_config(&self, key: &str, value: &str) -> Result<bool> {
        if self.contains_config(key, value) {
            return Ok(false);
        }

        let mut cfg = self.inner.config()?;

        cfg.set_multivar(key, "^$", value)?;

        Ok(true)
    }

    fn contains_config(&self, key: &str, value: &str) -> bool {
        let entries = self.list_config(key).unwrap_or_default();
        entries.iter().any(|e| e == value)
    }

    fn remove_config(
        &self,
        key: &str,
        value: &str,
    ) -> Result<bool> {
        if !self.contains_config(key, value) {
            return Ok(false);
        }

        let mut cfg = self.inner.config()?;
        let pattern = format!("^{}$", regex::escape(value));
        cfg.remove_multivar(key, &pattern)?;

        Ok(true)
    }

    fn list_config(&self, key: &str) -> Result<Vec<String>> {
        let cfg = self.inner.config()?;
        let entries = cfg
            .entries(Some(key))?
            .filter_map(|e| e.ok().and_then(|e| e.value().map(|e| e.to_owned())))
            .collect();

        Ok(entries)
    }

    fn get_config(&self, key: &str) -> Option<ConfigValue> {
        let cfg = self.inner.config().ok()?;
        if let Ok(value) = cfg.get_bool(key) {
            Some(value.into())
        } else if let Ok(value) = cfg.get_string(key) {
            Some(value.into())
        } else {
            None
        }
    }

    fn set_config(&self, key: &str, value: ConfigValue) -> Result<bool> {
        let mut cfg = self.inner.config()?;
        let contains = self.get_config(key).is_some();
        match value {
            ConfigValue::Bool(b) => cfg.set_bool(key, b)?,
            ConfigValue::String(s) => cfg.set_str(key, &s)?,
        }
        Ok(!contains)
    }
}
