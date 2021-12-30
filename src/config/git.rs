use crate::{ctx::Context, git::Repository};

use super::{Container, Result};

const CONFIG_PATH: &str = "git-agenix.config";

pub struct GitConfigEntry {
    value: String,
}

impl GitConfigEntry {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl From<String> for GitConfigEntry {
    fn from(value: String) -> Self {
        Self { value }
    }
}

impl From<GitConfigEntry> for String {
    fn from(val: GitConfigEntry) -> Self {
        val.value
    }
}

pub(crate) struct GitConfig<'a, C: Context> {
    ctx: &'a C,
    ns: String,
}

impl<'a, C: Context> GitConfig<'a, C> {
    pub fn new(ctx: &'a C, ns: String) -> Self {
        Self { ctx, ns }
    }
}

impl From<crate::git::Error> for super::Error {
    fn from(err: crate::git::Error) -> Self {
        match err {
            crate::git::Error::AlreadyExists(v) => Self::AlreadyExists(v),
            crate::git::Error::NotExist(v) => Self::NotExist(v),
            crate::git::Error::Other(e) => Self::Other(e),
        }
    }
}

impl<C: Context> Container for GitConfig<'_, C> {
    type Item = GitConfigEntry;

    fn add(&mut self, item: Self::Item) -> Result<()> {
        let entry_name = format!("{}.{}", CONFIG_PATH, self.ns);
        self.ctx.repo().add_config(&entry_name, &item.value)?;
        Ok(())
    }

    fn remove(&mut self, item: Self::Item) -> Result<()> {
        let entry_name = format!("{}.{}", CONFIG_PATH, self.ns);

        self.ctx.repo().remove_config(&entry_name, &item.value)?;
        Ok(())
    }

    fn list(&self) -> Result<Vec<Self::Item>> {
        Ok(self
            .ctx
            .repo()
            .list_config(&self.ns)?
            .into_iter()
            .map(GitConfigEntry::new)
            .collect())
    }
}
