use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{KnowledgeCategory, KnowledgeEntry};

pub async fn insert_knowledge(
    pool: &PgPool,
    user_id: Uuid,
    content: &str,
    summary: Option<&str>,
    category: &str,
    subcategory: Option<&str>,
    tags: &[String],
    source_agent: Option<&str>,
    source_project: Option<&str>,
    raw_input: Option<&str>,
) -> Result<KnowledgeEntry, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        INSERT INTO knowledge_entries (user_id, content, summary, category, subcategory, tags, source_agent, source_project, raw_input)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(content)
    .bind(summary)
    .bind(category)
    .bind(subcategory)
    .bind(tags)
    .bind(source_agent)
    .bind(source_project)
    .bind(raw_input)
    .fetch_one(pool)
    .await
}

pub async fn query_knowledge(
    pool: &PgPool,
    user_id: Uuid,
    keyword: Option<&str>,
    category: Option<&str>,
    tag: Option<&str>,
    source_project: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<KnowledgeEntry>, sqlx::Error> {
    let mut conditions = vec!["user_id = $1".to_string()];
    let mut param_idx = 2;

    if keyword.is_some() {
        conditions.push(format!("(content ILIKE ${0} OR summary ILIKE ${0})", param_idx));
        param_idx += 1;
    }
    if category.is_some() {
        conditions.push(format!("category = ${}", param_idx));
        param_idx += 1;
    }
    if tag.is_some() {
        conditions.push(format!("${} = ANY(tags)", param_idx));
        param_idx += 1;
    }
    if source_project.is_some() {
        conditions.push(format!("source_project ILIKE ${}", param_idx));
        param_idx += 1;
    }

    let where_clause = conditions.join(" AND ");
    let query_str = format!(
        "SELECT * FROM knowledge_entries WHERE {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        where_clause, param_idx, param_idx + 1
    );

    let mut query = sqlx::query_as::<_, KnowledgeEntry>(&query_str).bind(user_id);

    if let Some(kw) = keyword {
        query = query.bind(format!("%{}%", kw));
    }
    if let Some(cat) = category {
        query = query.bind(cat);
    }
    if let Some(t) = tag {
        query = query.bind(t);
    }
    if let Some(sp) = source_project {
        query = query.bind(format!("%{}%", sp));
    }

    query = query.bind(limit).bind(offset);
    query.fetch_all(pool).await
}

pub async fn get_knowledge_by_id(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<KnowledgeEntry>, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        "SELECT * FROM knowledge_entries WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn delete_knowledge(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM knowledge_entries WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_categories(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<KnowledgeCategory>, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeCategory>(
        r#"
        SELECT category, COUNT(*) as count
        FROM knowledge_entries
        WHERE user_id = $1
        GROUP BY category
        ORDER BY count DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn count_knowledge(
    pool: &PgPool,
    user_id: Uuid,
    keyword: Option<&str>,
    category: Option<&str>,
    tag: Option<&str>,
    source_project: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let mut conditions = vec!["user_id = $1".to_string()];
    let mut param_idx = 2;

    if keyword.is_some() {
        conditions.push(format!("(content ILIKE ${0} OR summary ILIKE ${0})", param_idx));
        param_idx += 1;
    }
    if category.is_some() {
        conditions.push(format!("category = ${}", param_idx));
        param_idx += 1;
    }
    if tag.is_some() {
        conditions.push(format!("${} = ANY(tags)", param_idx));
        param_idx += 1;
    }
    if source_project.is_some() {
        conditions.push(format!("source_project ILIKE ${}", param_idx));
        // param_idx += 1;
    }

    let where_clause = conditions.join(" AND ");
    let query_str = format!("SELECT COUNT(*) FROM knowledge_entries WHERE {}", where_clause);

    let mut query = sqlx::query_scalar::<_, i64>(&query_str).bind(user_id);

    if let Some(kw) = keyword {
        query = query.bind(format!("%{}%", kw));
    }
    if let Some(cat) = category {
        query = query.bind(cat);
    }
    if let Some(t) = tag {
        query = query.bind(t);
    }
    if let Some(sp) = source_project {
        query = query.bind(format!("%{}%", sp));
    }

    query.fetch_one(pool).await
}
