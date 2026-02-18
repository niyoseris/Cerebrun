use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Layer0Public {
    pub user_id: Uuid,
    pub language: Option<String>,
    pub timezone: Option<String>,
    pub output_format: Option<serde_json::Value>,
    pub blocked_topics: Option<serde_json::Value>,
    pub communication_style: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer0Update {
    pub language: Option<String>,
    pub timezone: Option<String>,
    pub output_format: Option<serde_json::Value>,
    pub blocked_topics: Option<serde_json::Value>,
    pub communication_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Layer1Context {
    pub user_id: Uuid,
    pub active_projects: Option<serde_json::Value>,
    pub recent_conversations: Option<serde_json::Value>,
    pub working_directories: Option<serde_json::Value>,
    pub current_goals: Option<serde_json::Value>,
    pub pinned_memories: Option<serde_json::Value>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer1Update {
    pub active_projects: Option<serde_json::Value>,
    pub recent_conversations: Option<serde_json::Value>,
    pub working_directories: Option<serde_json::Value>,
    pub current_goals: Option<serde_json::Value>,
    pub pinned_memories: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Layer2Personal {
    pub user_id: Uuid,
    pub display_name: Option<String>,
    pub location: Option<String>,
    pub interests: Option<serde_json::Value>,
    pub contact_preferences: Option<serde_json::Value>,
    pub relationship_notes: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer2Update {
    pub display_name: Option<String>,
    pub location: Option<String>,
    pub interests: Option<serde_json::Value>,
    pub contact_preferences: Option<serde_json::Value>,
    pub relationship_notes: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Layer3Vault {
    pub user_id: Uuid,
    pub encryption_key_hash: String,
    pub encrypted_data: Vec<u8>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VaultConsentRequest {
    pub id: Uuid,
    pub user_id: Uuid,
    pub api_key_id: Uuid,
    pub reason: String,
    pub requested_data: serde_json::Value,
    pub status: Option<String>,
    pub vault_token_hash: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub action: String,
    pub layer: Option<String>,
    pub granted: bool,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct VaultAccessRequest {
    pub reason: String,
    pub requested_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct VaultApproveRequest {
    pub request_id: Uuid,
    pub expires_in_hours: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct VaultDenyRequest {
    pub request_id: Uuid,
}
