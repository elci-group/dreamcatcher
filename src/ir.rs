// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! The Dream IR graph: dreams as nodes, typed relationships as edges.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dream::Dream;

/// Type of a relationship between two dreams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// Declared `depends_on`.
    DependsOn,
    /// Declared `conflicts_with`.
    ConflictsWith,
    /// Declared `supersedes`.
    Supersedes,
    /// Auto-detected similarity.
    RelatedTo,
    /// Cluster membership.
    ConvergesWith,
}

/// A directed edge between dream ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Edge type.
    pub kind: EdgeKind,
    /// Source dream id.
    pub from: String,
    /// Target dream id.
    pub to: String,
    /// How the edge came to exist ("declared" or an analysis note).
    pub origin: String,
}

/// The full intermediate representation over a dreamspace.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DreamIr {
    /// All dreams, keyed by id (BTreeMap for deterministic serialisation).
    pub nodes: BTreeMap<String, Dream>,
    /// Typed edges.
    pub edges: Vec<Edge>,
}

impl DreamIr {
    /// Build the declared (pre-analysis) graph from parsed dreams.
    pub fn build(dreams: Vec<Dream>) -> Self {
        let mut ir = DreamIr {
            nodes: dreams.into_iter().map(|d| (d.id.0.clone(), d)).collect(),
            edges: Vec::new(),
        };
        let ids: Vec<String> = ir.nodes.keys().cloned().collect();
        for id in ids {
            let meta = ir.nodes[&id].meta.clone();
            for dep in meta.depends_on {
                ir.edges.push(Edge {
                    kind: EdgeKind::DependsOn,
                    from: id.clone(),
                    to: dep,
                    origin: "declared".into(),
                });
            }
            for c in meta.conflicts_with {
                ir.edges.push(Edge {
                    kind: EdgeKind::ConflictsWith,
                    from: id.clone(),
                    to: c,
                    origin: "declared".into(),
                });
            }
            for s in meta.supersedes {
                ir.edges.push(Edge {
                    kind: EdgeKind::Supersedes,
                    from: id.clone(),
                    to: s,
                    origin: "declared".into(),
                });
            }
        }
        ir
    }

    /// Dreams whose declared edges point at ids that do not exist.
    pub fn dangling_references(&self) -> Vec<(String, String)> {
        self.edges
            .iter()
            .filter(|e| !self.nodes.contains_key(&e.to))
            .map(|e| (e.from.clone(), e.to.clone()))
            .collect()
    }

    /// Cycles among `depends_on` edges (each cycle sorted, deduped).
    pub fn dependency_cycles(&self) -> Vec<Vec<String>> {
        let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for e in &self.edges {
            if e.kind == EdgeKind::DependsOn && self.nodes.contains_key(&e.to) {
                graph
                    .entry(e.from.as_str())
                    .or_default()
                    .push(e.to.as_str());
            }
        }
        let mut seen = std::collections::HashSet::new();
        let mut cycles = Vec::new();
        for start in graph.keys().copied() {
            // DFS from each node; record any path returning to start.
            let mut stack = vec![(start, vec![start])];
            while let Some((node, path)) = stack.pop() {
                for next in graph.get(node).into_iter().flatten() {
                    if *next == start {
                        let mut cycle: Vec<String> = path.iter().map(|s| s.to_string()).collect();
                        cycle.sort();
                        if seen.insert(cycle.join("->")) {
                            cycles.push(cycle);
                        }
                    } else if !path.contains(next) && path.len() <= self.nodes.len() {
                        let mut p = path.clone();
                        p.push(next);
                        stack.push((next, p));
                    }
                }
            }
        }
        cycles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dream(id: &str, depends: &[&str]) -> Dream {
        Dream {
            id: crate::dream::DreamId(id.into()),
            title: None,
            created: None,
            intent: crate::dream::Intent {
                statement: "s".into(),
                kind: None,
            },
            subject: Default::default(),
            evidence: Default::default(),
            meta: crate::dream::Meta {
                depends_on: depends.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            },
            path: format!("{id}.dream"),
            sha256: id.repeat(64).chars().take(64).collect(),
        }
    }

    #[test]
    fn detects_cycle_and_dangling() {
        let ir = DreamIr::build(vec![
            dream("a", &["b"]),
            dream("b", &["a"]),
            dream("c", &["zzz"]),
        ]);
        assert_eq!(ir.dependency_cycles().len(), 1);
        assert_eq!(ir.dangling_references(), vec![("c".into(), "zzz".into())]);
    }
}
