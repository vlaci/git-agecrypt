use std::path::PathBuf;

use clap::{AppSettings, Args, Parser, Subcommand};

use crate::commands::public::ConfigCommand;

/// Transparently encrypt/decrypt age secrets
#[derive(Parser)]
#[clap(author, version, about)]
#[clap(global_setting(AppSettings::UseLongFormatForHelpSubcommand))]
#[clap(global_setting(AppSettings::DeriveDisplayOrder))]
#[clap(setting(AppSettings::SubcommandRequiredElseHelp))]
pub(crate) struct CliArgs {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
#[clap(
    after_help = "In addition to the above, The following subcommands are used from git filters:
    clean, smudge, textconv"
)]
pub(crate) enum Commands {
    /// Set-up repository for use with git-agenix
    Init,

    /// Configure encryption settings
    Config {
        #[clap(flatten)]
        cfg: Config,
    },

    /// Remove repository specific configuration
    Deinit,

    /// Encrypt files for commit
    #[clap(setting(AppSettings::Hidden))]
    Clean {
        /// Path to secrets.nix
        #[clap(short, long, default_value = "secrets/secrets.nix")]
        secrets_nix: PathBuf,

        /// File to clean
        #[clap(short, long)]
        file: PathBuf,
    },

    /// Decrypt files from checkout
    #[clap(setting(AppSettings::Hidden))]
    Smudge {
        #[clap(short, long)]
        identities: Vec<String>,

        /// File to smudge
        #[clap(short, long)]
        file: PathBuf,
    },

    // Decrypt files for diff
    #[clap(setting(AppSettings::Hidden))]
    Textconv {
        /// Additional identities to use
        #[clap(short, long)]
        identities: Vec<String>,

        /// File to show
        path: PathBuf,
    },
}

#[derive(Args)]
pub(crate) struct Config {
    /// Register identity usable for decryption
    #[clap(short, long, group = "config")]
    add_identity: Option<PathBuf>,

    /// Remove registered identity
    #[clap(short, long, group = "config")]
    remove_identity: Option<PathBuf>,

    /// List registered identities
    #[clap(short, long, group = "config")]
    list_identities: bool,
}

impl From<Config> for ConfigCommand {
    fn from(val: Config) -> Self {
        if let Some(identity) = val.add_identity {
            Self::AddIdentity(identity)
        } else if let Some(identity) = val.remove_identity {
            Self::RemoveIdentity(identity)
        } else if val.list_identities {
            Self::ListIdentities
        } else {
            panic!("Misconfigured config parser")
        }
    }
}

pub(crate) fn parse_args() -> CliArgs {
    CliArgs::parse()
}
