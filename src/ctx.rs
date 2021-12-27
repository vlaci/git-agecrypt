use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::git;

pub(crate) trait Context {
    type Repo: git::Repository;

    fn repo(&self) -> &Self::Repo;

    fn get_sidecar(&self, path: &Path, extension: &str) -> Result<PathBuf>;

    fn sidecar_directory(&self) -> PathBuf;

    fn add_config(&self, key: &str, value: &str) -> Result<bool>;

    fn remove_config(&self, key: &str, value: &str) -> Result<bool>;

    fn list_config(&self, key: &str) -> Result<Vec<String>>;

    fn configure_filter(&self) -> Result<()>;

    fn deconfigure_filter(&self) -> Result<()>;

    fn remove_sidecar_files(&self) -> Result<()>;
}

struct ContextWrapper<R: git::Repository> {
    repo: R,
}

impl<R: git::Repository> ContextWrapper<R> {
    pub(crate) fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R: git::Repository> Context for ContextWrapper<R> {
    type Repo = R;
    fn repo(&self) -> &R {
        &self.repo
    }

    fn get_sidecar(&self, path: &Path, extension: &str) -> Result<PathBuf> {
        let relpath = path.strip_prefix(self.repo.workdir())?;
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

    fn add_config(&self, key: &str, value: &str) -> Result<bool> {
        let entry_name = format!("{}.{}", CONFIG_PATH, key);
        self.repo.add_config(&entry_name, value)
    }

    fn remove_config(&self, key: &str, value: &str) -> Result<bool> {
        let entry_name = format!("{}.{}", CONFIG_PATH, key);

        self.repo.remove_config(&entry_name, value)
    }

    fn list_config(&self, key: &str) -> Result<Vec<String>> {
        let _entry_name = format!("{}.{}", CONFIG_PATH, key);
        self.repo.list_config(key)
    }

    fn configure_filter(&self) -> Result<()> {
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

    fn deconfigure_filter(&self) -> Result<()> {
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

    fn remove_sidecar_files(&self) -> Result<()> {
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

fn exist_ok<T>(result: git::Result<T>, default: T) -> Result<T> {
    match result {
        Ok(ok) => Ok(ok),
        Err(err) => match err {
            git::Error::AlreadyExists(_) => Ok(default),
            err => Err(anyhow::anyhow!(err)),
        },
    }
}

pub(crate) fn new(repo: git::LibGit2Repository) -> impl Context<Repo = git::LibGit2Repository> {
    ContextWrapper::new(repo)
}
