// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Thin binary over the dreamcatcher library.

use anyhow::Result;
use clap::Parser;

use dreamcatcher::cli::Cli;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();
    let cli = Cli::parse();
    let code = dreamcatcher::cli_run(&cli)?;
    match code {
        0 => Ok(()),
        n => std::process::exit(n),
    }
}
