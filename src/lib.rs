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
pub mod spinner;
pub mod style;

use std::io::IsTerminal;
use std::time::Instant;

use style::Color;

/// Crate version, mirrored in `VERSION` and `man/dreamcatcher.1`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the CLI. Returns the process exit code.
///
/// Non-zero exit on parse errors, dangling references, dependency cycles,
/// or (with `--strict`) any warnings.
pub fn cli_run(cli: &cli::Cli) -> anyhow::Result<i32> {
    use clap::CommandFactory as _;
    use cli::Command;

    // Telemetry: warnings by default, -v info, -vv debug; RUST_LOG wins.
    let default_filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_filter)),
        )
        .with_target(false)
        .try_init();

    // Styling and animation only for interactive text output; JSON and
    // pipes stay clean.
    let is_tty = std::io::stderr().is_terminal();
    let text_mode = cli.format == cli::Format::Text;
    style::init(style::should_enable(is_tty) && text_mode);
    let animated = is_tty && text_mode;

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
            println!("✨ wrote dreamcatcher.toml");
            Ok(0)
        }
        Some(Command::Schema) => {
            println!("{}", serde_json::to_string_pretty(&schema::dream_schema())?);
            Ok(0)
        }
        Some(Command::Completions { shell }) => {
            use std::io::Write;
            let mut cmd = cli::Cli::command();
            let name = cmd.get_name().to_string();
            let mut out = std::io::stdout();
            clap_complete::generate(*shell, &mut cmd, name, &mut out);
            out.flush()?;
            Ok(0)
        }
        Some(Command::Validate) => validate(cli, &roots, animated),
        Some(Command::Ir { emit }) => dump_ir(emit, &config, &roots, animated),
        None => compile(cli, &config, &roots, animated),
    }
}

/// Discover with a spinner, surfacing parse errors as styled `❌` lines.
fn discover_or_report(
    roots: &[std::path::PathBuf],
    animated: bool,
    phase: &str,
) -> Result<(Vec<dream::Dream>, discover::DiscoveryStats, Vec<String>), i32> {
    let sp = spinner::Spinner::start(phase, animated);
    match discover::discover(roots) {
        Ok(v) => Ok(v),
        Err(e) => {
            sp.finish("");
            eprintln!("{} {e}", style::paint(Color::Red, "❌ dream parse error:"));
            Err(1)
        }
    }
}

/// `validate` subcommand: parse-check every dream, style the results.
fn validate(cli: &cli::Cli, roots: &[std::path::PathBuf], animated: bool) -> anyhow::Result<i32> {
    let sp = spinner::Spinner::start("🔍 discovering dreams", animated);
    let (dreams, stats, warnings) = match discover::discover_with_progress(roots, |n| {
        sp.set_message(&format!("🔍 scanned {n} files…"));
    }) {
        Ok(v) => v,
        Err(e) => {
            sp.finish("");
            eprintln!("{} {e}", style::paint(Color::Red, "❌ dream parse error:"));
            return Ok(1);
        }
    };
    sp.finish(&format!(
        "🔍 discovery: {} dream(s) in {} files scanned",
        style::paint(Color::Bold, &dreams.len().to_string()),
        stats.files_scanned
    ));
    for d in &dreams {
        println!(
            "{} {} {} {}",
            style::paint(Color::Green, "✅"),
            style::paint(Color::Cyan, &d.id.0),
            style::paint(Color::Dim, "—"),
            d.path
        );
    }
    for w in &warnings {
        eprintln!("{} {w}", style::paint(Color::Yellow, "⚠️ "));
    }
    println!(
        "🌙 {} dream(s) valid",
        style::paint(Color::Bold, &dreams.len().to_string())
    );
    if cli.strict && !warnings.is_empty() {
        return Ok(2);
    }
    Ok(0)
}

