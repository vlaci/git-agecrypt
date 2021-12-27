use std::{
    fmt::{Display, Formatter},
};

use anyhow::Result;

use crate::{
    commands::{internal, public}, ctx::Context,
};

use super::args::{Args, Commands};

pub(crate) fn run(args: Args, ctx: Context) -> Result<()> {
    match args.command {
        Commands::Init => {
            public::init(ctx)?;
            print!("Success!");
            Ok(())
        }
        Commands::Deinit => {
            public::deinit(ctx)?;
            println!("Success!");
            Ok(())
        }
        Commands::Status => {
            let status = public::status(ctx)?;
            print!("{}", status);
            Ok(())
        }
        Commands::Config { cfg } => {
            print!("{}", public::config(ctx, cfg.into())?);
            Ok(())
        }
        Commands::Clean { secrets_nix, file } => internal::clean(ctx, &secrets_nix, &file),
        Commands::Smudge { identities, file } => internal::smudge(ctx, &identities, &file),
        Commands::Textconv { identities, path } => internal::textconv(ctx, &identities, &path),
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
