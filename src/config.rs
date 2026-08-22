// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Configuration loading (`dreamcatcher.toml`), with serde defaults everywhere.

use serde::{Deserialize, Serialize};

/// Weights and thresholds for the deterministic analysis passes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Token-overlap similarity (0.0-1.0) above which two dreams converge.
    pub cluster_similarity_threshold: f64,
    /// Scoring weights.
    pub weights: Weights,
    /// Directories to scan when no --root is given.
    pub roots: Vec<String>,
    /// Output directory for deliverables.
    pub out_dir: String,
}

/// Scoring weights for prioritisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Weights {
    /// Weight per priority level step.
    pub priority: u32,
    /// Weight per evidence item.
    pub evidence: u32,
    /// Weight per additional dream in a cluster.
    pub convergence: u32,
    /// Penalty per unit of effort.
    pub effort: u32,
    /// Penalty per risk level above low.
    pub risk: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cluster_similarity_threshold: 0.34,
            weights: Weights::default(),
            roots: vec![".".into()],
            out_dir: "deliverables".into(),
        }
    }
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            priority: 10,
            evidence: 2,
            convergence: 3,
            effort: 3,
            risk: 2,
        }
    }
}

/// Load config from an optional path; fall back to defaults.
pub fn load(path: Option<&std::path::Path>) -> anyhow::Result<Config> {
    match path {
        Some(p) => {
            let text = std::fs::read_to_string(p)
                .map_err(|e| anyhow::anyhow!("reading {}: {e}", p.display()))?;
            Ok(toml::from_str(&text)?)
        }
        None => Ok(Config::default()),
    }
}
