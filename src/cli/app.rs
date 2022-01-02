use anyhow::Result;

use crate::ctx::Context;

use super::{internal, public};

use super::args::{Args, Commands, InternalCommands, ModifyConfig, PublicCommands, QueryConfig};

pub(crate) fn run(args: Args, ctx: impl Context) -> Result<()> {
    match args.command {
        Commands::Public(c) => run_public_command(c, ctx),
        Commands::Internal(c) => run_internal_command(c, ctx),
    }
}

fn run_internal_command(commands: InternalCommands, ctx: impl Context) -> Result<()> {
    let cmd = internal::CommandContext { ctx };
    match commands {
        InternalCommands::Clean { file } => cmd.clean(&file),
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
        PublicCommands::Config(cfg) => match cfg {
            super::args::ConfigCommands::Add(what) => match ModifyConfig::from(what) {
                ModifyConfig::Identity(id) => cmd.add_identity(id)?,
                ModifyConfig::Recipient(paths, recipients) => {
                    cmd.add_recipients(recipients, paths)?
                }
            },
            super::args::ConfigCommands::Remove(what) => match ModifyConfig::from(what) {
                ModifyConfig::Identity(id) => cmd.remove_identity(id)?,
                ModifyConfig::Recipient(paths, recipients) => {
                    cmd.remove_recipients(recipients, paths)?
                }
            },
            super::args::ConfigCommands::List(what) => match QueryConfig::from(what) {
                QueryConfig::Identities => cmd.list_identities()?,
                QueryConfig::Recipients => cmd.list_recipients()?,
            },
        },
    }
    Ok(())
}
