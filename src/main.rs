mod age;
mod cli;
mod commands;
mod git;
mod nix;
mod ctx;

use cli::run;

fn main() -> Result<()> {
    env_logger::init();
    let args = cli::parse_args();
    let repo = git::Repository::from_current_dir()?;
    let ctx = ctx::Context::new(repo);

    run(args, ctx)
}
