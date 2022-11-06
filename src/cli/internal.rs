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
        let mut existing_hash = [0u8; 32];
        if let Some(hash_buffer) = self.ctx.load_sidecar(&file, "hash")? {
            existing_hash = hash_buffer.as_slice().try_into()?
        } else {
            log::debug!("No saved hash file found");
        }

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

        let saved = if hash == old_hash {
            self.ctx.load_sidecar(&file, "age")?
        } else {
            None
        };

        let result = if let Some(content) = saved {
            log::debug!("File didn't change since last encryption, loading from git HEAD");
            content
        } else {
            log::debug!("File changed since last encryption, re-encrypting");

            let cfg = self.ctx.config()?;
            let public_keys = cfg.get_public_keys(&file)?;

            let res = age::encrypt(public_keys, &mut &contents[..])?;
            self.ctx.store_sidecar(&file, "hash", hash.as_bytes())?;
            self.ctx.store_sidecar(&file, "age", &res)?;
            res
        };
        Ok(io::stdout().write_all(&result)?)
    }

    pub(crate) fn smudge(&self, file: impl AsRef<Path>) -> Result<()> {
        log::info!("Decrypting file");
        let file = self.ctx.repo().workdir().join(file);

        log::debug!("Loading identities from config");
        let all_identities = self.ctx.repo().list_config("identity")?;
        log::debug!(
            "Loaded identities from config; identities='{:?}'",
            all_identities
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

    pub(crate) fn textconv(&self, path: impl AsRef<Path>) -> Result<()> {
        log::info!("Decrypting file to show in diff");

        let all_identities: Vec<String> = self
            .ctx
            .age_identities()
            .list()?
            .into_iter()
            .map(|i| i.path)
            .collect();

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
