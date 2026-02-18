use sqlx::PgPool;
use uuid::Uuid;

use crate::models::AuditLogEntry;

pub async fn log_access(
    pool: &PgPool,
    user_id: Uuid,
    api_key_id: Option<Uuid>,
    action: &str,
    layer: Option<&str>,
    granted: bool,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    metadata: Option<&serde_json::Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_log (user_id, api_key_id, action, layer, granted, ip_address, user_agent, metadata)
        VALUES ($1, $2, $3, $4, $5, $6::inet, $7, COALESCE($8, '{}'::jsonb))
        "#,
    )
    .bind(user_id)
    .bind(api_key_id)
    .bind(action)
    .bind(layer)
    .bind(granted)
    .bind(ip_address)
    .bind(user_agent)
    .bind(metadata)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_audit_log(
    pool: &PgPool,
    user_id: Uuid,
    layer: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditLogEntry>, sqlx::Error> {
    if let Some(layer) = layer {
        sqlx::query_as::<_, AuditLogEntry>(
            r#"
            SELECT id, user_id, api_key_id, action, layer, granted,
                   host(ip_address)::text as ip_address, user_agent, metadata, created_at
            FROM audit_log
            WHERE user_id = $1 AND layer = $2
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(user_id)
        .bind(layer)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, AuditLogEntry>(
            r#"
            SELECT id, user_id, api_key_id, action, layer, granted,
                   host(ip_address)::text as ip_address, user_agent, metadata, created_at
            FROM audit_log
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(user_id)
        .bind(layer)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }
}
