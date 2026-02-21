use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct KnowledgeEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub summary: Option<String>,
    pub category: String,
    pub subcategory: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source_agent: Option<String>,
    pub source_project: Option<String>,
    pub raw_input: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct PushKnowledgeRequest {
    pub content: String,
    pub summary: Option<String>,
    pub category: Option<String>,
    pub subcategory: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source_agent: Option<String>,
    pub source_project: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeEntryInfo {
    pub id: Uuid,
    pub content: String,
    pub summary: Option<String>,
    pub category: String,
    pub subcategory: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source_agent: Option<String>,
    pub source_project: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<KnowledgeEntry> for KnowledgeEntryInfo {
    fn from(k: KnowledgeEntry) -> Self {
        Self {
            id: k.id,
            content: k.content,
            summary: k.summary,
            category: k.category,
            subcategory: k.subcategory,
            tags: k.tags,
            source_agent: k.source_agent,
            source_project: k.source_project,
            created_at: k.created_at,
            updated_at: k.updated_at,
        }
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct KnowledgeCategory {
    pub category: String,
    pub count: Option<i64>,
}
