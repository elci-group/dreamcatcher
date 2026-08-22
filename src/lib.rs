// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Dreamcatcher: the compiler of the Dream suite.
//!
//! Discovers `.dream` files, builds a Dream IR graph, runs deterministic
//! analysis passes (clustering, conflict detection, dependency resolution,
//! synthesis, prioritisation) and emits `.deliver` execution contracts for
//! Dreamboat.

pub mod analyze;
pub mod cli;
pub mod config;
pub mod discover;
pub mod dream;
pub mod emit;
pub mod ir;
pub mod report;
pub mod schema;

/// Crate version, mirrored in `VERSION` and `man/dreamcatcher.1`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the CLI. Returns the process exit code.
///
/// Non-zero exit on parse errors, dangling references, dependency cycles,
/// or (with `--strict`) any warnings.
pub fn cli_run(cli: &cli::Cli) -> anyhow::Result<i32> {
    use clap::CommandFactory as _;
    use cli::Command;
    use std::io::Write;

    let config = config::load(cli.config.as_deref())?;
    let roots: Vec<std::path::PathBuf> = if cli.root.is_empty() {
        config.roots.iter().map(std::path::PathBuf::from).collect()
    } else {
        cli.root.clone()
    };

    match &cli.command {
        Some(Command::Init) => {
            const TEMPLATE: &str = include_str!("../dreamcatcher.toml.example");
            std::fs::write("dreamcatcher.toml", TEMPLATE)?;
            println!("wrote dreamcatcher.toml");
            Ok(0)
        }
        Some(Command::Schema) => {
            println!("{}", serde_json::to_string_pretty(&schema::dream_schema())?);
            Ok(0)
        }
        Some(Command::Completions { shell }) => {
            let mut cmd = cli::Cli::command();
            let name = cmd.get_name().to_string();
            let mut out = std::io::stdout();
            clap_complete::generate(*shell, &mut cmd, name, &mut out);
            out.flush()?;
            Ok(0)
        }
        Some(Command::Validate) => {
            let (dreams, warnings) =
                discover::discover(&roots).map_err(|e| anyhow::anyhow!("{e}"))?;
            for d in &dreams {
                println!("ok: {} ({})", d.id.0, d.path);
            }
            for w in &warnings {
                eprintln!("warning: {w}");
            }
            if cli.strict && !warnings.is_empty() {
                return Ok(2);
            }
            Ok(0)
        }
        Some(Command::Ir { emit }) => {
            let (dreams, warnings) =
                discover::discover(&roots).map_err(|e| anyhow::anyhow!("{e}"))?;
            let ir = ir::DreamIr::build(dreams);
            let analysis = analyze::analyse(&ir, &config);
            let mut full = ir.clone();
            full.edges
                .extend(analyze::cluster::cluster_edges(&analysis.clusters));
            full.edges
                .extend(analyze::conflict::conflict_edges(&analysis.conflicts));
            let doc = serde_json::to_string_pretty(&full)?;
            match emit {
                Some(path) => std::fs::write(path, doc)?,
                None => println!("{doc}"),
            }
            let _ = warnings;
            Ok(0)
        }
        None => compile(cli, &config, &roots),
    }
}

fn compile(
    cli: &cli::Cli,
    config: &config::Config,
    roots: &[std::path::PathBuf],
) -> anyhow::Result<i32> {
    use cli::Format;

    let (dreams, warnings) = discover::discover(roots).map_err(|e| anyhow::anyhow!("{e}"))?;
    let ir = ir::DreamIr::build(dreams);

    let dangling = ir.dangling_references();
    if !dangling.is_empty() {
        for (from, to) in &dangling {
            eprintln!("error: dream {from} references unknown dream {to}");
        }
        return Ok(1);
    }
    let cycles = ir.dependency_cycles();
    if !cycles.is_empty() {
        for c in &cycles {
            eprintln!("error: dependency cycle: {}", c.join(" -> "));
        }
        return Ok(1);
    }

    let analysis = analyze::analyse(&ir, config);
    let report = report::build(&ir, &analysis, warnings.clone());

    if !cli.dry_run {
        emit::emit_all(&cli.out, &analysis.candidates, &ir)?;
    }

    match cli.format {
        Format::Text => print!("{}", report::render_text(&report)),
        Format::Json => println!("{}", report::render_json(&report)?),
    }

    if cli.strict && !report.warnings.is_empty() {
        return Ok(2);
    }
    if !report.cycle_excluded.is_empty() {
        return Ok(1);
    }
    Ok(0)
}
