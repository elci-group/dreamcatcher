// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Human and JSON reports of a compilation run.
//!
//! Text output is emoji-led, colourised (when interactive) and laid out as
//! a box-drawn table; JSON output stays machine-readable but carries the
//! same telemetry (files scanned, per-phase timings, roots).

use serde::Serialize;

use crate::analyze::Analysis;
use crate::discover::DiscoveryStats;
use crate::ir::DreamIr;
use crate::style::{self, Color};

/// Wall-clock telemetry per phase.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Timings {
    /// Discovery + parse, in milliseconds.
    pub discovery_ms: u64,
    /// IR analysis passes, in milliseconds.
    pub analysis_ms: u64,
    /// Writing deliverables, in milliseconds.
    pub emit_ms: u64,
    /// Whole run, in milliseconds.
    pub total_ms: u64,
}

/// Summary of one compilation run.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Number of dreams discovered.
    pub dreams: usize,
    /// Number of clusters (convergence groups).
    pub clusters: usize,
    /// Conflicts detected (pairs).
    pub conflicts: usize,
    /// Deliverables emitted (in priority order).
    pub deliverables: Vec<DeliverableLine>,
    /// Dream ids excluded due to dependency cycles.
    pub cycle_excluded: Vec<String>,
    /// Non-fatal warnings (duplicates, walk errors).
    pub warnings: Vec<String>,
    /// Files walked during discovery (all extensions).
    pub files_scanned: usize,
    /// Discovery roots (normalised).
    pub roots: Vec<String>,
    /// Per-phase wall-clock timings.
    pub timings: Timings,
}

/// One line of the deliverables summary.
#[derive(Debug, Serialize)]
pub struct DeliverableLine {
    /// Deliverable id.
    pub id: String,
    /// Slug.
    pub slug: String,
    /// Priority.
    pub priority: String,
    /// Score.
    pub score: i64,
    /// Source dream count.
    pub dreams: usize,
    /// Human decision required?
    pub human: bool,
}

/// Build the report from an analysis plus discovery telemetry.
pub fn build(
    ir: &DreamIr,
    analysis: &Analysis,
    warnings: Vec<String>,
    stats: DiscoveryStats,
    timings: Timings,
) -> Report {
    let weights = crate::config::Weights::default();
    Report {
        dreams: ir.nodes.len(),
        clusters: analysis.clusters.len(),
        conflicts: analysis.conflicts.len(),
        deliverables: analysis
            .candidates
            .iter()
            .map(|c| DeliverableLine {
                id: c.id.clone(),
                slug: c.slug.clone(),
                priority: format!("{:?}", c.priority).to_lowercase(),
                score: crate::analyze::prioritise::score(c, &weights),
                dreams: c.source_dreams.len(),
                human: c.is_conflict || !c.required_human_decisions.is_empty(),
            })
            .collect(),
        cycle_excluded: analysis.cycle_excluded.clone(),
        warnings,
        files_scanned: stats.files_scanned,
        roots: stats.roots,
        timings,
    }
}

/// Pad `s` with spaces to `w` display columns (best effort with emoji).
fn pad(s: &str, w: usize) -> String {
    let n = s.chars().count();
    format!("{s}{}", " ".repeat(w.saturating_sub(n)))
}

fn priority_color(priority: &str) -> Color {
    match priority {
        "critical" => Color::Red,
        "high" => Color::Yellow,
        "medium" => Color::Cyan,
        _ => Color::Dim,
    }
}

