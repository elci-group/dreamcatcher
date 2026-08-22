// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Synthesis: compile clusters into deliverable candidates.

use std::collections::BTreeSet;

use crate::analyze::{rollup_priority, Candidate};
use crate::config::Config;
use crate::dream::{Dream, Risk};
use crate::ir::DreamIr;

/// Slugify a title for filenames.
pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn title_for(dreams: &[&Dream]) -> String {
    match dreams {
        [one] => one
            .title
            .clone()
            .unwrap_or_else(|| one.intent.statement.clone()),
        many => {
            let subject = many
                .iter()
                .find_map(|d| d.subject.name.clone())
                .unwrap_or_else(|| "shared".into());
            format!("{} evolution ({} dreams)", subject, many.len())
        }
    }
}

fn risk_rollup(dreams: &[&Dream]) -> Option<Risk> {
    dreams.iter().filter_map(|d| d.meta.risk_hint).max()
}

/// Synthesise candidates from clusters and conflicts.
///
/// Conflict pairs are never merged: each pair becomes one
/// `is_conflict` candidate whose only content is the required human
/// decision. Dream-level `depends_on` is mapped to deliverable-level
/// dependencies via cluster membership.
pub fn synthesize(
    ir: &DreamIr,
    clusters: &[Vec<String>],
    conflicts: &[(String, String)],
    order: &[String],
    config: &Config,
) -> Vec<Candidate> {
    // Map dream id -> cluster index.
    let mut cluster_of: std::collections::BTreeMap<&str, usize> = Default::default();
    for (i, c) in clusters.iter().enumerate() {
        for id in c {
            cluster_of.insert(id.as_str(), i);
        }
    }
    let conflicted: BTreeSet<&str> = conflicts
        .iter()
        .flat_map(|(a, b)| [a.as_str(), b.as_str()])
        .collect();

    let _ = config;
    let mut candidates: Vec<Candidate> = Vec::new();

    // Regular candidates, one per cluster (skipping conflicted dreams handled below).
    for (i, cluster) in clusters.iter().enumerate() {
        let dreams: Vec<&Dream> = cluster
            .iter()
            .filter(|id| !conflicted.contains(id.as_str()))
            .map(|id| &ir.nodes[id])
            .collect::<Vec<_>>()
            .into_iter()
            .filter(|d| order.contains(&d.id.0))
            .collect();
        if dreams.is_empty() {
            continue;
        }
        candidates.push(build_candidate(
            &format!("deliver-{:03}", i + 1),
            &dreams,
            &cluster_of,
            clusters,
        ));
    }

    // One conflict candidate per conflict pair.
    for (n, (a, b)) in conflicts.iter().enumerate() {
        let da = &ir.nodes[a];
        let db = &ir.nodes[b];
        let dreams = vec![da, db];
        let mut c = build_candidate(
            &format!("deliver-conflict-{:03}", n + 1),
            &dreams,
            &cluster_of,
            clusters,
        );
        c.is_conflict = true;
        c.objective = format!("Resolve conflicting dreams {a} and {b}");
        c.required_human_decisions = vec![format!(
            "Dreams {a} and {b} conflict (subject: {}). A human must choose a direction or supersede one.",
            da.subject.name.clone().unwrap_or_else(|| "unspecified".into())
        )];
        c.priority = rollup_priority(&dreams).max(crate::dream::Priority::High);
        candidates.push(c);
    }

    // Dreams excluded by dependency cycles are surfaced by the caller as
    // errors in the report; they deliberately produce no candidate here.

    // Ensure slugs are unique: a duplicated slug would make one .deliver
    // file overwrite another.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for c in &mut candidates {
        let base = c.slug.clone();
        let mut n = 1;
        let mut slug = base.clone();
        while !seen.insert(slug.clone()) {
            n += 1;
            slug = format!("{base}-{n}");
        }
        c.slug = slug;
    }
    candidates
}

#[allow(clippy::too_many_lines)]
fn build_candidate(
    id: &str,
    dreams: &[&Dream],
    cluster_of: &std::collections::BTreeMap<&str, usize>,
    clusters: &[Vec<String>],
) -> Candidate {
    let mut scope: BTreeSet<String> = Default::default();
    let mut capabilities: BTreeSet<String> = Default::default();
    let mut constraints: BTreeSet<String> = Default::default();
    let mut acceptance: BTreeSet<String> = Default::default();
    let mut decisions: Vec<String> = Vec::new();
    let mut evidence: BTreeSet<String> = Default::default();
    let mut concern_flagged = false;

    for d in dreams {
        if let Some(name) = &d.subject.name {
            scope.insert(name.clone());
        }
        for cap in &d.meta.requires_capabilities {
            capabilities.insert(cap.clone());
        }
        for ev in &d.evidence.items {
            evidence.insert(format!("{}: {ev}", d.id.0));
        }
        if matches!(d.intent.kind, Some(crate::dream::IntentKind::Concern)) {
            concern_flagged = true;
        }
        for q in &d.meta.open_questions {
            decisions.push(format!("{}: {q}", d.id.0));
        }
        acceptance.insert(format!(
            "{} intent is satisfiable: {}",
            d.id.0, d.intent.statement
        ));
    }
    if concern_flagged {
        constraints.insert("address the underlying concern, not just the symptom".into());
    }
    constraints.insert("deterministic and provenance-preserving".into());

    // Deliverable dependencies: any cluster that a member dream depends on.
    let mut dependencies: BTreeSet<String> = Default::default();
    for d in dreams {
        for dep in &d.meta.depends_on {
            if let Some(&idx) = cluster_of.get(dep.as_str()) {
                if clusters[idx]
                    .iter()
                    .any(|m| !dreams.iter().any(|d| d.id.0 == *m))
                {
                    dependencies.insert(format!("deliver-{:03}", idx + 1));
                }
            }
        }
    }

    let title = title_for(dreams);
    Candidate {
        id: id.to_string(),
        slug: slugify(&title),
        source_dreams: dreams.iter().map(|d| d.id.0.clone()).collect(),
        objective: dreams
            .iter()
            .map(|d| d.intent.statement.clone())
            .collect::<Vec<_>>()
            .join("; "),
        scope: scope.into_iter().collect(),
        required_capabilities: capabilities.into_iter().collect(),
        constraints: constraints.into_iter().collect(),
        acceptance: acceptance.into_iter().collect(),
        priority: rollup_priority(dreams),
        risk: risk_rollup(dreams).map(|r| format!("{r:?}").to_lowercase()),
        dependencies: dependencies.into_iter().collect(),
        required_human_decisions: decisions,
        is_conflict: false,
        evidence: evidence.into_iter().collect(),
        concern_flagged,
    }
}
