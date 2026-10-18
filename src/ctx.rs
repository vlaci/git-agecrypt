// SPDX-FileCopyrightText: 2021 László Vaskó <opensource@vlaci.email>
//
// SPDX-License-Identifier: MPL-2.0

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

use crate::{
    cache::{self, CacheEntry},
    config::{AgeIdentities, AgeIdentity, AppConfig, Container, GitConfig},
    git,
};

pub(crate) trait Context {
    type Repo: git::Repository;

    fn repo(&self) -> &Self::Repo;

    fn store_cache(&self, for_path: &Path, entry: &CacheEntry) -> Result<()>;

    fn load_cache(&self, for_path: &Path) -> Result<Option<CacheEntry>>;

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

    fn cache_directory(&self) -> PathBuf {
        self.sidecar_directory().join("v2")
    }
}

impl<R: git::Repository> Context for ContextWrapper<R> {
    type Repo = R;

    fn repo(&self) -> &R {
        &self.repo
    }

    fn store_cache(&self, for_path: &Path, entry: &CacheEntry) -> Result<()> {
        let dir = self.cache_directory();
        let relpath = for_path.strip_prefix(self.repo.workdir())?;
        let name = cache::file_name(relpath);

        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        // Restrict permission on creation
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(&dir)?;

        let tmp = dir.join(format!("{}.tmp{}", name, std::process::id()));
        let final_path = dir.join(&name);

        let write_result = (|| -> std::io::Result<()> {
            let mut opts = OpenOptions::new();
            opts.write(true).create(true).truncate(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                opts.mode(0o600);
            }
            let mut f = opts.open(&tmp)?;
            f.write_all(&entry.encode())?;
            fs::rename(&tmp, &final_path)?;
            Ok(())
        })();

        if let Err(e) = write_result {
            let _ = fs::remove_file(&tmp);
            return Err(e.into());
        }
        Ok(())
    }

    fn load_cache(&self, for_path: &Path) -> Result<Option<CacheEntry>> {
        let dir = self.cache_directory();
        let relpath = for_path.strip_prefix(self.repo.workdir())?;
        let name = cache::file_name(relpath);
        let path = dir.join(name);

        match File::open(&path) {
            Ok(mut f) => {
                let mut buff = Vec::new();
                f.read_to_end(&mut buff)?;
                match CacheEntry::decode(&buff) {
                    Some(entry) => Ok(Some(entry)),
                    None => {
                        log::warn!("Ignoring unreadable cache entry {}", path.display());
                        Ok(None)
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => bail!(e),
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
