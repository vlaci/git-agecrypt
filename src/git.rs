use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
};

use anyhow::{bail, Result};

pub(crate) struct Repository {
    inner: git2::Repository,
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

    pub(crate) fn get_sidecar(&self, path: impl AsRef<Path>, extension: &str) -> Result<PathBuf> {
        let relpath = path.as_ref().strip_prefix(self.workdir())?;
        let name = relpath.to_string_lossy().replace('/', "!");

        let dir = self.sidecar_directory();
        fs::create_dir_all(&dir)?;

        let mut rv = dir.join(name);
        rv.set_extension(extension);
        Ok(rv)
    }

    fn sidecar_directory(&self) -> PathBuf {
        self.inner.path().join("git-agenix")
    }

    pub(crate) fn get_file_contents(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let relpath = path.as_ref().strip_prefix(self.workdir())?;
        let entry = self.inner.head()?.peel_to_tree()?.get_path(relpath)?;
        let contents = entry.to_object(&self.inner)?;

        Ok(contents.as_blob().unwrap().content().into())
    }

    pub(crate) fn configure_filter(&self) -> Result<()> {
        let exe = std::env::current_exe()?;
        let exe = exe.to_string_lossy();
        let mut cfg = self.inner.config()?;
        cfg.set_bool("filter.git-agenix.required", true)?;
        cfg.set_str(
            "filter.git-agenix.smudge",
            format!("{} smudge -f %f", exe).as_str(),
        )?;
        cfg.set_str(
            "filter.git-agenix.clean",
            format!("{} clean -f %f", exe).as_str(),
        )?;
        cfg.set_str(
            "diff.git-agenix.textconv",
            format!("{} textconv", exe).as_str(),
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
