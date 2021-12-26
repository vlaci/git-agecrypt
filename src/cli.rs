use std::path::PathBuf;

use clap::{AppSettings, Parser, Subcommand};

/// Transparently encrypt/decrypt age secrets
#[derive(Parser)]
#[clap(author, version, about)]
#[clap(global_setting(AppSettings::UseLongFormatForHelpSubcommand))]
#[clap(global_setting(AppSettings::DeriveDisplayOrder))]
#[clap(setting(AppSettings::SubcommandRequiredElseHelp))]
pub(crate) struct Args {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Encrypt files for commit
    Clean {
        /// Path to secrets.nix
        #[clap(short, long, default_value = "secrets/secrets.nix")]
        secrets_nix: PathBuf,

        /// File to clean
        #[clap(short, long)]
        file: PathBuf,
    },

    /// Decrypt files from checkout
    Smudge {
        #[clap(short, long)]
        identities: Vec<String>,

        /// File to smudge
        #[clap(short, long)]
        file: PathBuf,
    },

    // Decrypt files for diff
    Textconv {
        /// Additional identities to use
        #[clap(short, long)]
        identities: Vec<String>,

        /// File to show
        path: PathBuf,
    },
}

pub(crate) fn parse_args() -> Args {
    Args::parse()
}
