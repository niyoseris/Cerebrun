use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContentVersion {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content_type: String,
    pub content_id: String,
    pub version_number: i32,
    pub content_snapshot: String,
    pub summary_snapshot: Option<String>,
    pub changed_by_provider: Option<String>,
    pub changed_by_model: Option<String>,
    pub changed_by_api_key: Option<String>,
    pub changed_by_user: Option<bool>,
    pub action: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteConfirmation {
    pub content_type: String,
    pub content_id: String,
    pub confirm: bool, // first confirmation
    pub force_purge: bool, // if true, also delete version history
}