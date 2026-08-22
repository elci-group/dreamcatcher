// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Clap CLI definitions only.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Output format for reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Human-readable text.
    Text,
    /// JSON.
    Json,
}

/// Compile .dream files into .deliver execution contracts.
#[derive(Debug, Parser)]
#[command(name = "dreamcatcher", version, about, long_about = None)]
pub struct Cli {
    /// Discovery roots (repeatable; default: current directory).
    #[arg(long, value_name = "DIR")]
    pub root: Vec<PathBuf>,

    /// Config file path.
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Output directory for deliverables.
    #[arg(short, long, value_name = "DIR", default_value = "deliverables")]
    pub out: PathBuf,

    /// Report format.
    #[arg(short = 'f', long, value_enum, default_value = "text")]
    pub format: Format,

    /// Analyse and report without writing deliverables.
    #[arg(long)]
    pub dry_run: bool,

    /// Treat warnings as errors.
    #[arg(long)]
    pub strict: bool,

    /// Subcommand.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Validate all .dream files (parse-check only, no emit).
    Validate,
    /// Dump the Dream IR graph as JSON.
    Ir {
        /// Write to a file instead of stdout.
        #[arg(long, value_name = "PATH")]
        emit: Option<PathBuf>,
    },
    /// Write a commented dreamcatcher.toml template.
    Init,
    /// Print the JSON Schema of the .dream format.
    Schema,
    /// Generate shell completions.
    Completions {
        /// Target shell.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}
