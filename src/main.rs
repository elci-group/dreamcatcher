// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Thin binary over the dreamcatcher library.

use anyhow::Result;
use clap::Parser;

use dreamcatcher::cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match dreamcatcher::cli_run(&cli)? {
        0 => Ok(()),
        n => std::process::exit(n),
    }
}
