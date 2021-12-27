use std::{
    fmt::Debug,
    fs::{self, File},
    io::{self, Read, Seek, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Result};
use blake3::Hash;

use crate::{age, ctx::Context, git::Repository, nix};

pub(crate) struct CommandContext<C: Context> {
    pub ctx: C,
}

impl<C: Context> CommandContext<C> {
    pub(crate) fn clean(
        &self,
        secrets_nix: impl AsRef<Path>,
        file: impl AsRef<Path>,
    ) -> Result<()> {
        log::info!("Encrypting file");
        let file = self.ctx.repo().workdir().join(file);
        let sidecar = self.ctx.get_sidecar(&file, "hash")?;

        log::debug!(
            "Looking for saved has information. target={:?}, sidecar={:?}",
            file,
            sidecar
        );
        let mut existing_hash = [0u8; 32];

        match File::open(&sidecar) {
            Ok(mut f) => {
                f.read_exact(&mut existing_hash)?;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                log::debug!("No saved hash file found")
            }
            Err(e) => {
                bail!(e);
            }
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
        let result = if hash == old_hash {
            log::debug!("File didn't change since last encryption, loading from git HEAD");
            self.ctx.repo().get_file_contents(&file)
        } else {
            log::debug!("File changed since last encryption, re-encrypting");
            File::create(sidecar)?.write_all(hash.as_bytes())?;
            let rule = load_rule_for(&secrets_nix, file)?;
            age::encrypt(&rule.public_keys, &mut &contents[..])
        }?;
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
            let sidecar = self.ctx.get_sidecar(&file, "hash")?;
            let mut hasher = blake3::Hasher::new();
            let hash = hasher.update(&rv).finalize();

            log::debug!(
                "Storing hash for file; hash={:?} sidecar={:?}",
                hash.to_hex().as_str(),
                sidecar
            );
            File::create(sidecar)?.write_all(hash.as_bytes())?;

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

        let mut all_identities = self.ctx.list_config("identities")?;
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

#[derive(Debug)]
struct AgenixRule {
    pub path: PathBuf,
    pub public_keys: Vec<String>,
}

fn load_rule_for(rules_path: impl AsRef<Path>, for_file: impl AsRef<Path>) -> Result<AgenixRule> {
    let val = nix::eval_file(&rules_path)?;
    let dir = fs::canonicalize(rules_path.as_ref().parent().unwrap())?;

    for (pth, v) in val
        .as_object()
        .ok_or(anyhow!("Expected to contain objects"))?
        .iter()
    {
        let abs_path = dir.join(pth);
        if abs_path != for_file.as_ref() {
            log::debug!(
                "Encryption rule doesn't match; candidate={:?}, target={:?}",
                abs_path,
                for_file.as_ref()
            );
            continue;
        }
        log::debug!("Encryption rule matches; target={:?}", abs_path);
        let public_keys = v
            .as_object()
            .ok_or(anyhow!("Expected to contain objects"))?
            .get("publicKeys")
            .ok_or(anyhow!("publicKeys attribute missing"))?
            .as_array()
            .ok_or(anyhow!("publicKeys must be a list"))?
            .iter()
            .map(|k| {
                Ok(k.as_str()
                    .ok_or(anyhow!("publicKeys should be list of strings"))?
                    .to_string())
            })
            .collect::<Result<Vec<_>>>()?;

        log::debug!(
            "Collected public keys; target={:?}, public_keys={:?}",
            abs_path,
            public_keys
        );
        return Ok(AgenixRule {
            path: abs_path,
            public_keys,
        });
    }

    bail!(
        "No rule in {} for {}",
        rules_path.as_ref().to_string_lossy(),
        for_file.as_ref().to_string_lossy()
    );
}
