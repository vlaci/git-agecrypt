use std::{
    fs::File,
    io::{self, Read, Seek, Write},
    path::Path,
};

use anyhow::{bail, Result};
use blake3::Hash;

use crate::{age, ctx::Context, git::Repository};

pub(crate) struct CommandContext<C: Context> {
    pub ctx: C,
}

impl<C: Context> CommandContext<C> {
    pub(crate) fn clean(&self, file: impl AsRef<Path>) -> Result<()> {
        log::info!("Encrypting file");
        let file = self.ctx.repo().workdir().join(file);

        log::debug!("Looking for saved has information. target={:?}", file,);
        let existing_hash: [u8; 32] = self.ctx.load_sidecar(&file, "hash")?.unwrap_or_else(|| {
            log::debug!("No saved hash file found");
            Default::default()
        });

        let mut hasher = blake3::Hasher::new();
        let mut contents = vec![];
        io::stdin().read_to_end(&mut contents)?;
        let hash = hasher.update(&contents).finalize();

        let old_hash = Hash::from(existing_hash);
        log::debug!(
            "Comparing hashes for file; old_hash={}, new_hash={:?}",
            old_hash.to_hex().as_str(),
            hash.to_hex().as_str()
        );
        let result = if hash == old_hash {
            log::debug!("File didn't change since last encryption, loading from git HEAD");
            self.ctx.repo().get_file_contents(&file)?
        } else {
            log::debug!("File changed since last encryption, re-encrypting");

            let cfg = self.ctx.config()?;
            let public_keys = cfg.get_public_keys(&file)?;

            let res = age::encrypt(public_keys, &mut &contents[..])?;
            self.ctx.store_sidecar(&file, "hash", hash.as_bytes())?;
            res
        };
        Ok(io::stdout().write_all(&result)?)
    }

    pub(crate) fn smudge(
        &self,
        identities: &[impl AsRef<Path>],
        file: impl AsRef<Path>,
    ) -> Result<()> {
        log::info!("Decrypting file");
        let file = self.ctx.repo().workdir().join(file);

        log::debug!("Loading identities from config");
        let mut all_identities = self.ctx.repo().list_config("identity")?;
        log::debug!(
            "Loaded identities from config; identities='{:?}'",
            all_identities
        );
        all_identities.extend(
            identities
                .iter()
                .map(|i| i.as_ref().to_string_lossy().into()),
        );

        if let Some(rv) = age::decrypt(&all_identities, &mut io::stdin())? {
            log::info!("Decrypted file");
            let mut hasher = blake3::Hasher::new();
            let hash = hasher.update(&rv).finalize();

            log::debug!("Storing hash for file; hash={:?}", hash.to_hex().as_str(),);
            self.ctx.store_sidecar(&file, "hash", hash.as_bytes())?;

            Ok(io::stdout().write_all(&rv)?)
        } else {
            bail!("Input isn't encrypted")
        }
    }

    pub(crate) fn textconv(
        &self,
        identities: &[impl AsRef<Path>],
        path: impl AsRef<Path>,
    ) -> Result<()> {
        log::info!("Decrypting file to show in diff");

        let mut all_identities: Vec<String> = self
            .ctx
            .age_identities()
            .list()?
            .into_iter()
            .map(|i| i.path)
            .collect();

        all_identities.extend(
            identities
                .iter()
                .map(|i| i.as_ref().to_string_lossy().into()),
        );

        let mut f = File::open(path)?;
        let result = if let Some(rv) = age::decrypt(&all_identities, &mut f)? {
            log::info!("Decrypted file to show in diff");
            rv
        } else {
            log::info!("File isn't encrypted, probably a working copy; showing as is.");
            f.rewind()?;
            let mut buff = vec![];
            f.read_to_end(&mut buff)?;
            buff
        };
        Ok(io::stdout().write_all(&result)?)
    }
}
