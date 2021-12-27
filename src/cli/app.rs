use std::{
    fmt::{Display, Formatter},
    io::stdout,
};

use anyhow::Result;

use crate::{
    commands::{internal, public},
    git::Repository,
};

use super::args::{Args, Commands};

pub(crate) fn run(args: Args, repo: Repository) -> Result<()> {
    match args.command {
        Commands::Init => {
            public::init(repo)?;
            print!("Success!");
            Ok(())
        }
        Commands::Deinit => {
            public::deinit(repo)?;
            println!("Success!");
            Ok(())
        }
        Commands::Status => {
            let status = public::status(repo)?;
            print!("{}", status);
            Ok(())
        }
        Commands::Config { cfg } => {
            print!("{}", public::config(repo, cfg.into())?);
            Ok(())
        }
        Commands::Clean { secrets_nix, file } => internal::clean(repo, &secrets_nix, &file),
        Commands::Smudge { identities, file } => internal::smudge(repo, &identities, &file),
        Commands::Textconv { identities, path } => internal::textconv(repo, &identities, &path),
    }?;
    Ok(())
}

impl Display for public::ConfigResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            public::ConfigResult::Succeeded => writeln!(f, "Success!"),
            public::ConfigResult::NothingDone => Ok(()),
            public::ConfigResult::Identities(identities) => {
                write!(f, "{}", identities)
            }
        }
    }
}

impl Display for public::Identities {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let padding = self.0.iter().map(|i| i.path.len()).max().unwrap_or(0);
        writeln!(f, "The following identities are currently configured:")?;
        for i in &self.0 {
            if let Err(err) = i.is_valid() {
                writeln!(f,
                    "    ⨯ {:padding$} -- {}",
                    i.path,
                    err.to_string(),
                    padding = padding
                )?;
            } else {
                writeln!(f, "    ✓ {}", i.path)?;
            }
        }
        Ok(())
    }
}

impl Display for public::StatusResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identities)
    }
}
