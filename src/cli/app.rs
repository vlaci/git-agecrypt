use anyhow::Result;

use crate::{commands::{internal, public}, git::Repository};

use super::args::{Args,Commands};

pub(crate) fn run(args: Args, repo: Repository) -> Result<()> {
    match args.command {
        Commands::Init => {
            public::init(repo)?;
            println!("Success!");
            Ok(())
        },
        Commands::Deinit => {
            public::deinit(repo)?;
            println!("Success!");
            Ok(())
        },
        Commands::Status => {
            let status = public::status(repo)?;
            print_identities(status.identities);
            Ok(())
        }
        Commands::Config { cfg } => {
            match public::config(repo, cfg.into())? {
                public::ConfigResult::Succeeded => println!("Success!"),
                public::ConfigResult::NothingDone => (),
                public::ConfigResult::Identities(identities) => {
                    print_identities(identities);
                }
            }
            Ok(())
        }
        Commands::Clean { secrets_nix, file } => internal::clean(repo, &secrets_nix, &file),
        Commands::Smudge { identities, file } => internal::smudge(repo, &identities, &file),
        Commands::Textconv { identities, path } => {
            internal::textconv(repo, &identities, &path)
        }
    }?;
    Ok(())
}

fn print_identities(identities: Vec<String>) {
    println!("The following identities are currently configured:");
    for i in identities {
        println!("    {}", i);
    }
}
