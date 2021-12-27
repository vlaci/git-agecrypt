use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::{age, ctx::Context};

pub(crate) fn init(ctx: Context) -> Result<()> {
    ctx.configure_filter()?;
    Ok(())
}

pub(crate) fn deinit(ctx: Context) -> Result<()> {
    ctx.deconfigure_filter()?;
    ctx.remove_sidecar_files()?;
    Ok(())
}

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

pub(crate) fn config(ctx: Context, cfg: ConfigCommand) -> Result<ConfigResult> {
    match cfg {
        ConfigCommand::AddIdentity(identity) => add_identity(ctx, identity),
        ConfigCommand::RemoveIdentity(identity) => remove_identity(ctx, identity),
        ConfigCommand::ListIdentities => Ok(ConfigResult::Identities(list_identities(ctx)?)),
    }
}

pub(crate) struct StatusResult {
    pub identities: Identities,
}

pub(crate) fn status(ctx: Context) -> Result<StatusResult> {
    let identities = list_identities(ctx)?;

    Ok(StatusResult { identities })
}

fn add_identity(ctx: Context, identity: PathBuf) -> Result<ConfigResult> {
    let fpath = ctx.repo().workdir().join(&identity);
    if let Err(err) = age::validate_identity(&fpath) {
        bail!("Not adding identity; details: {}", err);
    }
    Ok(ctx
        .add_config("identity", identity.to_string_lossy())?
        .into())
}

fn remove_identity(ctx: Context, identity: PathBuf) -> Result<ConfigResult> {
    Ok(ctx
        .remove_config("identity", identity.to_string_lossy())?
        .into())
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

fn list_identities(ctx: Context) -> Result<Identities> {
    let identities = ctx.list_config("identity")?;
    Ok(Identities(
        identities
            .into_iter()
            .map(move |path| Identity { path })
            .collect(),
    ))
}