/// `ir` subcommand: dump the analysed Dream IR graph as JSON.
fn dump_ir(
    emit_path: &Option<std::path::PathBuf>,
    config: &config::Config,
    roots: &[std::path::PathBuf],
    animated: bool,
) -> anyhow::Result<i32> {
    let (dreams, stats, warnings) =
        match discover_or_report(roots, animated, "🕸️  discovering dreams") {
            Ok(v) => v,
            Err(code) => return Ok(code),
        };
    let ir = ir::DreamIr::build(dreams);
    let analysis = analyze::analyse(&ir, config);
    let mut full = ir.clone();
    full.edges
        .extend(analyze::cluster::cluster_edges(&analysis.clusters));
    full.edges
        .extend(analyze::conflict::conflict_edges(&analysis.conflicts));
    let doc = serde_json::to_string_pretty(&full)?;
    match emit_path {
        Some(path) => std::fs::write(path, doc)?,
        None => println!("{doc}"),
    }
    eprintln!(
        "🕸️  IR: {} nodes · {} edges · {} files scanned",
        style::paint(Color::Bold, &full.nodes.len().to_string()),
        full.edges.len(),
        stats.files_scanned
    );
    for w in &warnings {
        eprintln!("{} {w}", style::paint(Color::Yellow, "⚠️ "));
    }
    Ok(0)
}

/// Default run: discover → analyse → emit, with per-phase telemetry.
fn compile(
    cli: &cli::Cli,
    config: &config::Config,
    roots: &[std::path::PathBuf],
    animated: bool,
) -> anyhow::Result<i32> {
    use cli::Format;

    let t_total = Instant::now();

    // Phase 1: discovery.
    let sp = spinner::Spinner::start("🔍 discovering dreams", animated);
    let t0 = Instant::now();
    let (dreams, stats, warnings) = match discover::discover_with_progress(roots, |n| {
        sp.set_message(&format!("🔍 scanned {n} files…"));
    }) {
        Ok(v) => v,
        Err(e) => {
            sp.finish("");
            eprintln!("{} {e}", style::paint(Color::Red, "❌ dream parse error:"));
            return Ok(1);
        }
    };
    let discovery_ms = t0.elapsed().as_millis() as u64;
    sp.finish(&format!(
        "🔍 discovery: {} dream(s) · {} files scanned",
        style::paint(Color::Bold, &dreams.len().to_string()),
        stats.files_scanned
    ));

    let ir = ir::DreamIr::build(dreams);

    let dangling = ir.dangling_references();
    if !dangling.is_empty() {
        for (from, to) in &dangling {
            eprintln!(
                "{} dream {from} references unknown dream {to}",
                style::paint(Color::Red, "🚫")
            );
        }
        return Ok(1);
    }
    let cycles = ir.dependency_cycles();
    if !cycles.is_empty() {
        for c in &cycles {
            eprintln!(
                "{} dependency cycle: {}",
                style::paint(Color::Red, "🚫"),
                c.join(" -> ")
            );
        }
        return Ok(1);
    }

    // Phase 2: analysis.
    let sp = spinner::Spinner::start("🕸️  analysing the dreamspace", animated);
    let t1 = Instant::now();
    let analysis = analyze::analyse(&ir, config);
    let analysis_ms = t1.elapsed().as_millis() as u64;
    sp.finish(&format!(
        "🕸️  analysis: {} cluster(s) · {} conflict pair(s)",
        style::paint(Color::Bold, &analysis.clusters.len().to_string()),
        analysis.conflicts.len()
    ));
    tracing::info!(
        clusters = analysis.clusters.len(),
        conflicts = analysis.conflicts.len(),
        candidates = analysis.candidates.len(),
        "analysis complete"
    );

    // Phase 3: emission.
    let t2 = Instant::now();
    let written = if cli.dry_run {
        Vec::new()
    } else {
        let written = emit::emit_all(&cli.out, &analysis.candidates, &ir)?;
        for w in &written {
            tracing::info!(path = %w.display(), "wrote deliverable file");
        }
        written
    };
    let emit_ms = t2.elapsed().as_millis() as u64;
    if cli.dry_run {
        eprintln!(
            "{} dry-run: deliverables not written",
            style::paint(Color::Dim, "🧪")
        );
    } else {
        eprintln!(
            "📦 emitted {} file(s) → {}",
            style::paint(Color::Bold, &written.len().to_string()),
            cli.out.display()
        );
    }

    let timings = report::Timings {
        discovery_ms,
        analysis_ms,
        emit_ms,
        total_ms: t_total.elapsed().as_millis() as u64,
    };
    let report = report::build(&ir, &analysis, warnings, stats, timings);

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
