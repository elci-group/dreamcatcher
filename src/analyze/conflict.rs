// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Conflict detection: declared conflicts plus same-subject contradictions.

use crate::dream::IntentKind;
use crate::ir::{DreamIr, Edge, EdgeKind};

/// All conflicts as sorted, de-duplicated id pairs.
pub fn detect(ir: &DreamIr) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let push = |a: String, b: String, out: &mut Vec<(String, String)>| {
        let pair = if a <= b { (a, b) } else { (b, a) };
        if !out.contains(&pair) {
            out.push(pair);
        }
    };

    for e in &ir.edges {
        if e.kind == EdgeKind::ConflictsWith && ir.nodes.contains_key(&e.to) {
            push(e.from.clone(), e.to.clone(), &mut out);
        }
    }

    // Detected: same subject, contradictory intent kinds, neither supersedes the other.
    let ids: Vec<&String> = ir.nodes.keys().collect();
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let a = &ir.nodes[ids[i]];
            let b = &ir.nodes[ids[j]];
            let same_subject = a.subject.name.is_some() && a.subject.name == b.subject.name;
            let contradictory = a
                .intent
                .kind
                .zip(b.intent.kind)
                .is_some_and(|(ka, kb)| ka.contradicts(kb));
            let supersedes =
                a.meta.supersedes.contains(&b.id.0) || b.meta.supersedes.contains(&a.id.0);
            if same_subject && contradictory && !supersedes {
                push(a.id.0.clone(), b.id.0.clone(), &mut out);
            }
        }
    }
    out.sort();
    out
}

/// Record `conflicts_with` edges for detected conflicts.
pub fn conflict_edges(conflicts: &[(String, String)]) -> Vec<Edge> {
    conflicts
        .iter()
        .map(|(a, b)| Edge {
            kind: EdgeKind::ConflictsWith,
            from: a.clone(),
            to: b.clone(),
            origin: "detected".into(),
        })
        .collect()
}

/// Kinds are re-exported for tests.
#[allow(dead_code)]
fn _kind_hint(k: IntentKind) -> bool {
    k.contradicts(IntentKind::Concern)
}
