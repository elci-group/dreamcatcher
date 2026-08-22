// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Dependency resolution and topological ordering.

use std::collections::BTreeSet;

use crate::ir::{DreamIr, EdgeKind};

/// Kahn topological sort over `depends_on` edges restricted to known nodes.
/// Dreams caught in a cycle are excluded and reported separately.
pub fn topological_order(ir: &DreamIr) -> (Vec<String>, Vec<String>) {
    let mut deps: std::collections::BTreeMap<String, BTreeSet<String>> = ir
        .nodes
        .keys()
        .map(|id| (id.clone(), BTreeSet::new()))
        .collect();
    for e in &ir.edges {
        if e.kind == EdgeKind::DependsOn && ir.nodes.contains_key(&e.to) {
            deps.entry(e.from.clone()).or_default().insert(e.to.clone());
        }
    }

    let mut resolved: Vec<String> = Vec::new();
    let mut remaining: BTreeSet<String> = deps.keys().cloned().collect();
    let mut ready: Vec<String> = remaining
        .iter()
        .filter(|id| deps[*id].is_subset(&resolved.iter().cloned().collect()))
        .cloned()
        .collect();
    ready.sort();
    let mut ready: std::collections::BTreeSet<String> = ready.into_iter().collect();

    while let Some(id) = ready.iter().next().cloned() {
        ready.remove(&id);
        remaining.remove(&id);
        resolved.push(id.clone());
        for other in &remaining {
            let now_unblocked = deps[other]
                .iter()
                .all(|d| resolved.contains(d) || !remaining.contains(d));
            if now_unblocked && !ready.contains(other) {
                ready.insert(other.clone());
            }
        }
    }

    let cycle_excluded: Vec<String> = remaining.into_iter().collect();
    (resolved, cycle_excluded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::{Dream, DreamId, Intent, Meta};

    fn dream(id: &str, depends: &[&str]) -> Dream {
        Dream {
            id: DreamId(id.into()),
            title: None,
            created: None,
            intent: Intent {
                statement: "s".into(),
                kind: None,
            },
            subject: Default::default(),
            evidence: Default::default(),
            meta: Meta {
                depends_on: depends.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            },
            path: format!("{id}.dream"),
            sha256: id.repeat(64).chars().take(64).collect(),
        }
    }

    #[test]
    fn orders_and_excludes_cycles() {
        let ir = DreamIr::build(vec![
            dream("a", &["b"]),
            dream("b", &[]),
            dream("x", &["y"]),
            dream("y", &["x"]),
        ]);
        let (order, excluded) = topological_order(&ir);
        assert_eq!(order, vec!["b".to_string(), "a".to_string()]);
        assert_eq!(excluded.len(), 2);
    }
}
