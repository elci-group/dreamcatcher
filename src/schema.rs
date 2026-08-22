// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Hand-written JSON Schema (draft-07) for the `.dream` format.

/// The schema document value.
pub fn dream_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": ".dream",
        "description": "A dream: a possibility expressed for dreamcatcher to compile.",
        "type": "object",
        "required": ["intent"],
        "properties": {
            "id": { "type": "string", "description": "Stable id; derived from filename+hash when absent" },
            "title": { "type": "string" },
            "created": { "type": "string", "format": "date" },
            "intent": {
                "type": "object",
                "required": ["statement"],
                "properties": {
                    "statement": { "type": "string", "minLength": 1 },
                    "kind": { "enum": ["opportunity", "desire", "observation", "proposal", "concern", "change"] }
                }
            },
            "subject": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "tags": { "type": "array", "items": { "type": "string" } }
                }
            },
            "evidence": {
                "type": "object",
                "properties": {
                    "items": { "type": "array", "items": { "type": "string" } }
                }
            },
            "meta": {
                "type": "object",
                "properties": {
                    "priority_hint": { "enum": ["low", "medium", "high", "critical"] },
                    "effort_hint": { "enum": ["small", "medium", "large"] },
                    "risk_hint": { "enum": ["low", "medium", "high"] },
                    "depends_on": { "type": "array", "items": { "type": "string" } },
                    "conflicts_with": { "type": "array", "items": { "type": "string" } },
                    "supersedes": { "type": "array", "items": { "type": "string" } },
                    "requires_capabilities": { "type": "array", "items": { "type": "string" } },
                    "open_questions": { "type": "array", "items": { "type": "string" } }
                }
            }
        }
    })
}
