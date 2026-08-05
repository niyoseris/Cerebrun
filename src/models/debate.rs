use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Debate {
    pub id: Uuid,
    pub public_id: Uuid,
    pub user_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub topic: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub completion_criteria: Option<serde_json::Value>,
    pub consensus_reached: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub concluded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DebateParticipant {
    pub id: Uuid,
    pub debate_id: Uuid,
    pub provider: String,
    pub model: String,
    pub api_key_name: Option<String>,
    pub joined_at: Option<DateTime<Utc>>,
    pub criteria_agreed: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DebateMessage {
    pub id: Uuid,
    pub debate_id: Uuid,
    pub participant_id: Option<Uuid>,
    pub provider: String,
    pub model: String,
    pub round: i32,
    pub role: Option<String>,
    pub content: String,
    pub criteria_proposal: Option<serde_json::Value>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDebateRequest {
    pub topic: String,
    pub description: Option<String>,
    pub opening_message: String,
    pub completion_criteria: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct JoinDebateRequest {
    pub debate_id: String,
    pub message: String,
    pub role: Option<String>,
    pub criteria_agreement: Option<Vec<String>>, // criteria items this participant agrees with
    pub criteria_proposal: Option<Vec<String>>, // new criteria to propose
}

#[derive(Debug, Deserialize)]
pub struct ConcludeDebateRequest {
    pub debate_id: String,
    pub reason: Option<String>,
    pub force: Option<bool>, // if true, user forces conclusion
}