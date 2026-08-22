// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Convergence clustering: dreams about the same thing become one candidate.

use std::collections::{BTreeMap, BTreeSet};

use crate::config::Config;
use crate::ir::{DreamIr, Edge, EdgeKind};

/// Lowercase, de-duplicated, sorted token set of a dream's text fields.
fn tokens(text: &str) -> BTreeSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 2)
        .map(str::to_string)
        .collect()
}

/// Jaccard similarity of two token sets.
pub fn similarity(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    inter as f64 / union as f64
}

fn dream_text(ir: &DreamIr, id: &str) -> String {
    let d = &ir.nodes[id];
    format!(
        "{} {} {} {}",
        d.title.as_deref().unwrap_or(""),
        d.intent.statement,
        d.evidence.items.join(" "),
        d.subject.tags.join(" ")
    )
}

/// Group dream ids into clusters (each cluster sorted, clusters sorted by
/// first member for determinism). Singletons are their own cluster.
///
/// Two dreams converge when they share a normalised subject name, or when
/// their text similarity meets the configured threshold. Convergence is
/// transitive via union-find.
pub fn cluster(ir: &DreamIr, config: &Config) -> Vec<Vec<String>> {
    let ids: Vec<String> = ir.nodes.keys().cloned().collect();
    let mut parent: std::collections::BTreeMap<String, String> =
        ids.iter().cloned().map(|id| (id.clone(), id)).collect();

    fn find(parent: &mut std::collections::BTreeMap<String, String>, x: &str) -> String {
        let root = parent[x].clone();
        if root == x {
            return root;
        }
        let r = find(parent, &root);
        parent.insert(x.to_string(), r.clone());
        r
    }

    let texts: Vec<(String, BTreeSet<String>)> = ids
        .iter()
        .map(|id| (id.clone(), tokens(&dream_text(ir, id))))
        .collect();
    let subjects: Vec<(String, String)> = ids
        .iter()
        .map(|id| {
            (
                id.clone(),
                ir.nodes[id]
                    .subject
                    .name
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .to_lowercase(),
            )
        })
        .collect();

    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let (id_a, tok_a) = &texts[i];
            let (id_b, tok_b) = &texts[j];
            let same_subject = !subjects[i].1.is_empty() && subjects[i].1 == subjects[j].1;
            let similar = similarity(tok_a, tok_b) >= config.cluster_similarity_threshold;
            if same_subject || similar {
                let ra = find(&mut parent, id_a);
                let rb = find(&mut parent, id_b);
                if ra != rb {
                    parent.insert(ra, rb);
                }
            }
        }
    }

    let mut groups: BTreeMap<String, Vec<String>> = Default::default();
    for id in &ids {
        let root = find(&mut parent, id);
        groups.entry(root).or_default().push(id.clone());
    }
    let mut clusters: Vec<Vec<String>> = groups.into_values().collect();
    for c in &mut clusters {
        c.sort();
    }
    clusters.sort();
    clusters
}

/// Record `converges_with` edges for cluster membership (multi-member clusters only).
pub fn cluster_edges(clusters: &[Vec<String>]) -> Vec<Edge> {
    let mut edges = Vec::new();
    for cluster in clusters {
        if cluster.len() > 1 {
            for pair in cluster.windows(2) {
                edges.push(Edge {
                    kind: EdgeKind::ConvergesWith,
                    from: pair[0].clone(),
                    to: pair[1].clone(),
                    origin: "cluster".into(),
                });
            }
        }
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jaccard_bounds() {
        let a: BTreeSet<String> = ["schema", "version", "migration"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let b: BTreeSet<String> = ["schema", "version", "registry"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!((similarity(&a, &b) - 0.5).abs() < 1e-9);
        assert_eq!(similarity(&a, &a), 1.0);
    }
}
