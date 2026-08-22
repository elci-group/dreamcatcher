// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Dreamboat-compatible JSON sidecar emission.
//!
//! Maps a candidate onto dreamboat's `JourneyPayload`/`TechnicalDirective`
//! model (see /home/sal/dreamsequence/dreamboat/src/model.rs). Unsigned in
//! v0.1; dreamboat's HMAC envelope is a documented TODO for the consumer.

use serde_json::{json, Value};

use crate::analyze::Candidate;
use crate::emit::provenance_for;
use crate::ir::DreamIr;

/// Build the unsigned journey payload value for a candidate.
pub fn sidecar(candidate: &Candidate, ir: &DreamIr) -> Value {
    let dream = ir.nodes.get(
        candidate
            .source_dreams
            .first()
            .map(|s| s.as_str())
            .unwrap_or_default(),
    );
    let repositories: Vec<String> = candidate.scope.iter().map(|s| s.to_lowercase()).collect();
    let problem_statement = if candidate.evidence.is_empty() {
        "No evidence recorded; compiled from dream intent.".to_string()
    } else {
        candidate.evidence.join("\n")
    };
    let proposed_solution = candidate.objective.clone();
    let desired_outcome = candidate
        .acceptance
        .iter()
        .map(|a| format!("- {a}"))
        .collect::<Vec<_>>()
        .join("\n");

    let directive = json!({
        "title": format!("{} ({})", candidate.slug, candidate.id),
        "problem_statement": problem_statement,
        "evidence": candidate.evidence,
        "proposed_solution": proposed_solution,
        "desired_outcome": desired_outcome,
        "repositories": repositories,
        "implementation_plan": format!(
            "Execute via Dreamboat. Required capabilities: {}",
            if candidate.required_capabilities.is_empty() {
                "none specified".into()
            } else {
                candidate.required_capabilities.join(", ")
            }
        ),
        "acceptance_criteria": candidate.acceptance,
        "constraints": candidate.constraints,
        "risk": candidate.risk.clone().unwrap_or_else(|| "low".into()),
        "execution_mode": "plan",
        "toolchain": {},
        "graph_context": {
            "deliverable_id": candidate.id,
            "source_dreams": candidate.source_dreams,
            "dependencies": candidate.dependencies,
            "needs_human_decision": candidate.is_conflict || !candidate.required_human_decisions.is_empty(),
        },
        "operator_guidance": if candidate.required_human_decisions.is_empty() {
            Value::Null
        } else {
            json!(candidate.required_human_decisions)
        },
    });

    json!({
        "schema_version": 1,
        "kind": "dreamboat.journey",
        "compiled_by": format!("dreamcatcher {}", crate::VERSION),
        "job_id": format!("{}-{}", candidate.id, candidate.slug),
        "directive_id": candidate.source_dreams.first().cloned()
            .or_else(|| dream.map(|d| d.id.0.clone()))
            .unwrap_or_else(|| candidate.id.clone()),
        "directive_revision": 1,
        "workspace": ".",
        "priority": format!("{:?}", candidate.priority).to_lowercase(),
        "signature": Value::Null,
        "directive": directive,
        "provenance": provenance_for(candidate, ir),
    })
}
