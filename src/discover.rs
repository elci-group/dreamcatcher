// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Discovery of `.dream` files under one or more roots.

use std::path::{Path, PathBuf};

use crate::dream::{parse_dream, Dream, ParseError};

/// Directories never descended into.
const SKIP_DIRS: [&str; 4] = [".git", "target", "node_modules", ".dream-catcher"];

/// Find and parse every `*.dream` file under `roots`.
///
/// Duplicate content (same sha256) is reported but only the first occurrence
/// is kept, deterministically (roots are walked in the given order).
pub fn discover(roots: &[PathBuf]) -> Result<(Vec<Dream>, Vec<String>), ParseError> {
    let mut dreams = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_hashes = std::collections::HashSet::new();
    let mut seen_ids = std::collections::HashMap::new();

    for root in roots {
        let root = if root.is_absolute() {
            root.clone()
        } else {
            std::env::current_dir().unwrap_or_default().join(root)
        };
        let canonical = root.canonicalize().unwrap_or_else(|_| root.clone());
        for entry in walkdir::WalkDir::new(&root)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                !e.file_type().is_dir()
                    || !SKIP_DIRS.contains(&e.file_name().to_string_lossy().as_ref())
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warnings.push(format!("walk error under {}: {e}", root.display()));
                    continue;
                }
            };
            if !entry.file_type().is_file()
                || entry.path().extension().and_then(|e| e.to_str()) != Some("dream")
            {
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
            dreams.push(dream);
        }
    }
    Ok((dreams, warnings))
}

/// Convenience: discover under a single root.
pub fn discover_one(root: &Path) -> Result<(Vec<Dream>, Vec<String>), ParseError> {
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
        std::fs::write(tmp.path().join("c.txt"), "not a dream").unwrap();
        let (dreams, _) = discover_one(tmp.path()).unwrap();
        assert_eq!(dreams.len(), 1);
        assert_eq!(dreams[0].intent.statement, "a");
    }
}
