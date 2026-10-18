use std::{
    fs::File,
    io::{self, Read, Seek, Write},
    path::Path,
};

use anyhow::{bail, Result};

use crate::{
    age,
    cache::{self, CacheEntry},
    ctx::Context,
    git::Error as GitError,
    git::Repository,
};

pub(crate) struct CommandContext<C: Context> {
    pub ctx: C,
}

impl<C: Context> CommandContext<C> {
    pub(crate) fn clean(&self, file: impl AsRef<Path>) -> Result<()> {
        log::info!("Encrypting file");
        let file = self.ctx.repo().workdir().join(file);

        let mut hasher = blake3::Hasher::new();
        let mut contents = vec![];
        io::stdin().read_to_end(&mut contents)?;
        let hash = hasher.update(&contents).finalize();
        let h = *hash.as_bytes();

        let cfg = self.ctx.config()?;
        let recipients = cfg.get_public_keys(&file)?;
        let fp = cache::recipients_fingerprint(recipients);

        let entry = self.ctx.load_cache(&file)?;

        if let Some(ref e) = entry {
            if e.plaintext_hash == h
                && (e.recipients == fp || e.recipients == cache::UNKNOWN_RECIPIENTS)
            {
                // Upgrade UNKNOWN_RECIPIENTS to the actual fingerprint
                if e.recipients == cache::UNKNOWN_RECIPIENTS {
                    self.ctx.store_cache(
                        &file,
                        &CacheEntry {
                            plaintext_hash: h,
                            recipients: fp,
                            ciphertext: e.ciphertext.clone(),
                        },
                    )?;
                }
                return Ok(io::stdout().write_all(&e.ciphertext)?);
            }
        }

        let head_reuse_allowed = entry
            .as_ref()
            .is_none_or(|e| e.recipients == fp || e.recipients == cache::UNKNOWN_RECIPIENTS);

        if head_reuse_allowed {
            if let Some(repo_contents) = self.try_head_reuse(&file, &contents)? {
                let ciphertext = repo_contents;
                self.ctx.store_cache(
                    &file,
                    &CacheEntry {
                        plaintext_hash: h,
                        recipients: fp,
                        ciphertext: ciphertext.clone(),
                    },
                )?;
                return Ok(io::stdout().write_all(&ciphertext)?);
            }
        }

        log::debug!("File changed since last encryption, re-encrypting");
        let res = age::encrypt(recipients, &mut &contents[..])?;
        self.ctx.store_cache(
            &file,
            &CacheEntry {
                plaintext_hash: h,
                recipients: fp,
                ciphertext: res.clone(),
            },
        )?;
        Ok(io::stdout().write_all(&res)?)
    }

    fn try_head_reuse(&self, file: &Path, contents: &[u8]) -> Result<Option<Vec<u8>>> {
        log::debug!("Encrypted content changed, checking decrypted version");
        let repo_contents = match self.ctx.repo().get_file_contents(file) {
            Ok(v) => v,
            Err(GitError::NotExist(s)) => {
                log::debug!("{s}");
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };

        let identities = self.get_identities()?;
        let mut cur = io::Cursor::new(repo_contents);
        let decrypted = age::decrypt(&identities, &mut cur)?.unwrap_or_default();
        if decrypted == contents {
            log::debug!("Decrypted content matches, using from working copy");
            Ok(Some(cur.into_inner()))
        } else {
            Ok(None)
        }
    }

    fn get_identities(&self) -> Result<Vec<String>> {
        log::debug!("Loading identities from config");
        let all_identities: Vec<String> = self
            .ctx
            .age_identities()
            .list()?
            .into_iter()
            .map(|i| i.path)
            .collect();
        log::debug!(
            "Loaded identities from config; identities='{:?}'",
            all_identities
        );
        Ok(all_identities)
    }

    pub(crate) fn smudge(&self, file: impl AsRef<Path>) -> Result<()> {
        log::info!("Decrypting file");
        let file = self.ctx.repo().workdir().join(file);

        let mut encrypted = vec![];
        io::stdin().read_to_end(&mut encrypted)?;
        let mut cur = io::Cursor::new(encrypted);
        let all_identities = self.get_identities()?;
        if let Some(rv) = age::decrypt(&all_identities, &mut cur)? {
            log::info!("Decrypted file");
            let mut hasher = blake3::Hasher::new();
            let hash = hasher.update(&rv).finalize();

            let fp = self
                .ctx
                .config()
                .ok()
                .and_then(|c| c.get_public_keys(&file).ok().map(|keys| keys.to_vec()))
                .map(|keys| cache::recipients_fingerprint(&keys))
                .unwrap_or(cache::UNKNOWN_RECIPIENTS);

            log::debug!("Storing hash for file; hash={:?}", hash.to_hex().as_str());
            self.ctx.store_cache(
                &file,
                &CacheEntry {
                    plaintext_hash: *hash.as_bytes(),
                    recipients: fp,
                    ciphertext: cur.into_inner(),
                },
            )?;

            Ok(io::stdout().write_all(&rv)?)
        } else {
            bail!("Input isn't encrypted")
        }
    }

    pub(crate) fn textconv(&self, path: impl AsRef<Path>) -> Result<()> {
        log::info!("Decrypting file to show in diff");

        let all_identities = self.get_identities()?;

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
