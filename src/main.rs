mod age;
mod cli;
mod commands;
mod ctx;
mod git;
mod nix;

use cli::run;

fn main() -> Result<()> {
    env_logger::init();
    let args = cli::parse_args();
    let repo = git::LibGit2Repository::from_current_dir()?;
    let ctx = ctx::Context::new(&repo);

    run(args, ctx)
}