/// Render the report as styled text with emoji and a box-drawn table.
pub fn render_text(report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&style::paint(
        Color::Bold,
        &format!(
            "🌙 dreamcatcher v{} — compiling the dreamspace",
            crate::VERSION
        ),
    ));
    out.push_str("\n\n");
    out.push_str(&format!(
        "📜 {} dreams   🕸️ {} clusters   ⚔️ {} conflicts   🎯 {} deliverables\n",
        style::paint(Color::Bold, &report.dreams.to_string()),
        report.clusters,
        report.conflicts,
        style::paint(Color::Bold, &report.deliverables.len().to_string()),
    ));

    if !report.deliverables.is_empty() {
        out.push_str("\n🎯 deliverables (priority order)\n");
        let headers = ["deliverable", "slug", "priority", "⚡", "📜", "status"];
        let rows: Vec<[String; 6]> = report
            .deliverables
            .iter()
            .map(|d| {
                [
                    d.id.clone(),
                    d.slug.clone(),
                    style::paint(priority_color(&d.priority), &d.priority),
                    d.score.to_string(),
                    d.dreams.to_string(),
                    if d.human {
                        style::paint(Color::Magenta, "🙋 needs decision")
                    } else {
                        style::paint(Color::Green, "✅ ready")
                    },
                ]
            })
            .collect();
        let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
        for row in &rows {
            for (i, cell) in row.iter().enumerate() {
                widths[i] = widths[i].max(cell.chars().count());
            }
        }
        let rule = |l: char, m: char, r: char| {
            let segs: Vec<String> = widths.iter().map(|w| "─".repeat(w + 2)).collect();
            format!(" {l}{}{r}\n", segs.join(&m.to_string()))
        };
        out.push_str(&rule('┌', '┬', '┐'));
        out.push_str(&format!(
            " │ {} │ {} │ {} │ {} │ {} │ {} │\n",
            pad(headers[0], widths[0]),
            pad(headers[1], widths[1]),
            pad(headers[2], widths[2]),
            pad(headers[3], widths[3]),
            pad(headers[4], widths[4]),
            pad(headers[5], widths[5]),
        ));
        out.push_str(&rule('├', '┼', '┤'));
        for row in &rows {
            out.push_str(&format!(
                " │ {} │ {} │ {} │ {} │ {} │ {} │\n",
                pad(&row[0], widths[0]),
                pad(&row[1], widths[1]),
                pad(&row[2], widths[2]),
                pad(&row[3], widths[3]),
                pad(&row[4], widths[4]),
                pad(&row[5], widths[5]),
            ));
        }
        out.push_str(&rule('└', '┴', '┘'));
    }

    if !report.cycle_excluded.is_empty() {
        out.push_str(&format!(
            "\n🚫 excluded by dependency cycles: {}\n",
            style::paint(Color::Red, &report.cycle_excluded.join(", "))
        ));
    }
    for w in &report.warnings {
        out.push_str(&format!("{} {w}\n", style::paint(Color::Yellow, "⚠️ ")));
    }
    out.push_str(&format!(
        "\n🔍 scanned {} files across {}\n",
        report.files_scanned,
        report.roots.join(", ")
    ));
    out.push_str(&format!(
        "{} discovery {}ms · analysis {}ms · emit {}ms · total {}ms\n",
        style::paint(Color::Dim, "⏱️"),
        report.timings.discovery_ms,
        report.timings.analysis_ms,
        report.timings.emit_ms,
        report.timings.total_ms,
    ));
    out
}

/// Render the report as JSON (machine-readable; telemetry included).
pub fn render_json(report: &Report) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(report)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Report {
        Report {
            dreams: 3,
            clusters: 1,
            conflicts: 2,
            deliverables: vec![DeliverableLine {
                id: "deliver-conflict-001".into(),
                slug: "s".into(),
                priority: "high".into(),
                score: 42,
                dreams: 2,
                human: true,
            }],
            cycle_excluded: vec![],
            warnings: vec![],
            files_scanned: 12,
            roots: vec!["/tmp/x".into()],
            timings: Timings {
                discovery_ms: 5,
                analysis_ms: 1,
                emit_ms: 2,
                total_ms: 8,
            },
        }
    }

    #[test]
    fn text_report_is_emoji_led() {
        let text = render_text(&sample());
        assert!(text.contains('🌙'));
        assert!(text.contains('🎯'));
        assert!(text.contains('🙋'));
        assert!(text.contains('⏱'));
        assert!(text.contains("deliver-conflict-001"));
    }

    #[test]
    fn json_report_carries_telemetry() {
        let json = render_json(&sample()).unwrap();
        assert!(json.contains("\"timings\""));
        assert!(json.contains("\"files_scanned\""));
        assert!(json.contains("\"roots\""));
    }
}
