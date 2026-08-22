// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Deterministic analysis passes over the Dream IR.

pub mod cluster;
pub mod conflict;
pub mod dependency;
pub mod prioritise;
pub mod synthesize;

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::dream::{Dream, Priority};
use crate::ir::DreamIr;

/// A candidate outcome produced by synthesis, before prioritisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    /// Deliverable id, e.g. `deliver-001`.
    pub id: String,
    /// Slug used for filenames.
    pub slug: String,
    /// Source dream ids, deterministically sorted.
    pub source_dreams: Vec<String>,
    /// Compiled objective statement.
    pub objective: String,
    /// Scope (subject names).
    pub scope: Vec<String>,
    /// Unioned required capabilities.
    pub required_capabilities: Vec<String>,
    /// Unioned constraints (derived from kinds and open questions).
    pub constraints: Vec<String>,
    /// Deterministic acceptance checks derived from the dreams.
    pub acceptance: Vec<String>,
    /// Rolled-up priority.
    pub priority: Priority,
    /// Unioned risks (max level).
    pub risk: Option<String>,
    /// Ids of other deliverables this one depends on.
    pub dependencies: Vec<String>,
    /// Decisions a human must make before Dreamboat executes.
    pub required_human_decisions: Vec<String>,
    /// True when the candidate exists to surface an unresolved conflict.
    pub is_conflict: bool,
    /// Unioned evidence across source dreams.
    pub evidence: Vec<String>,
    /// True if any source dream declared a concern kind.
    pub concern_flagged: bool,
}

/// Result of running the full pipeline over an IR.
#[derive(Debug, Clone)]
pub struct Analysis {
    /// Cluster assignments (dream id -> cluster index).
    pub clusters: Vec<Vec<String>>,
    /// Detected conflicts (pairs of dream ids, sorted).
    pub conflicts: Vec<(String, String)>,
    /// Topologically valid dream ordering (cycles excluded).
    pub order: Vec<String>,
    /// Dream ids excluded due to dependency cycles.
    pub cycle_excluded: Vec<String>,
    /// Synthesised candidates.
    pub candidates: Vec<Candidate>,
}

/// Run all passes: cluster -> conflict -> dependency -> synthesise.
pub fn analyse(ir: &DreamIr, config: &Config) -> Analysis {
    let clusters = cluster::cluster(ir, config);
    let conflicts = conflict::detect(ir);
    let (order, cycle_excluded) = dependency::topological_order(ir);
    let mut candidates = synthesize::synthesize(ir, &clusters, &conflicts, &order, config);
    prioritise::rank(&mut candidates);
    Analysis {
        clusters,
        conflicts,
        order,
        cycle_excluded,
        candidates,
    }
}

/// Roll up priority hints: max of members, defaulting to Medium.
pub(crate) fn rollup_priority(dreams: &[&Dream]) -> Priority {
    dreams
        .iter()
        .filter_map(|d| d.meta.priority_hint)
        .max()
        .unwrap_or(Priority::Medium)
}
