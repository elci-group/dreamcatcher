// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Parser for `.dream` TOML files.

use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::model::{Dream, DreamId, Evidence, Intent, Meta, Subject};

/// A dream parse failure with file context.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The file could not be read.
    #[error("{path}: {source}")]
    Io {
        /// Path of the file.
        path: String,
        #[source]
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// The TOML or schema is invalid.
    #[error("{path}: {message}")]
    Schema {
        /// Path of the file.
        path: String,
        /// Human-readable description of the problem.
        message: String,
    },
}

/// Raw on-disk shape; the intent statement is required, everything else defaults.
#[derive(Debug, Deserialize)]
struct RawDream {
    id: Option<String>,
    title: Option<String>,
    created: Option<chrono::NaiveDate>,
    intent: Intent,
    #[serde(default)]
    subject: Subject,
    #[serde(default)]
    evidence: Evidence,
    #[serde(default)]
    meta: Meta,
}

/// Derive a stable id from the file stem and content hash, e.g. `dream-a1b2c3d4`.
fn derived_id(stem: &str, sha256: &str) -> DreamId {
    DreamId(format!("dream-{}-{}", slug::slugify(stem), &sha256[..8]))
}

mod slug {
    /// Minimal slugifier: lowercase, alphanumerics kept, runs of others -> '-'.
    pub fn slugify(s: &str) -> String {
        let mut out = String::new();
        let mut prev_dash = false;
        for ch in s.chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
                prev_dash = false;
            } else if !prev_dash && !out.is_empty() {
                out.push('-');
                prev_dash = true;
            }
        }
        while out.ends_with('-') {
            out.pop();
        }
        out
    }
}

/// Parse a `.dream` file from disk.
pub fn parse_dream(path: &Path, root: &Path) -> Result<Dream, ParseError> {
    let display = path.display().to_string();
    let bytes = std::fs::read(path).map_err(|source| ParseError::Io {
        path: display.clone(),
        source,
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let raw: RawDream = toml::from_str(&text).map_err(|e| ParseError::Schema {
        path: display.clone(),
        message: e.to_string(),
    })?;
    if raw.intent.statement.trim().is_empty() {
        return Err(ParseError::Schema {
            path: display,
            message: "intent.statement is required and must not be empty".into(),
        });
    }
    let sha256 = hex(&Sha256::digest(&bytes));
    let id = match raw.id {
        Some(id) if !id.trim().is_empty() => DreamId(id),
        _ => derived_id(
            path.file_stem().and_then(|s| s.to_str()).unwrap_or("dream"),
            &sha256,
        ),
    };
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string();
    Ok(Dream {
        id,
        title: raw.title,
        created: raw.created,
        intent: raw.intent,
        subject: raw.subject,
        evidence: raw.evidence,
        meta: raw.meta,
        path: rel,
        sha256,
    })
}

/// Lowercase hex encoding.
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(tmp: &Path, name: &str, body: &str) -> std::path::PathBuf {
        let p = tmp.join(name);
        std::fs::write(&p, body).unwrap();
        p
    }

    #[test]
    fn parses_minimal_dream() {
        let tmp = tempfile::tempdir().unwrap();
        let p = write(
            tmp.path(),
            "forward-compat.dream",
            "[intent]\nstatement = \"old files keep working\"\n",
        );
        let d = parse_dream(&p, tmp.path()).unwrap();
        assert_eq!(d.intent.statement, "old files keep working");
        assert!(d.id.0.starts_with("dream-forward-compat-"));
        assert_eq!(d.meta.depends_on.len(), 0);
    }

    #[test]
    fn explicit_id_wins() {
        let tmp = tempfile::tempdir().unwrap();
        let p = write(
            tmp.path(),
            "a.dream",
            "id = \"dream-001\"\n[intent]\nstatement = \"x\"\n",
        );
        let d = parse_dream(&p, tmp.path()).unwrap();
        assert_eq!(d.id.0, "dream-001");
    }

    #[test]
    fn missing_statement_is_error() {
        let tmp = tempfile::tempdir().unwrap();
        let p = write(tmp.path(), "bad.dream", "title = \"no intent\"\n");
        assert!(parse_dream(&p, tmp.path()).is_err());
    }
}
