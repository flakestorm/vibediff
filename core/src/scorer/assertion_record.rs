use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionRecord {
    pub assertion_id: Uuid,
    pub model: String,
    pub commit_hash: String,
    pub timestamp: DateTime<Utc>,
    pub scores: DimensionScores,
    pub composite_score: f64,
    pub label: VibeLabel,
    pub reasoning: DimensionReasoning,
    pub flagged_entities: Vec<FlaggedEntity>,
    pub suggested_commit_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScores {
    pub logic_match: f64,
    pub scope_adherence: f64,
    pub side_effect_detection: f64,
    pub structural_proportionality: f64,
}

impl DimensionScores {
    pub fn composite(&self) -> f64 {
        (self.logic_match * 0.35)
            + (self.scope_adherence * 0.30)
            + (self.side_effect_detection * 0.20)
            + (self.structural_proportionality * 0.15)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionReasoning {
    pub logic_match: String,
    pub scope_adherence: String,
    pub side_effect_detection: String,
    pub structural_proportionality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlaggedEntity {
    pub entity_name: String,
    pub concern: ConcernType,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConcernType {
    ScopeViolation,
    UndocumentedSideEffect,
    DisproportionateChange,
    LogicMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VibeLabel {
    Aligned,
    Drifting,
    Suspect,
    Misaligned,
}

impl VibeLabel {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.85 {
            Self::Aligned
        } else if score >= 0.70 {
            Self::Drifting
        } else if score >= 0.50 {
            Self::Suspect
        } else {
            Self::Misaligned
        }
    }
}
