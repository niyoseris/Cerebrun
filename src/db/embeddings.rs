use sqlx::PgPool;
use uuid::Uuid;

pub async fn upsert_context_embedding(
    pool: &PgPool,
    user_id: Uuid,
    source_type: &str,
    source_key: &str,
    content_text: &str,
    embedding: &[f32],
) -> Result<(), sqlx::Error> {
    let embedding_str = format!(
        "[{}]",
        embedding.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",")
    );

    sqlx::query(
        r#"
        INSERT INTO context_embeddings (user_id, source_type, source_key, content_text, embedding, updated_at)
        VALUES ($1, $2, $3, $4, $5::vector, NOW())
        ON CONFLICT (user_id, source_type, source_key)
        DO UPDATE SET content_text = $4, embedding = $5::vector, updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(source_type)
    .bind(source_key)
    .bind(content_text)
    .bind(&embedding_str)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_knowledge_embedding(
    pool: &PgPool,
    knowledge_id: Uuid,
    embedding: &[f32],
) -> Result<(), sqlx::Error> {
    let embedding_str = format!(
        "[{}]",
        embedding.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",")
    );

    sqlx::query(
        "UPDATE knowledge_entries SET embedding = $1::vector WHERE id = $2",
    )
    .bind(&embedding_str)
    .bind(knowledge_id)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct SimilarContextResult {
    pub source_type: String,
    pub source_key: String,
    pub content_text: String,
    pub similarity: Option<f64>,
}

pub async fn search_similar_context(
    pool: &PgPool,
    user_id: Uuid,
    query_embedding: &[f32],
    limit: i64,
    min_similarity: f64,
) -> Result<Vec<SimilarContextResult>, sqlx::Error> {
    let embedding_str = format!(
        "[{}]",
        query_embedding.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",")
    );

    sqlx::query_as::<_, SimilarContextResult>(
        r#"
        SELECT source_type, source_key, content_text,
               1 - (embedding <=> $1::vector) as similarity
        FROM context_embeddings
        WHERE user_id = $2
          AND embedding IS NOT NULL
          AND 1 - (embedding <=> $1::vector) >= $3
        ORDER BY embedding <=> $1::vector
        LIMIT $4
        "#,
    )
    .bind(&embedding_str)
    .bind(user_id)
    .bind(min_similarity)
    .bind(limit)
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct SimilarKnowledgeResult {
    pub id: Uuid,
    pub content: String,
    pub summary: Option<String>,
    pub category: String,
    pub tags: Option<Vec<String>>,
    pub source_project: Option<String>,
    pub similarity: Option<f64>,
}

pub async fn search_similar_knowledge(
    pool: &PgPool,
    user_id: Uuid,
    query_embedding: &[f32],
    limit: i64,
    min_similarity: f64,
) -> Result<Vec<SimilarKnowledgeResult>, sqlx::Error> {
    let embedding_str = format!(
        "[{}]",
        query_embedding.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",")
    );

    sqlx::query_as::<_, SimilarKnowledgeResult>(
        r#"
        SELECT id, content, summary, category, tags, source_project,
               1 - (embedding <=> $1::vector) as similarity
        FROM knowledge_entries
        WHERE user_id = $2
          AND embedding IS NOT NULL
          AND 1 - (embedding <=> $1::vector) >= $3
        ORDER BY embedding <=> $1::vector
        LIMIT $4
        "#,
    )
    .bind(&embedding_str)
    .bind(user_id)
    .bind(min_similarity)
    .bind(limit)
    .fetch_all(pool)
    .await
}
