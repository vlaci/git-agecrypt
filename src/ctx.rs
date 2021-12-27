use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::git;

pub(crate) struct Context<'a> {
    repo: &'a dyn git::Repository,
}

const CONFIG_PATH: &str = "git-agenix.config";

impl<'a> Context<'a> {
    pub(crate) fn new(repo: &'a dyn git::Repository) -> Self {
        Self { repo }
    }

    pub(crate) fn repo(&self) -> &dyn git::Repository {
        self.repo
    }

    pub(crate) fn get_sidecar(&self, path: impl AsRef<Path>, extension: &str) -> Result<PathBuf> {
        let relpath = path.as_ref().strip_prefix(self.repo.workdir())?;
        let name = relpath.to_string_lossy().replace('/', "!");

        let dir = self.sidecar_directory();
        fs::create_dir_all(&dir)?;

        let mut rv = dir.join(name);
        rv.set_extension(extension);
        Ok(rv)
    }

    fn sidecar_directory(&self) -> PathBuf {
        self.repo.path().join("git-agenix")
    }

    pub(crate) fn add_config(&self, key: impl AsRef<str>, value: impl AsRef<str>) -> Result<bool> {
        let entry_name = format!("{}.{}", CONFIG_PATH, key.as_ref());
        self.repo.add_config(&entry_name, value.as_ref())
    }

    pub(crate) fn remove_config(
        &self,
        key: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<bool> {
        let entry_name = format!("{}.{}", CONFIG_PATH, key.as_ref());

        self.repo.remove_config(&entry_name, value.as_ref())
    }

    pub(crate) fn list_config(&self, key: impl AsRef<str>) -> Result<Vec<String>> {
        let _entry_name = format!("{}.{}", CONFIG_PATH, key.as_ref());
        self.repo.list_config(key.as_ref())
    }

    pub(crate) fn configure_filter(&self) -> Result<()> {
        let exe = std::env::current_exe()?;
        let exe = exe.to_string_lossy();

        self.repo
            .set_config("filter.git-agenix.required", true.into())?;
        self.repo.set_config(
            "filter.git-agenix.smudge",
            format!("{} smudge -f %f", exe).into(),
        )?;
        self.repo.set_config(
            "filter.git-agenix.clean",
            format!("{} clean -f %f", exe).into(),
        )?;
        self.repo.set_config(
            "diff.git-agenix.textconv",
            format!("{} textconv", exe).into(),
        )?;
        Ok(())
    }

    pub(crate) fn deconfigure_filter(&self) -> Result<()> {
        // Unfortunately there is no `git config --remove-section <section>` equivalent in libgit2
        let mut command = process::Command::new("git");
        command
            .arg("config")
            .arg("--remove-section")
            .arg("filter.git-agenix");
        let output = command.output()?;

        if !output.status.success() {
            log::error!(
                "Failed to execute command. This may not be an issue; command='{:?}' status='{}', stdout={:?}, stderr={:?}",
                command,
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let mut command = process::Command::new("git");
        command
            .arg("config")
            .arg("--remove-section")
            .arg("diff.git-agenix");
        let output = command.output()?;

        if !output.status.success() {
            log::error!(
                "Failed to execute command. This may not be an issue; command='{:?}' status='{}', stdout={:?}, stderr={:?}",
                command,
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    pub(crate) fn remove_sidecar_files(&self) -> Result<()> {
        let dir = self.sidecar_directory();
        fs::remove_dir_all(dir).or_else(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                Ok(())
            } else {
                Err(err)
            }
        })?;
        Ok(())
    }
}
