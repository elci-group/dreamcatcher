// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! YAML contract rendering (deterministic: sorted keys via BTree structures).

use serde::{Deserialize, Serialize};
use serde_yaml::Mapping;

use crate::analyze::Candidate;
use crate::emit::provenance_for;
use crate::ir::DreamIr;

/// The `.deliver` contract document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    /// Compiled objective.
    pub objective: String,
    /// Subjects in scope.
    pub scope: Vec<String>,
    /// Capabilities required to execute.
    pub required_capabilities: Vec<String>,
    /// Constraints on execution.
    pub constraints: Vec<String>,
    /// Acceptance checks.
    pub acceptance: Vec<String>,
    /// Priority.
    pub priority: String,
    /// Risk level.
    pub risk: Option<String>,
    /// Other deliverables this depends on.
    pub dependencies: Vec<String>,
    /// Evidence carried through from dreams.
    pub evidence: Vec<String>,
    /// Decisions a human must make.
    pub required_human_decisions: Vec<String>,
    /// Executor.
    pub execution: String,
    /// Traceability back to `.dream` sources.
    pub provenance: Vec<crate::emit::Provenance>,
}

fn seq(values: &[String]) -> serde_yaml::Value {
    serde_yaml::Value::Sequence(
        values
            .iter()
            .map(|v| serde_yaml::Value::String(v.clone()))
            .collect(),
    )
}

/// Render the contract as a YAML string with a leading comment header.
pub fn contract_yaml(candidate: &Candidate, ir: &DreamIr) -> anyhow::Result<String> {
    let contract = Contract {
        objective: candidate.objective.clone(),
        scope: candidate.scope.clone(),
        required_capabilities: candidate.required_capabilities.clone(),
        constraints: candidate.constraints.clone(),
        acceptance: candidate.acceptance.clone(),
        priority: format!("{:?}", candidate.priority).to_lowercase(),
        risk: candidate.risk.clone(),
        dependencies: candidate.dependencies.clone(),
        evidence: candidate.evidence.clone(),
        required_human_decisions: candidate.required_human_decisions.clone(),
        execution: "Dreamboat".into(),
        provenance: provenance_for(candidate, ir),
    };

    // Hand-build a Mapping to control field order (readable header first).
    let mut m = Mapping::new();
    let insert = |m: &mut Mapping, k: &str, v: serde_yaml::Value| {
        m.insert(serde_yaml::Value::String(k.into()), v);
    };
    insert(
        &mut m,
        "objective",
        serde_yaml::Value::String(contract.objective.clone()),
    );
    insert(&mut m, "scope", seq(&contract.scope));
    insert(
        &mut m,
        "required_capabilities",
        seq(&contract.required_capabilities),
    );
    insert(&mut m, "constraints", seq(&contract.constraints));
    insert(&mut m, "acceptance", seq(&contract.acceptance));
    insert(
        &mut m,
        "priority",
        serde_yaml::Value::String(contract.priority.clone()),
    );
    if let Some(r) = &contract.risk {
        insert(&mut m, "risk", serde_yaml::Value::String(r.clone()));
    }
    if !contract.dependencies.is_empty() {
        insert(&mut m, "dependencies", seq(&contract.dependencies));
    }
    if !contract.evidence.is_empty() {
        insert(&mut m, "evidence", seq(&contract.evidence));
    }
    if !contract.required_human_decisions.is_empty() {
        insert(
            &mut m,
            "required_human_decisions",
            seq(&contract.required_human_decisions),
        );
    }
    insert(
        &mut m,
        "execution",
        serde_yaml::Value::String("Dreamboat".into()),
    );

    let mut prov = serde_yaml::Sequence::new();
    for p in &contract.provenance {
        let mut pm = Mapping::new();
        pm.insert("id".into(), p.id.clone().into());
        pm.insert("path".into(), p.path.clone().into());
        pm.insert("sha256".into(), p.sha256.clone().into());
        prov.push(serde_yaml::Value::Mapping(pm));
    }
    insert(&mut m, "provenance", serde_yaml::Value::Sequence(prov));

    let body = serde_yaml::to_string(&serde_yaml::Value::Mapping(m))?;
    Ok(format!(
        "# .deliver contract compiled by dreamcatcher {}\n# deliverable: {} ({} source dream(s))\n{}",
        crate::VERSION,
        candidate.id,
        candidate.source_dreams.len(),
        body
    ))
}
