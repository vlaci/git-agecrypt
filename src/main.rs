use std::{
    env,
    fs::{self, File},
    io::{self, Read, Seek, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Result};

mod age;
mod cli;
mod nix;

fn main() -> Result<()> {
    let cli = cli::parse_args();

    let result = match cli.command {
        cli::Commands::Clean { secrets_nix, file } => clean(&secrets_nix, &file),
        cli::Commands::Smudge { identities } => smudge(&identities),
        cli::Commands::Textconv { identities, path } => textconv(&identities, &path),
    }?;
    Ok(())
}

fn clean(secrets_nix: impl AsRef<Path>, file: impl AsRef<Path>) -> Result<Vec<u8>> {
    let rule = load_rule_for(secrets_nix, file)?;
    age::encrypt(&rule.public_keys, &mut io::stdin())
}

fn smudge(identities: &[impl AsRef<Path>]) -> Result<Vec<u8>> {
    if let Some(rv) = age::decrypt(identities, &mut io::stdin())? {
        Ok(rv)
    } else {
        bail!("Input isn't encrypted")
    }
}

fn textconv(identities: &[impl AsRef<Path>], path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let mut f = File::open(path)?;
    if let Some(rv) = age::decrypt(identities, &mut f)? {
        Ok(rv)
    } else {
        f.rewind()?;
        let mut buff = vec![];
        f.read_to_end(&mut buff)?;
        buff
    };
    Ok(io::stdout().write_all(&result)?)
}

#[derive(Debug)]
struct AgenixRule {
    pub path: PathBuf,
    pub public_keys: Vec<String>,
}

fn load_rule_for(rules_path: impl AsRef<Path>, for_file: impl AsRef<Path>) -> Result<AgenixRule> {
    let val = nix::eval_file(&rules_path)?;

    let dir = fs::canonicalize(rules_path.as_ref().parent().unwrap())?;
    let for_file = env::current_dir()?.join(for_file);

    for (pth, v) in val
        .as_object()
        .ok_or(anyhow!("Expected to contain objects"))?
        .iter()
    {
        let abs_path = dir.join(pth);
        if abs_path != for_file {
            continue;
        }
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

        return Ok(AgenixRule {
            path: abs_path,
            public_keys,
        });
    }

    bail!(
        "No rule in {} for {}",
        rules_path.as_ref().to_string_lossy(),
        for_file.to_string_lossy()
    );
}
