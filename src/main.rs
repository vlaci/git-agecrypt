// SPDX-FileCopyrightText: 2021 László Vaskó <opensource@vlaci.email>
//
// SPDX-License-Identifier: MPL-2.0

mod age;
mod cache;
mod cli;
mod config;
mod ctx;
mod git;

use anyhow::Result;
use cli::run;

fn main() -> Result<()> {
    env_logger::init();
    let args = cli::parse_args();
    let repo = git::LibGit2Repository::from_current_dir()?;
    let ctx = ctx::new(repo);

    run(args, ctx)
}
