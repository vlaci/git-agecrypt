use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

pub(crate) struct Repository {
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

    pub(crate) fn path(&self) -> &Path {
        self.inner.path()
    }

    pub(crate) fn get_file_contents(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let relpath = path.as_ref().strip_prefix(self.workdir())?;
        let entry = self.inner.head()?.peel_to_tree()?.get_path(relpath)?;
        let contents = entry.to_object(&self.inner)?;

        Ok(contents.as_blob().unwrap().content().into())
    }

    pub(crate) fn add_config(&self, key: impl AsRef<str>, value: impl AsRef<str>) -> Result<bool> {
        if self.contains_config(&key, &value) {
            return Ok(false);
        }

        let mut cfg = self.inner.config()?;

        cfg.set_multivar(key.as_ref(), "^$", value.as_ref())?;

        Ok(true)
    }

    fn contains_config(&self, key: impl AsRef<str>, value: impl AsRef<str>) -> bool {
        let entries = self.list_config(key).unwrap_or_default();
        entries.iter().any(|e| e == value.as_ref())
    }

    pub(crate) fn remove_config(
        &self,
        key: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<bool> {
        if !self.contains_config(&key, &value) {
            return Ok(false);
        }

        let mut cfg = self.inner.config()?;
        let pattern = format!("^{}$", regex::escape(value.as_ref()));
        cfg.remove_multivar(key.as_ref(), &pattern)?;

        Ok(true)
    }

    pub(crate) fn list_config(&self, key: impl AsRef<str>) -> Result<Vec<String>> {
        let cfg = self.inner.config()?;
        let entries = cfg
            .entries(Some(key.as_ref()))?
            .filter_map(|e| e.ok().and_then(|e| e.value().map(|e| e.to_owned())))
            .collect();

        Ok(entries)
    }

    pub(crate) fn get_config(&self, key: impl AsRef<str>) -> Option<ConfigValue> {
        let cfg = self.inner.config().ok()?;
        if let Ok(value) = cfg.get_bool(key.as_ref()) {
            Some(value.into())
        } else if let Ok(value) = cfg.get_string(key.as_ref()) {
            Some(value.into())
        } else {
            None
        }
    }

    pub(crate) fn set_config(&self, key: impl AsRef<str>, value: ConfigValue) -> Result<bool> {
        let mut cfg = self.inner.config()?;
        let contains = self.get_config(&key).is_some();
        match value {
            ConfigValue::Bool(b) => cfg.set_bool(key.as_ref(), b)?,
            ConfigValue::String(s) => cfg.set_str(key.as_ref(), &s)?,
        }
        Ok(!contains)
    }
}
