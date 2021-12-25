use std::{env, path::PathBuf};

use anyhow::{anyhow, Result};
use git2::Repository;

pub(crate) fn guess_repository() -> Result<PathBuf> {
    Ok(Repository::discover(env::current_dir()?)?
        .workdir()
        .ok_or_else(|| anyhow!("Cannot be used in a bare repository"))?
        .to_path_buf())
}
