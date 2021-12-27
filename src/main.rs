use anyhow::Result;

mod age;
mod cli;
mod commands;
mod git;
mod nix;

use cli::run;

fn main() -> Result<()> {
    env_logger::init();
    let cli = cli::parse_args();
    let repo = git::Repository::from_current_dir()?;

    run(cli, repo)
}
