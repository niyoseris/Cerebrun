use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SystemModel {
    pub id: Uuid,
    pub provider: String,
    pub model_name: String,
    pub display_name: Option<String>,
    pub is_active: bool,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct AddSystemModelRequest {
    pub provider: String,
    pub model_name: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SystemSetting {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: Option<DateTime<Utc>>,
}
