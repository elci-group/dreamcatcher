// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Data model for a single `.dream` file.

use serde::{Deserialize, Serialize};

/// Stable identifier for a dream. Derived from the filename hash when the
/// file does not declare an `id`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DreamId(pub String);

impl std::fmt::Display for DreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What kind of possibility the dream expresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntentKind {
    /// An opportunity spotted in reality.
    Opportunity,
    /// A want.
    Desire,
    /// Something noticed.
    Observation,
    /// A concrete suggestion.
    Proposal,
    /// A worry.
    Concern,
    /// An intended change.
    Change,
}

impl IntentKind {
    /// Kinds that structurally oppose each other on the same subject.
    pub fn contradicts(self, other: Self) -> bool {
        use IntentKind::*;
        matches!(
            (self, other),
            (Proposal, Concern) | (Concern, Proposal) | (Change, Concern) | (Concern, Change)
        )
    }
}

/// Priority hint from the dream author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// Low.
    Low,
    /// Medium.
    Medium,
    /// High.
    High,
    /// Critical.
    Critical,
}

impl Priority {
    /// Numeric weight used by the scoring pass.
    pub fn weight(self) -> u32 {
        match self {
            Priority::Low => 1,
            Priority::Medium => 2,
            Priority::High => 3,
            Priority::Critical => 4,
        }
    }
}

/// Effort hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    /// Small.
    Small,
    /// Medium.
    Medium,
    /// Large.
    Large,
}

impl Effort {
    /// Numeric cost used by the scoring pass.
    pub fn cost(self) -> u32 {
        match self {
            Effort::Small => 1,
            Effort::Medium => 2,
            Effort::Large => 3,
        }
    }
}

/// Risk hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    /// Low.
    Low,
    /// Medium.
    Medium,
    /// High.
    High,
}

/// What is being sought.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Intent {
    /// One-sentence statement of the dream. Required.
    pub statement: String,
    /// Kind of possibility. Optional.
    pub kind: Option<IntentKind>,
}

/// What the dream is about.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Subject {
    /// Project, repository, tool, person, system or process name.
    #[serde(default)]
    pub name: Option<String>,
    /// Free-form tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Why the dream exists.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Evidence {
    /// Observed facts motivating the dream.
    #[serde(default)]
    pub items: Vec<String>,
}

/// Optional metadata and relationships.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Meta {
    /// Author priority hint.
    pub priority_hint: Option<Priority>,
    /// Author effort hint.
    pub effort_hint: Option<Effort>,
    /// Author risk hint.
    pub risk_hint: Option<Risk>,
    /// Ids of dreams this dream depends on.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Ids of dreams this dream conflicts with.
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    /// Ids of dreams this dream supersedes.
    #[serde(default)]
    pub supersedes: Vec<String>,
    /// Capabilities required to act on this dream.
    #[serde(default)]
    pub requires_capabilities: Vec<String>,
    /// Unresolved questions; each becomes a required human decision.
    #[serde(default)]
    pub open_questions: Vec<String>,
}

/// A parsed `.dream` file with provenance attached.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dream {
    /// Stable id.
    pub id: DreamId,
    /// Short human title.
    #[serde(default)]
    pub title: Option<String>,
    /// Creation date (YYYY-MM-DD) if declared.
    pub created: Option<chrono::NaiveDate>,
    /// The intent.
    pub intent: Intent,
    /// The subject.
    #[serde(default)]
    pub subject: Subject,
    /// The evidence.
    #[serde(default)]
    pub evidence: Evidence,
    /// Optional metadata and relationships.
    #[serde(default)]
    pub meta: Meta,
    /// Provenance: path relative to the discovery root.
    pub path: String,
    /// Provenance: sha256 of file contents.
    pub sha256: String,
}
