use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::age;

use super::Commands;

pub(crate) enum ConfigCommand {
    AddIdentity(PathBuf),
    RemoveIdentity(PathBuf),
    ListIdentities,
}

pub(crate) enum ConfigResult {
    Succeeded,
    NothingDone,
    Identities(Identities),
}

impl From<bool> for ConfigResult {
    fn from(val: bool) -> Self {
        if val {
            Self::Succeeded
        } else {
            Self::NothingDone
        }
    }
}
pub(crate) struct Identities(pub(crate) Vec<Identity>);
pub(crate) struct Identity {
    pub path: String,
}

impl Identity {
    pub fn is_valid(&self) -> Result<()> {
        age::validate_identity(&self.path)
    }
}
pub(crate) struct StatusResult {
    pub identities: Identities,
}

impl<'a> Commands<'a> {
    pub(crate) fn init(&self) -> Result<()> {
        self.ctx.configure_filter()?;
        Ok(())
    }

    pub(crate) fn deinit(&self) -> Result<()> {
        self.ctx.deconfigure_filter()?;
        self.ctx.remove_sidecar_files()?;
        Ok(())
    }

    pub(crate) fn config(&self, cfg: ConfigCommand) -> Result<ConfigResult> {
        match cfg {
            ConfigCommand::AddIdentity(identity) => self.add_identity(identity),
            ConfigCommand::RemoveIdentity(identity) => self.remove_identity(identity),
            ConfigCommand::ListIdentities => Ok(ConfigResult::Identities(self.list_identities()?)),
        }
    }
    fn list_identities(&self) -> Result<Identities> {
        let identities = self.ctx.list_config("identity")?;
        Ok(Identities(
            identities
                .into_iter()
                .map(move |path| Identity { path })
                .collect(),
        ))
    }

    pub(crate) fn status(&self) -> Result<StatusResult> {
        let identities = self.list_identities()?;

        Ok(StatusResult { identities })
    }

    fn add_identity(&self, identity: PathBuf) -> Result<ConfigResult> {
        let fpath = self.ctx.repo().workdir().join(&identity);
        if let Err(err) = age::validate_identity(&fpath) {
            bail!("Not adding identity; details: {}", err);
        }
        Ok(self
            .ctx
            .add_config("identity", identity.to_string_lossy())?
            .into())
    }

    fn remove_identity(&self, identity: PathBuf) -> Result<ConfigResult> {
        Ok(self
            .ctx
            .remove_config("identity", identity.to_string_lossy())?
            .into())
    }
}
