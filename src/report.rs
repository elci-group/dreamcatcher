// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Human and JSON reports of a compilation run.

use serde::Serialize;

use crate::analyze::Analysis;
use crate::ir::DreamIr;

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

/// Build the report from an analysis.
pub fn build(ir: &DreamIr, analysis: &Analysis, warnings: Vec<String>) -> Report {
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
    }
}

/// Render the report as text.
pub fn render_text(report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "dreams: {}  clusters: {}  conflicts: {}  deliverables: {}\n",
        report.dreams,
        report.clusters,
        report.conflicts,
        report.deliverables.len()
    ));
    if !report.deliverables.is_empty() {
        out.push_str("\ndeliverables (priority order):\n");
        for d in &report.deliverables {
            out.push_str(&format!(
                "  {:<22} {:<8} score {:<4} dreams {} {}\n",
                d.id,
                d.priority,
                d.score,
                d.dreams,
                if d.human {
                    "[needs human decision]"
                } else {
                    ""
                }
            ));
        }
    }
    if !report.cycle_excluded.is_empty() {
        out.push_str(&format!(
            "\nEXCLUDED by dependency cycles: {}\n",
            report.cycle_excluded.join(", ")
        ));
    }
    for w in &report.warnings {
        out.push_str(&format!("warning: {w}\n"));
    }
    out
}

/// Render the report as JSON.
pub fn render_json(report: &Report) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(report)?)
}
