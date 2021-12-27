use anyhow::Result;

mod age;
mod cli;
mod commands;
mod git;
mod nix;

use commands::{internal, public};

fn main() -> Result<()> {
    env_logger::init();
    let cli = cli::parse_args();
    let repo = git::Repository::from_current_dir()?;

    match cli.command {
        cli::Commands::Init => {
            public::init(repo)?;
            println!("Success!");
            Ok(())
        },
        cli::Commands::Deinit => {
            public::deinit(repo)?;
            println!("Success!");
            Ok(())
        },
        cli::Commands::Status => {
            let status = public::status(repo)?;
            print_identities(status.identities);
            Ok(())
        }
        cli::Commands::Config { cfg } => {
            match public::config(repo, cfg.into())? {
                public::ConfigResult::Succeeded => println!("Success!"),
                public::ConfigResult::NothingDone => (),
                public::ConfigResult::Identities(identities) => {
                    print_identities(identities);
                }
            }
            Ok(())
        }
        cli::Commands::Clean { secrets_nix, file } => internal::clean(repo, &secrets_nix, &file),
        cli::Commands::Smudge { identities, file } => internal::smudge(repo, &identities, &file),
        cli::Commands::Textconv { identities, path } => {
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
