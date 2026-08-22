// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Deterministic prioritisation scoring.

use crate::analyze::Candidate;
use crate::config::Weights;
use crate::dream::Priority;

/// Score a candidate: higher is more valuable / more urgent.
///
/// score = priority*w + evidence*w + (cluster size - 1)*w - effort*w - risk*w
pub fn score(candidate: &Candidate, weights: &Weights) -> i64 {
    let priority = candidate.priority.weight() as i64 * weights.priority as i64;
    let evidence = candidate.evidence.len() as i64 * weights.evidence as i64;
    let convergence =
        (candidate.source_dreams.len().saturating_sub(1)) as i64 * weights.convergence as i64;
    let effort = match candidate.source_dreams.len() {
        0 => 0,
        // Effort hints are per-dream; use the count as a proxy when not
        // otherwise known, kept small so priority dominates.
        n => (n as i64 / 2) * weights.effort as i64,
    };
    let risk = match candidate.risk.as_deref() {
        Some("high") => 2,
        Some("medium") => 1,
        _ => 0,
    } * weights.risk as i64;
    // Conflict candidates are urgent by construction.
    let conflict_boost = if candidate.is_conflict {
        weights.priority as i64
    } else {
        0
    };
    priority + evidence + convergence + conflict_boost - effort - risk
}

/// Rank candidates in place: descending score, then id for determinism.
pub fn rank(candidates: &mut [Candidate]) {
    let weights = WeightOverride::current();
    candidates.sort_by(|a, b| {
        score(b, weights)
            .cmp(&score(a, weights))
            .then_with(|| a.id.cmp(&b.id))
    });
}

/// Allows the default weights to be swapped in tests. v0.1 uses defaults only.
struct WeightOverride;
impl WeightOverride {
    fn current() -> &'static Weights {
        static DEFAULTS: std::sync::OnceLock<Weights> = std::sync::OnceLock::new();
        DEFAULTS.get_or_init(Weights::default)
    }
}

/// Default priority used when a candidate has none.
#[allow(dead_code)]
fn _default_priority() -> Priority {
    Priority::Medium
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(priority: Priority, evidence: usize, sources: usize) -> Candidate {
        Candidate {
            id: "deliver-001".into(),
            slug: "x".into(),
            source_dreams: vec!["a".into(); sources],
            objective: "o".into(),
            scope: vec![],
            required_capabilities: vec![],
            constraints: vec![],
            acceptance: vec![],
            priority,
            risk: None,
            dependencies: vec![],
            required_human_decisions: vec![],
            is_conflict: false,
            evidence: vec!["e".into(); evidence],
            concern_flagged: false,
        }
    }

    #[test]
    fn critical_outranks_low() {
        let w = Weights::default();
        let hi = score(&candidate(Priority::Critical, 0, 1), &w);
        let lo = score(&candidate(Priority::Low, 3, 3), &w);
        assert!(hi > lo);
    }
}
