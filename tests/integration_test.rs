// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! End-to-end tests using the compiled binary in tempdir sandboxes.

use std::path::Path;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dreamcatcher"))
}

fn write_dream(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(format!("{name}.dream")), body).unwrap();
}

fn run_in(dir: &Path, args: &[&str]) -> (bool, String) {
    let out = bin()
        .current_dir(dir)
        .args(args)
        .output()
        .expect("run dreamcatcher");
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

/// Like `run_in` but keeps stdout and stderr separate (stdout carries the
/// JSON report; stderr carries phase telemetry).
fn run_split(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let out = bin()
        .current_dir(dir)
        .args(args)
        .output()
        .expect("run dreamcatcher");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

#[test]
fn single_dream_produces_one_deliverable() {
    let tmp = tempfile::tempdir().unwrap();
    write_dream(
        tmp.path(),
        "forward-compat",
        "[intent]\nstatement = \"old files keep working\"\n[subject]\nname = \"kaptaind\"\n",
    );
    let (ok, out) = run_in(tmp.path(), &["--out", "deliverables"]);
    assert!(ok, "{out}");
    let manifest = std::fs::read_to_string(tmp.path().join("deliverables/manifest.json")).unwrap();
    assert!(manifest.contains("\"deliverables\""));
    // Sidecar carries the subject scope as dreamboat repositories.
    let sidecar: Vec<_> = std::fs::read_dir(tmp.path().join("deliverables"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".deliver.json"))
        .collect();
    assert_eq!(sidecar.len(), 1);
    let side = std::fs::read_to_string(&sidecar[0]).unwrap();
    assert!(side.contains("\"kaptaind\""), "{side}");
    // YAML contract with provenance
    let d: Vec<_> = std::fs::read_dir(tmp.path().join("deliverables"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert!(d.iter().any(|p| p.to_string_lossy().ends_with(".deliver")));
}

#[test]
fn convergent_dreams_merge_with_provenance() {
    let tmp = tempfile::tempdir().unwrap();
    for n in ["a", "b", "c"] {
        write_dream(
            tmp.path(),
            n,
            &format!(
                "[intent]\nstatement = \"kaptaind schema versioning should be deterministic\"\n\
                 [subject]\nname = \"kaptaind\"\ntags = [\"schema\"]\n[evidence]\nitems = [\"e{n}\"]\n"
            ),
        );
    }
    let (ok, out) = run_in(tmp.path(), &["--out", "deliverables", "--format", "json"]);
    assert!(ok, "{out}");
    let manifest = std::fs::read_to_string(tmp.path().join("deliverables/manifest.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    let dels = v["deliverables"].as_array().unwrap();
    assert_eq!(dels.len(), 1, "three same-subject dreams should merge");
    assert_eq!(dels[0]["source_dreams"].as_array().unwrap().len(), 3);
}

#[test]
fn conflict_emits_human_decision() {
    let tmp = tempfile::tempdir().unwrap();
    write_dream(
        tmp.path(),
        "want",
        "[intent]\nstatement = \"rewrite the config loader\"\nkind = \"proposal\"\n[subject]\nname = \"kaptaind\"\n",
    );
    write_dream(
        tmp.path(),
        "worry",
        "[intent]\nstatement = \"rewrites risk data loss\"\nkind = \"concern\"\n[subject]\nname = \"kaptaind\"\n",
    );
    let (ok, out) = run_in(tmp.path(), &["--out", "deliverables", "--format", "json"]);
    assert!(ok, "{out}");
    let manifest = std::fs::read_to_string(tmp.path().join("deliverables/manifest.json")).unwrap();
    assert!(manifest.contains("conflict"), "{manifest}");
    assert!(
        manifest.contains("needs_human_decision\": true"),
        "{manifest}"
    );
    // Every manifest entry must have its own unique files (no slug collisions).
    let v: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    let mut slugs: Vec<String> = v["deliverables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["slug"].as_str().unwrap().to_string())
        .collect();
    let total = slugs.len();
    slugs.sort();
    slugs.dedup();
    assert_eq!(slugs.len(), total, "slugs must be unique: {manifest}");
    for s in &slugs {
        assert!(
            tmp.path()
                .join(format!("deliverables/{s}.deliver"))
                .exists(),
            "missing deliverables/{s}.deliver"
        );
    }
}

#[test]
fn missing_dependency_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    write_dream(
        tmp.path(),
        "a",
        "[intent]\nstatement = \"x\"\n[meta]\ndepends_on = [\"dream-ghost\"]\n",
    );
    let (ok, out) = run_in(tmp.path(), &["--out", "deliverables"]);
    assert!(!ok, "{out}");
    assert!(out.contains("unknown dream"), "{out}");
}

#[test]
fn cycle_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    write_dream(
        tmp.path(),
        "a",
        "id = \"a\"\n[intent]\nstatement = \"x\"\n[meta]\ndepends_on = [\"b\"]\n",
    );
    write_dream(
        tmp.path(),
        "b",
        "id = \"b\"\n[intent]\nstatement = \"y\"\n[meta]\ndepends_on = [\"a\"]\n",
    );
    let (ok, out) = run_in(tmp.path(), &["--out", "deliverables"]);
    assert!(!ok, "{out}");
    assert!(out.contains("cycle"), "{out}");
}

#[test]
fn validate_catches_malformed_toml() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("bad.dream"), "not [valid toml").unwrap();
    let (ok, out) = run_in(tmp.path(), &["validate"]);
    assert!(!ok, "{out}");
}

#[test]
fn output_is_deterministic() {
    let tmp = tempfile::tempdir().unwrap();
    write_dream(
        tmp.path(),
        "one",
        "[intent]\nstatement = \"deterministic schema migration\"\n[subject]\nname = \"kaptaind\"\n",
    );
    let (ok, out) = run_in(tmp.path(), &["--out", "d1"]);
    assert!(ok, "{out}");
    let (ok, out) = run_in(tmp.path(), &["--out", "d2"]);
    assert!(ok, "{out}");
    for entry in std::fs::read_dir(tmp.path().join("d1")).unwrap() {
        let p = entry.unwrap().path();
        if p.file_name().unwrap() == "manifest.json" {
            continue; // contains a timestamp
        }
        let q = tmp.path().join("d2").join(p.file_name().unwrap());
        assert_eq!(
            std::fs::read(&p).unwrap(),
            std::fs::read(&q).unwrap(),
            "{} must be byte-identical across runs",
            p.display()
        );
    }
}

#[test]
fn text_output_is_emoji_led_and_telemetrised() {
    let tmp = tempfile::tempdir().unwrap();
    write_dream(
        tmp.path(),
        "one",
        "[intent]\nstatement = \"make it pretty\"\n[subject]\nname = \"kaptaind\"\n",
    );
    let (ok, out) = run_in(tmp.path(), &["--out", "deliverables"]);
    assert!(ok, "{out}");
    assert!(out.contains('🌙'), "missing brand banner: {out}");
    assert!(out.contains('🎯'), "missing deliverable section: {out}");
    assert!(out.contains('🔍'), "missing scan telemetry: {out}");
    assert!(out.contains("files"), "missing files-scanned count: {out}");
    assert!(out.contains("total"), "missing timings: {out}");
}

#[test]
fn json_report_carries_telemetry() {
    let tmp = tempfile::tempdir().unwrap();
    write_dream(
        tmp.path(),
        "one",
        "[intent]\nstatement = \"telemetry please\"\n[subject]\nname = \"kaptaind\"\n",
    );
    let (ok, out, err) = run_split(tmp.path(), &["--out", "deliverables", "--format", "json"]);
    assert!(ok, "stdout: {out}\nstderr: {err}");
    let v: serde_json::Value = serde_json::from_str(out.trim())
        .unwrap_or_else(|e| panic!("stdout not pure JSON ({e}): {out}\n--- stderr: {err}"));
    assert!(v["timings"]["total_ms"].is_u64());
    assert!(v["files_scanned"].is_u64());
    assert!(v["roots"].as_array().is_some());
    assert!(err.contains("🔍 discovery"), "phase telemetry missing: {err}");
}

#[test]
fn hidden_directories_are_not_scanned() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".rustup-huge")).unwrap();
    std::fs::write(
        tmp.path().join(".rustup-huge/trap.dream"),
        "[intent]\nstatement = \"should never be seen\"\n",
    )
    .unwrap();
    let (ok, out, err) = run_split(tmp.path(), &["--out", "deliverables", "--format", "json"]);
    assert!(ok, "stdout: {out}\nstderr: {err}");
    let v: serde_json::Value = serde_json::from_str(out.trim())
        .unwrap_or_else(|e| panic!("stdout not pure JSON ({e}): {out}"));
    assert_eq!(
        v["dreams"].as_u64().unwrap(),
        0,
        "hidden dir leaked into discovery"
    );
}

#[test]
fn validate_output_is_styled() {
    let tmp = tempfile::tempdir().unwrap();
    write_dream(
        tmp.path(),
        "ok-one",
        "[intent]\nstatement = \"fine\"\n",
    );
    let (ok, out) = run_in(tmp.path(), &["validate"]);
    assert!(ok, "{out}");
    assert!(out.contains('✅'), "missing per-dream check mark: {out}");
    assert!(out.contains('🌙'), "missing summary banner: {out}");
    assert!(out.contains("ok-one"), "{out}");
}
