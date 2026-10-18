use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

use crate::{
    config::{AgeIdentities, AgeIdentity, AppConfig, Container, GitConfig},
    git,
};

pub(crate) trait Context {
    type Repo: git::Repository;

    fn repo(&self) -> &Self::Repo;

    fn store_sidecar(&self, for_path: &Path, extension: &str, content: &[u8]) -> Result<()>;

    fn load_sidecar(&self, for_path: &Path, extension: &str) -> Result<Option<Vec<u8>>>;

    fn current_exe(&self) -> Result<String>;

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
    fn sidecar_directory(&self) -> PathBuf {
        self.repo.path().join("git-agecrypt")
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
}

impl<R: git::Repository> Context for ContextWrapper<R> {
    type Repo = R;
    fn repo(&self) -> &R {
        &self.repo
    }

    fn store_sidecar(&self, for_path: &Path, extension: &str, content: &[u8]) -> Result<()> {
        let sidecar_path = self.get_sidecar(for_path, extension)?;
        File::create(sidecar_path)?.write_all(content)?;
        Ok(())
    }

    fn load_sidecar(&self, for_path: &Path, extension: &str) -> Result<Option<Vec<u8>>> {
        let sidecar_path = self.get_sidecar(for_path, extension)?;
        match File::open(sidecar_path) {
            Ok(mut f) => {
                let mut buff = Vec::new();
                f.read_to_end(&mut buff)?;
                Ok(Some(buff))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => {
                bail!(e)
            }
        }
    }

    fn current_exe(&self) -> Result<String> {
        let exe = std::env::current_exe()?;
        let exe = exe.to_string_lossy();
        Ok(exe.into())
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
        Ok(AppConfig::load(
            &PathBuf::from("git-agecrypt.toml"),
            self.repo.workdir(),
        )?)
    }
}

pub(crate) fn new(repo: git::LibGit2Repository) -> impl Context<Repo = git::LibGit2Repository> {
    ContextWrapper::new(repo)
}
