use anyhow::Result;

use crate::{
    commands::{self, public},
    ctx::Context,
};

use super::args::{Args, Commands};

pub(crate) fn run(args: Args, ctx: impl Context) -> Result<()> {
    let cmd = commands::Commands { ctx };
    match args.command {
        Commands::Init => {
            cmd.init()?;
            print!("Success!");
            Ok(())
        }
        Commands::Deinit => {
            cmd.deinit()?;
            println!("Success!");
            Ok(())
        }
        Commands::Status => {
            let status = cmd.status()?;
            print!("{}", status);
            Ok(())
        }
        Commands::Config { cfg } => {
            print!("{}", cmd.config(cfg.into())?);
            Ok(())
        }
        Commands::Clean { secrets_nix, file } => cmd.clean(&secrets_nix, &file),
        Commands::Smudge { identities, file } => cmd.smudge(&identities, &file),
        Commands::Textconv { identities, path } => cmd.textconv(&identities, &path),
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
                writeln!(
                    f,
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
