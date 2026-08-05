use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ContentVersion;

/// Record a new version of content before it changes
pub async fn record_version(
    pool: &PgPool,
    user_id: Uuid,
    content_type: &str,
    content_id: &str,
    content_snapshot: &str,
    summary_snapshot: Option<&str>,
    changed_by_provider: Option<&str>,
    changed_by_model: Option<&str>,
    changed_by_api_key: Option<&str>,
    changed_by_user: bool,
    action: &str,
) -> Result<ContentVersion, sqlx::Error> {
    // Get next version number
    let next_version: (i32,) = sqlx::query_as(
        "SELECT COALESCE(MAX(version_number), 0) + 1 FROM content_versions WHERE user_id = $1 AND content_type = $2 AND content_id = $3",
    )
    .bind(user_id)
    .bind(content_type)
    .bind(content_id)
    .fetch_one(pool)
    .await?;

    sqlx::query_as::<_, ContentVersion>(
        r#"
        INSERT INTO content_versions (
            user_id, content_type, content_id, version_number,
            content_snapshot, summary_snapshot,
            changed_by_provider, changed_by_model, changed_by_api_key,
            changed_by_user, action
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(content_type)
    .bind(content_id)
    .bind(next_version.0)
    .bind(content_snapshot)
    .bind(summary_snapshot)
    .bind(changed_by_provider)
    .bind(changed_by_model)
    .bind(changed_by_api_key)
    .bind(changed_by_user)
    .bind(action)
    .fetch_one(pool)
    .await
}

/// Get version history for a specific content item
pub async fn get_versions(
    pool: &PgPool,
    user_id: Uuid,
    content_type: &str,
    content_id: &str,
    limit: i64,
) -> Result<Vec<ContentVersion>, sqlx::Error> {
    sqlx::query_as::<_, ContentVersion>(
        r#"
        SELECT * FROM content_versions
        WHERE user_id = $1 AND content_type = $2 AND content_id = $3
        ORDER BY version_number DESC LIMIT $4
        "#,
    )
    .bind(user_id)
    .bind(content_type)
    .bind(content_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Purge all version history for a content item (permanent deletion of history)
pub async fn purge_versions(
    pool: &PgPool,
    user_id: Uuid,
    content_type: &str,
    content_id: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM content_versions WHERE user_id = $1 AND content_type = $2 AND content_id = $3",
    )
    .bind(user_id)
    .bind(content_type)
    .bind(content_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() as i64)
}

/// Get a specific version
pub async fn get_version(
    pool: &PgPool,
    user_id: Uuid,
    content_type: &str,
    content_id: &str,
    version_number: i32,
) -> Result<Option<ContentVersion>, sqlx::Error> {
    sqlx::query_as::<_, ContentVersion>(
        r#"
        SELECT * FROM content_versions
        WHERE user_id = $1 AND content_type = $2 AND content_id = $3 AND version_number = $4
        "#,
    )
    .bind(user_id)
    .bind(content_type)
    .bind(content_id)
    .bind(version_number)
    .fetch_optional(pool)
    .await
}