use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::{age, ctx::Context, git::Repository};

pub(crate) struct CommandContext<C: Context> {
    pub ctx: C,
}

pub(crate) enum ConfigCommand {
    AddIdentity(PathBuf),
    RemoveIdentity(PathBuf),
    ListIdentities,
}

pub(crate) enum Outcome<T> {
    NoChanges,
    Changes(T),
    Output(T),
}

pub struct NoOutput;

use Outcome::*;

pub(crate) enum ConfigResult {
    IdentityAdded,
    IdentityRemoved,
    Identities(Identities),
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

impl<C: Context> CommandContext<C> {
    pub(crate) fn init(&self) -> Result<Outcome<NoOutput>> {
        let changed = self.ctx.configure_filter()?;
        if changed {
            Ok(Changes(NoOutput{}))
        } else {
            Ok(NoChanges)
        }
    }

    pub(crate) fn deinit(&self) -> Result<Outcome<NoOutput>> {
        let changed =
        self.ctx.deconfigure_filter()? ||
        self.ctx.remove_sidecar_files()?;
        if changed {
            Ok(Changes(NoOutput{}))
        } else {
            Ok(NoChanges)
        }
    }

    pub(crate) fn config(&self, cfg: ConfigCommand) -> Result<Outcome<ConfigResult>> {
        let rv = match cfg {
            ConfigCommand::AddIdentity(identity) => self.add_identity(identity)?,
            ConfigCommand::RemoveIdentity(identity) => self.remove_identity(identity)?,
            ConfigCommand::ListIdentities => Output(ConfigResult::Identities(self.list_identities()?)),
        };
        Ok(rv)
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

    fn add_identity(&self, identity: PathBuf) -> Result<Outcome<ConfigResult>> {
        let fpath = self.ctx.repo().workdir().join(&identity);
        if let Err(err) = age::validate_identity(&fpath) {
            bail!("Not adding identity; details: {}", err);
        }
        let changed = self
            .ctx
            .add_config("identity", &identity.to_string_lossy())?;
        if changed {
            Ok(Changes(ConfigResult::IdentityAdded))
        } else {
            Ok(NoChanges)
        }
    }

    fn remove_identity(&self, identity: PathBuf) -> Result<Outcome<ConfigResult>> {
        let changed = self
            .ctx
            .remove_config("identity", &identity.to_string_lossy())?;
        if changed {
            Ok(Changes(ConfigResult::IdentityRemoved))
        } else {
            Ok(NoChanges)
        }
    }
}
