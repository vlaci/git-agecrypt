use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::{
    config::{AgeIdentities, AgeIdentity, AppConfig, Container, GitConfig},
    git,
};

pub(crate) trait Context {
    type Repo: git::Repository;

    fn repo(&self) -> &Self::Repo;

    fn get_sidecar(&self, path: &Path, extension: &str) -> Result<PathBuf>;

    fn sidecar_directory(&self) -> PathBuf;

    fn configure_filter(&self) -> Result<()>;

    fn deconfigure_filter(&self) -> Result<()>;

    fn remove_sidecar_files(&self) -> Result<()>;

    fn age_identities(&self) -> Box<dyn Container<Item = AgeIdentity> + '_>;

    fn config(&self) -> Result<AppConfig>;
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
        self.repo.path().join("git-agecrypt")
    }

    fn configure_filter(&self) -> Result<()> {
        let exe = std::env::current_exe()?;
        let exe = exe.to_string_lossy();

        ensure_state(self.repo.set_config("filter.git-agecrypt.required", "true"))?;
        ensure_state(self.repo.set_config(
            "filter.git-agecrypt.smudge",
            &format!("{} smudge -f %f", exe),
        ))?;
        ensure_state(
            self.repo
                .set_config("filter.git-agecrypt.clean", &format!("{} clean -f %f", exe)),
        )?;
        ensure_state(
            self.repo
                .set_config("diff.git-agecrypt.textconv", &format!("{} textconv", exe)),
        )?;
        Ok(())
    }

    fn deconfigure_filter(&self) -> Result<()> {
        ensure_state(self.repo.remove_config_section("fiter.git-agecrypt"))?;
        ensure_state(self.repo.remove_config_section("diff.git-agecrypt"))?;

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

    fn age_identities(&self) -> Box<dyn Container<Item = AgeIdentity> + '_> {
        let cfg = GitConfig::new(self, "identity".into());
        Box::new(AgeIdentities::new(cfg))
    }

    fn config(&self) -> Result<AppConfig> {
        Ok(AppConfig::load(&PathBuf::from("git-agecrypt.toml"))?)
    }
}

fn ensure_state(result: git::Result<()>) -> Result<()> {
    match result {
        Ok(()) => Ok(()),
        Err(err) => match err {
            git::Error::AlreadyExists(_) => Ok(()),
            git::Error::NotExist(_) => Ok(()),
            err => Err(anyhow::anyhow!(err)),
        },
    }
}

pub(crate) fn new(repo: git::LibGit2Repository) -> impl Context<Repo = git::LibGit2Repository> {
    ContextWrapper::new(repo)
}
