use anyhow::Result;

use crate::{cli::args::ConfigCommand, ctx::Context};

use super::{internal, public};

use super::args::{Args, Commands, InternalCommands, PublicCommands};

pub(crate) fn run(args: Args, ctx: impl Context) -> Result<()> {
    match args.command {
        Commands::Public(c) => run_public_command(c, ctx),
        Commands::Internal(c) => run_internal_command(c, ctx),
    }
}

fn run_internal_command(commands: InternalCommands, ctx: impl Context) -> Result<()> {
    let cmd = internal::CommandContext { ctx };
    match commands {
        InternalCommands::Clean { secrets_nix, file } => cmd.clean(&secrets_nix, &file),
        InternalCommands::Smudge { identities, file } => cmd.smudge(&identities, &file),
        InternalCommands::Textconv { identities, path } => cmd.textconv(&identities, &path),
    }
}

fn run_public_command(commands: PublicCommands, ctx: impl Context) -> Result<()> {
    let cmd = public::CommandContext::new(ctx);
    match commands {
        PublicCommands::Init => {
            cmd.init()?;
        }
        PublicCommands::Deinit => {
            cmd.deinit()?;
        }
        PublicCommands::Status => {
            cmd.status()?;
        }
        PublicCommands::Config { cfg } => match ConfigCommand::from(cfg) {
            ConfigCommand::AddIdentity(path) => cmd.add_identity(path)?,
            ConfigCommand::RemoveIdentity(path) => cmd.remove_identity(path)?,
            ConfigCommand::ListIdentities => cmd.list_identities()?,
        },
    }
    Ok(())
}
