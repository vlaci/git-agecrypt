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
    Identities(Vec<String>),
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
        ConfigCommand::ListIdentities => list_identities(repo),
    }
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

fn list_identities(repo: git::Repository) -> Result<ConfigResult> {
    let identities = repo.list_config("identity")?;
    let mut rv = Vec::with_capacity(identities.len());

    let padding = identities.iter().map(|i| i.len()).max().unwrap_or(0);
    for i in identities {
        let is_valid = age::validate_identity(&i);
        if let Err(err) = is_valid {
            rv.push(format!(
                "⨯ {:padding$} -- {}",
                i,
                err.to_string(),
                padding = padding
            ));
        } else {
            rv.push(format!("✓ {}", &i));
        }
    }

    Ok(ConfigResult::Identities(rv))
}
