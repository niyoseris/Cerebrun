use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub permissions: serde_json::Value,
    pub status: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub name: String,
    pub permissions: serde_json::Value,
    pub status: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

impl From<ApiKey> for ApiKeyInfo {
    fn from(k: ApiKey) -> Self {
        Self {
            id: k.id,
            name: k.name,
            permissions: k.permissions,
            status: k.status,
            last_used_at: k.last_used_at,
            created_at: k.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub key: String,
}
