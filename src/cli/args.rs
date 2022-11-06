use std::path::PathBuf;

use clap::{ArgGroup, Parser, Subcommand};

/// Transparently encrypt/decrypt age secrets
#[derive(Parser)]
#[clap(author, version, about)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
#[clap(
    after_help = "In addition to the above, The following subcommands are used from git filters:
    clean, smudge, textconv"
)]
pub enum Commands {
    #[command(flatten)]
    Public(PublicCommands),
    #[command(flatten)]
    Internal(InternalCommands),
}

#[derive(Subcommand)]
pub enum PublicCommands {
    /// Set-up repository for use with git-agecrypt
    Init,

    /// Display configuration status information
    Status,

    /// Configure encryption settings
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Remove repository specific configuration
    Deinit,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Add a configuration entry
    Add(AddConfig),

    /// Remove a configuration entry
    Remove(RemoveConfig),

    /// List configuration entries
    List(ConfigType),
}

#[derive(clap::Args)]
#[clap(group(
    ArgGroup::new("config")
        .args(&["identity", "recipient"])
        .required(true)
))]
#[clap(group(
    ArgGroup::new("rec")
        .args(&["recipient"])
        .requires("path")
))]
pub struct AddConfig {
    /// Identity usable for decryption
    #[arg(short, long, num_args = 1.., group = "config")]
    identity: Option<PathBuf>,

    /// Recipient for encryption
    #[arg(short, long, num_args = 1.., group = "config")]
    recipient: Option<Vec<String>>,

    /// Path to encrypt for the given recipient
    #[arg(short, long, num_args = 1..)]
    path: Option<Vec<PathBuf>>,
}

pub(crate) enum ModifyConfig {
    Identity(PathBuf),
    Recipient(Vec<PathBuf>, Vec<String>),
}

impl From<AddConfig> for ModifyConfig {
    fn from(val: AddConfig) -> Self {
        if let Some(identity) = val.identity {
            Self::Identity(identity)
        } else if let Some(recipients) = val.recipient {
            Self::Recipient(val.path.unwrap(), recipients)
        } else {
            panic!("Misconfigured config parser")
        }
    }
}

#[derive(clap::Args)]
#[clap(group(
    ArgGroup::new("config")
        .args(&["identity", "recipient"])
))]
pub struct RemoveConfig {
    /// Identity usable for decryption
    #[clap(short, long, group = "config")]
    identity: Option<PathBuf>,

    /// Recipient for encryption
    #[clap(short, long, group = "config")]
    recipient: Option<Vec<String>>,

    /// Path to encrypt for the given recipient
    #[clap(short, long)]
    path: Option<Vec<PathBuf>>,
}

impl From<RemoveConfig> for ModifyConfig {
    fn from(val: RemoveConfig) -> Self {
        if let Some(identity) = val.identity {
            Self::Identity(identity)
        } else if let Some(recipients) = val.recipient {
            Self::Recipient(val.path.unwrap_or_default(), recipients)
        } else if let Some(paths) = val.path {
            Self::Recipient(paths, vec![])
        } else {
            panic!("Misconfigured config parser")
        }
    }
}

#[derive(clap::Args)]
#[clap(group(
    ArgGroup::new("type")
        .args(&["identity", "recipient"])
        .required(true)
))]
pub struct ConfigType {
    /// Identity usable for decryption
    #[arg(short, long)]
    identity: bool,

    /// Recipient for encryption
    #[arg(short, long)]
    recipient: bool,
}

pub(crate) enum QueryConfig {
    Identities,
    Recipients,
}

impl From<ConfigType> for QueryConfig {
    fn from(val: ConfigType) -> Self {
        if val.identity {
            Self::Identities
        } else if val.recipient {
            Self::Recipients
        } else {
            panic!("Misconfigured config parser")
        }
    }
}

#[derive(Subcommand)]
pub enum InternalCommands {
    /// Encrypt files for commit
    #[command(hide = true)]
    Clean {
        /// File to clean
        #[clap(short, long)]
        file: PathBuf,
    },

    /// Decrypt files from checkout
    #[command(hide = true)]
    Smudge {
        /// File to smudge
        #[clap(short, long)]
        file: PathBuf,
    },

    // Decrypt files for diff
    #[command(hide = true)]
    Textconv {
        /// File to show
        path: PathBuf,
    },
}

pub fn parse_args() -> Args {
    Args::parse()
}
