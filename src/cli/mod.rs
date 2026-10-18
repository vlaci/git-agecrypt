// SPDX-FileCopyrightText: 2021 László Vaskó <opensource@vlaci.email>
//
// SPDX-License-Identifier: MPL-2.0

mod app;
mod args;
mod internal;
mod public;
pub(crate) use app::run;
pub(crate) use args::parse_args;
