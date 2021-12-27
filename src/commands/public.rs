use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::{age, git};

pub(crate) fn init(repo: git::Repository) -> Result<()> {
    repo.configure_filter()?;
    Ok(())
}

pub(crate) fn deinit(repo: git::Repository) -> Result<()> {
    repo.deconfigure_filter()?;
    repo.remove_sidecar_files()?;
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

pub(crate) fn config(repo: git::Repository, cfg: ConfigCommand) -> Result<ConfigResult> {
    match cfg {
        ConfigCommand::AddIdentity(identity) => add_identity(repo, identity),
        ConfigCommand::RemoveIdentity(identity) => remove_identity(repo, identity),
        ConfigCommand::ListIdentities => Ok(ConfigResult::Identities(list_identities(repo)?)),
    }
}

pub(crate) struct StatusResult {
    pub identities: Identities,
}

pub(crate) fn status(repo: git::Repository) -> Result<StatusResult> {
    let identities = list_identities(repo)?;

    Ok(StatusResult { identities })
}

fn add_identity(repo: git::Repository, identity: PathBuf) -> Result<ConfigResult> {
    let fpath = repo.workdir().join(&identity);
    if let Err(err) = age::validate_identity(&fpath) {
        bail!("Not adding identity; details: {}", err);
    }
    Ok(repo
        .add_config("identity", identity.to_string_lossy())?
        .into())
}

fn remove_identity(repo: git::Repository, identity: PathBuf) -> Result<ConfigResult> {
    Ok(repo
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

fn list_identities(repo: git::Repository) -> Result<Identities> {
    let identities = repo.list_config("identity")?;
    Ok(Identities(
        identities
            .into_iter()
            .map(move |path| Identity { path })
            .collect(),
    ))
}
