use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LlmProviderKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub key_name: String,
    #[serde(skip_serializing)]
    pub encrypted_key: Vec<u8>,
    pub status: Option<String>,
    pub last_validated_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct LlmProviderKeyInfo {
    pub id: Uuid,
    pub provider: String,
    pub key_name: String,
    pub status: Option<String>,
    pub last_validated_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

impl From<LlmProviderKey> for LlmProviderKeyInfo {
    fn from(k: LlmProviderKey) -> Self {
        Self {
            id: k.id,
            provider: k.provider,
            key_name: k.key_name,
            status: k.status,
            last_validated_at: k.last_validated_at,
            created_at: k.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AddProviderKeyRequest {
    pub provider: String,
    pub key_name: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Conversation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub forked_from: Option<Uuid>,
    pub fork_point_message_id: Option<Uuid>,
    pub inject_context: Option<bool>,
    pub context_token_budget: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConversationMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub provider: String,
    pub model: String,
    pub title: Option<String>,
    pub inject_context: Option<bool>,
    pub context_token_budget: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub provider: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompareRequest {
    pub message: String,
    pub targets: Vec<CompareTarget>,
    pub conversation_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CompareTarget {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct ForkRequest {
    pub message_id: Uuid,
    pub new_provider: String,
    pub new_model: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub message: ConversationMessage,
    pub usage: TokenUsage,
}

#[derive(Debug, Serialize, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LlmUsageRecord {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub conversation_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub cost_usd: Option<rust_decimal::Decimal>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct UsageSummary {
    pub total_tokens: i64,
    pub by_provider: Vec<ProviderUsage>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProviderUsage {
    pub provider: String,
    pub model: String,
    pub total_tokens: Option<i64>,
    pub request_count: Option<i64>,
}
