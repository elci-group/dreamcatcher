// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Discovery of `.dream` files under one or more roots.

use std::path::{Path, PathBuf};

use crate::dream::{parse_dream, Dream, ParseError};

/// Directories never descended into. Hidden directories (any name starting
/// with `.`) are always skipped — walking `.rustup`/`.cargo`-scale trees is
/// what made early builds appear to hang.
const SKIP_DIRS: [&str; 2] = ["target", "node_modules"];

/// Aggregate discovery telemetry for the report.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryStats {
    /// Regular files walked (all extensions).
    pub files_scanned: usize,
    /// Normalised discovery roots.
    pub roots: Vec<String>,
}

/// Find and parse every `*.dream` file under `roots`.
///
/// Hidden directories, `target/` and `node_modules/` are never entered; an
/// explicitly named root is always entered regardless. Duplicate content
/// (same sha256) is reported as a warning and only the first occurrence is
/// kept. `on_progress` fires every 1000 files scanned so callers can keep
/// the user informed on large trees.
pub fn discover_with_progress<F: FnMut(usize)>(
    roots: &[PathBuf],
    mut on_progress: F,
) -> Result<(Vec<Dream>, DiscoveryStats, Vec<String>), ParseError> {
    let mut dreams = Vec::new();
    let mut warnings = Vec::new();
    let mut stats = DiscoveryStats::default();
    let mut seen_hashes = std::collections::HashSet::new();
    let mut seen_ids = std::collections::HashMap::new();

    for root in roots {
        let root = if root.is_absolute() {
            root.clone()
        } else {
            std::env::current_dir().unwrap_or_default().join(root)
        };
        let canonical = root.canonicalize().unwrap_or_else(|_| root.clone());
        stats.roots.push(canonical.display().to_string());
        for entry in walkdir::WalkDir::new(&root)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                let name = e.file_name().to_string_lossy();
                !(name.starts_with('.') || SKIP_DIRS.contains(&name.as_ref()))
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warnings.push(format!("walk error under {}: {e}", root.display()));
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            stats.files_scanned += 1;
            if stats.files_scanned % 1000 == 0 {
                on_progress(stats.files_scanned);
            }
            if entry.path().extension().and_then(|e| e.to_str()) != Some("dream") {
                continue;
            }
            let dream = parse_dream(entry.path(), &canonical)?;
            if !seen_hashes.insert(dream.sha256.clone()) {
                warnings.push(format!(
                    "duplicate dream content skipped: {} (same as an earlier file)",
                    dream.path
                ));
                continue;
            }
            if let Some(first) = seen_ids.get(dream.id.0.as_str()) {
                return Err(ParseError::Schema {
                    path: dream.path.clone(),
                    message: format!("duplicate id '{}' (already used by {first})", dream.id.0),
                });
            }
            seen_ids.insert(dream.id.0.clone(), dream.path.clone());
            tracing::debug!(path = %dream.path, id = %dream.id.0, "parsed dream");
            dreams.push(dream);
        }
    }
    on_progress(stats.files_scanned);
    tracing::info!(
        dreams = dreams.len(),
        files_scanned = stats.files_scanned,
        "discovery complete"
    );
    Ok((dreams, stats, warnings))
}

/// Discover without a progress callback.
pub fn discover(
    roots: &[PathBuf],
) -> Result<(Vec<Dream>, DiscoveryStats, Vec<String>), ParseError> {
    discover_with_progress(roots, |_| {})
}

/// Convenience: discover under a single root.
pub fn discover_one(root: &Path) -> Result<(Vec<Dream>, DiscoveryStats, Vec<String>), ParseError> {
    discover(&[root.to_path_buf()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_and_skips() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.dream"), "[intent]\nstatement = \"a\"\n").unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        std::fs::write(
            tmp.path().join(".git/b.dream"),
            "[intent]\nstatement = \"b\"\n",
        )
        .unwrap();
        std::fs::create_dir(tmp.path().join(".cache")).unwrap();
        std::fs::write(
            tmp.path().join(".cache/c.dream"),
            "[intent]\nstatement = \"c\"\n",
        )
        .unwrap();
        std::fs::create_dir(tmp.path().join("target")).unwrap();
        std::fs::write(
            tmp.path().join("target/d.dream"),
            "[intent]\nstatement = \"d\"\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join("e.txt"), "not a dream").unwrap();
        let (dreams, stats, _) = discover_one(tmp.path()).unwrap();
        assert_eq!(dreams.len(), 1);
        assert_eq!(dreams[0].intent.statement, "a");
        assert!(stats.files_scanned >= 2);
    }

    #[test]
    fn explicit_hidden_root_is_entered() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join(".dreams")).unwrap();
        std::fs::write(
            tmp.path().join(".dreams/x.dream"),
            "[intent]\nstatement = \"x\"\n",
        )
        .unwrap();
        let hidden = tmp.path().join(".dreams");
        let (dreams, _, _) = discover_one(&hidden).unwrap();
        assert_eq!(dreams.len(), 1);
    }

    #[test]
    fn progress_callback_fires() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.dream"), "[intent]\nstatement = \"a\"\n").unwrap();
        let mut calls = 0;
        discover_with_progress(&[tmp.path().to_path_buf()], |_| calls += 1).unwrap();
        assert!(calls >= 1);
    }
}
