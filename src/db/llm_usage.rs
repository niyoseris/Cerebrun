use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{LlmUsageRecord, ProviderUsage};

pub async fn record_usage(
    pool: &PgPool,
    user_id: Uuid,
    conversation_id: Option<Uuid>,
    message_id: Option<Uuid>,
    provider: &str,
    model: &str,
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    cost_usd: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO llm_usage (user_id, conversation_id, message_id, provider, model, prompt_tokens, completion_tokens, total_tokens, cost_usd)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(user_id)
    .bind(conversation_id)
    .bind(message_id)
    .bind(provider)
    .bind(model)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total_tokens)
    .bind(rust_decimal::Decimal::from_f64_retain(cost_usd).unwrap_or_default())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_usage_summary(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<ProviderUsage>, sqlx::Error> {
    sqlx::query_as::<_, ProviderUsage>(
        r#"
        SELECT provider, model,
               SUM(total_tokens)::bigint as total_tokens,
               SUM(cost_usd) as total_cost,
               COUNT(*)::bigint as request_count
        FROM llm_usage
        WHERE user_id = $1
        GROUP BY provider, model
        ORDER BY total_cost DESC NULLS LAST
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}
