// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Emission of `.deliver` contracts, JSON sidecars and the manifest DAG.

pub mod json;
pub mod yaml;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::analyze::Candidate;
use crate::dream::Dream;
use crate::ir::DreamIr;

/// A provenance record tying a deliverable back to its dreams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    /// Dream id.
    pub id: String,
    /// Path of the `.dream` file.
    pub path: String,
    /// sha256 of the `.dream` file.
    pub sha256: String,
}

/// Manifest entry: one node of the deliver DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Deliverable id.
    pub id: String,
    /// Output filename stem.
    pub slug: String,
    /// Source dream ids.
    pub source_dreams: Vec<String>,
    /// Other deliverables this one depends on.
    pub dependencies: Vec<String>,
    /// Priority (highest first ordering is reflected in manifest.json order).
    pub priority: String,
    /// Computed score, descending.
    pub score: i64,
    /// True when a human decision is required before execution.
    pub needs_human_decision: bool,
}

/// The full manifest: the execution DAG Dreamboat walks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Tool that produced this manifest.
    pub generator: String,
    /// Dreamcatcher version.
    pub version: String,
    /// UTC timestamp of emission.
    pub generated_at: String,
    /// Entries in priority order.
    pub deliverables: Vec<ManifestEntry>,
}

/// Render the YAML contract document for a candidate.
pub fn contract_yaml(candidate: &Candidate, ir: &DreamIr) -> anyhow::Result<String> {
    yaml::contract_yaml(candidate, ir)
}

/// Build the dreamboat-compatible JSON sidecar value for a candidate.
pub fn sidecar_json(candidate: &Candidate, ir: &DreamIr) -> serde_json::Value {
    json::sidecar(candidate, ir)
}

/// Write all outputs for the given candidates into `out_dir`.
///
/// Writes `<slug>.deliver` (YAML), `<slug>.deliver.json` (sidecar) and
/// `manifest.json`. Returns the paths written.
pub fn emit_all(
    out_dir: &Path,
    candidates: &[Candidate],
    ir: &DreamIr,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    std::fs::create_dir_all(out_dir)?;
    let weights = crate::config::Weights::default();
    let mut written = Vec::new();
    let mut entries = Vec::new();

    for c in candidates {
        let yaml = yaml::contract_yaml(c, ir)?;
        let ypath = out_dir.join(format!("{}.deliver", c.slug));
        std::fs::write(&ypath, yaml)?;
        let jpath = out_dir.join(format!("{}.deliver.json", c.slug));
        std::fs::write(&jpath, serde_json::to_vec_pretty(&json::sidecar(c, ir))?)?;
        written.push(ypath);
        written.push(jpath);
        entries.push(ManifestEntry {
            id: c.id.clone(),
            slug: c.slug.clone(),
            source_dreams: c.source_dreams.clone(),
            dependencies: c.dependencies.clone(),
            priority: format!("{:?}", c.priority).to_lowercase(),
            score: crate::analyze::prioritise::score(c, &weights),
            needs_human_decision: c.is_conflict || !c.required_human_decisions.is_empty(),
        });
    }

    let manifest = Manifest {
        generator: "dreamcatcher".into(),
        version: crate::VERSION.to_string(),
        generated_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        deliverables: entries,
    };
    let mpath = out_dir.join("manifest.json");
    std::fs::write(&mpath, serde_json::to_vec_pretty(&manifest)?)?;
    written.push(mpath);
    Ok(written)
}

/// Collect provenance records for a candidate's source dreams.
pub fn provenance_for(candidate: &Candidate, ir: &DreamIr) -> Vec<Provenance> {
    candidate
        .source_dreams
        .iter()
        .filter_map(|id| ir.nodes.get(id))
        .map(|d: &Dream| Provenance {
            id: d.id.0.clone(),
            path: d.path.clone(),
            sha256: d.sha256.clone(),
        })
        .collect()
}
