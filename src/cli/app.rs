use anyhow::Result;

use crate::{
    commands::{internal, public},
    ctx::Context,
};

use super::args::{Args, Commands, InternalCommands, PublicCommands};

pub(crate) fn run(args: Args, ctx: impl Context) -> Result<()> {
    match args.command {
        Commands::Public(c) => run_public_command(c, ctx),
        Commands::Internal(c) => run_internal_command(c, ctx),
    }
}

fn run_internal_command(
    commands: InternalCommands,
    ctx: impl Context,
) -> Result<(), anyhow::Error> {
    let cmd = internal::CommandContext { ctx };
    match commands {
        InternalCommands::Clean { secrets_nix, file } => cmd.clean(&secrets_nix, &file),
        InternalCommands::Smudge { identities, file } => cmd.smudge(&identities, &file),
        InternalCommands::Textconv { identities, path } => cmd.textconv(&identities, &path),
    }
}

fn run_public_command(commands: PublicCommands, ctx: impl Context) -> Result<(), anyhow::Error> {
    let cmd = public::CommandContext { ctx };
    match commands {
        PublicCommands::Init => {
            let result = cmd.init()?;
            print!("{}", result);
            Ok(())
        }
        PublicCommands::Deinit => {
            let result = cmd.deinit()?;
            print!("{}", result);
            Ok(())
        }
        PublicCommands::Status => {
            let result = cmd.status()?;
            print!("{}", result);
            Ok(())
        }
        PublicCommands::Config { cfg } => {
            print!("{}", cmd.config(cfg.into())?);
            Ok(())
        }
    }
}

impl<T: Display> Display for public::Outcome<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            public::Outcome::Changes(c) => write!(f, "Changes made:\n{}", c),
            public::Outcome::NoChanges => writeln!(f, "No changes needed"),
            public::Outcome::Output(o) => write!(f, "{}", o),
        }
    }
}

impl Display for public::NoOutput {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Display for public::ConfigResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            public::ConfigResult::IdentityAdded => writeln!(f, "Identity added"),
            public::ConfigResult::IdentityRemoved => writeln!(f, "Identity removed"),
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
