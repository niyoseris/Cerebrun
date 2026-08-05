use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Debate, DebateMessage, DebateParticipant};

pub async fn create_debate(
    pool: &PgPool,
    user_id: Uuid,
    api_key_id: Uuid,
    topic: &str,
    description: Option<&str>,
    completion_criteria: Option<&[String]>,
) -> Result<Debate, sqlx::Error> {
    let criteria_json = completion_criteria
        .map(|c| serde_json::to_value(c).unwrap_or(serde_json::json!([])))
        .unwrap_or(serde_json::json!([]));

    sqlx::query_as::<_, Debate>(
        r#"
        INSERT INTO debates (user_id, api_key_id, topic, description, completion_criteria)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(api_key_id)
    .bind(topic)
    .bind(description)
    .bind(&criteria_json)
    .fetch_one(pool)
    .await
}

pub async fn get_debate(pool: &PgPool, debate_id: Uuid) -> Result<Option<Debate>, sqlx::Error> {
    sqlx::query_as::<_, Debate>("SELECT * FROM debates WHERE id = $1")
        .bind(debate_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_debate_by_public_id(
    pool: &PgPool,
    public_id: Uuid,
) -> Result<Option<Debate>, sqlx::Error> {
    sqlx::query_as::<_, Debate>("SELECT * FROM debates WHERE public_id = $1")
        .bind(public_id)
        .fetch_optional(pool)
        .await
}

pub async fn list_debates(
    pool: &PgPool,
    user_id: Uuid,
    status_filter: Option<&str>,
    limit: i64,
) -> Result<Vec<Debate>, sqlx::Error> {
    if let Some(status) = status_filter {
        sqlx::query_as::<_, Debate>(
            "SELECT * FROM debates WHERE user_id = $1 AND status = $2 ORDER BY updated_at DESC LIMIT $3",
        )
        .bind(user_id)
        .bind(status)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, Debate>(
            "SELECT * FROM debates WHERE user_id = $1 ORDER BY updated_at DESC LIMIT $2",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

pub async fn update_debate_criteria(
    pool: &PgPool,
    debate_id: Uuid,
    criteria: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE debates SET completion_criteria = $2, updated_at = NOW() WHERE id = $1")
        .bind(debate_id)
        .bind(criteria)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn conclude_debate(
    pool: &PgPool,
    debate_id: Uuid,
    consensus_reached: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE debates SET status = 'concluded', consensus_reached = $2, concluded_at = NOW(), updated_at = NOW() WHERE id = $1",
    )
    .bind(debate_id)
    .bind(consensus_reached)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn close_debate(pool: &PgPool, debate_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE debates SET status = 'closed', concluded_at = NOW(), updated_at = NOW() WHERE id = $1",
    )
    .bind(debate_id)
    .execute(pool)
    .await?;
    Ok(())
}

// Participants

pub async fn add_participant(
    pool: &PgPool,
    debate_id: Uuid,
    provider: &str,
    model: &str,
    api_key_name: Option<&str>,
    criteria_agreed: Option<&serde_json::Value>,
) -> Result<DebateParticipant, sqlx::Error> {
    let default_criteria = serde_json::json!([]);
    let agreed = criteria_agreed.unwrap_or(&default_criteria);
    sqlx::query_as::<_, DebateParticipant>(
        r#"
        INSERT INTO debate_participants (debate_id, provider, model, api_key_name, criteria_agreed)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (debate_id, provider, model) DO UPDATE SET
            criteria_agreed = EXCLUDED.criteria_agreed,
            joined_at = NOW()
        RETURNING *
        "#,
    )
    .bind(debate_id)
    .bind(provider)
    .bind(model)
    .bind(api_key_name)
    .bind(agreed)
    .fetch_one(pool)
    .await
}

pub async fn list_participants(
    pool: &PgPool,
    debate_id: Uuid,
) -> Result<Vec<DebateParticipant>, sqlx::Error> {
    sqlx::query_as::<_, DebateParticipant>(
        "SELECT * FROM debate_participants WHERE debate_id = $1 ORDER BY joined_at ASC",
    )
    .bind(debate_id)
    .fetch_all(pool)
    .await
}

pub async fn get_participant(
    pool: &PgPool,
    debate_id: Uuid,
    provider: &str,
    model: &str,
) -> Result<Option<DebateParticipant>, sqlx::Error> {
    sqlx::query_as::<_, DebateParticipant>(
        "SELECT * FROM debate_participants WHERE debate_id = $1 AND provider = $2 AND model = $3",
    )
    .bind(debate_id)
    .bind(provider)
    .bind(model)
    .fetch_optional(pool)
    .await
}

// Messages

pub async fn add_message(
    pool: &PgPool,
    debate_id: Uuid,
    participant_id: Option<Uuid>,
    provider: &str,
    model: &str,
    round: i32,
    role: &str,
    content: &str,
    criteria_proposal: Option<&serde_json::Value>,
) -> Result<DebateMessage, sqlx::Error> {
    sqlx::query_as::<_, DebateMessage>(
        r#"
        INSERT INTO debate_messages (debate_id, participant_id, provider, model, round, role, content, criteria_proposal)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(debate_id)
    .bind(participant_id)
    .bind(provider)
    .bind(model)
    .bind(round)
    .bind(role)
    .bind(content)
    .bind(criteria_proposal)
    .fetch_one(pool)
    .await
}

pub async fn list_messages(
    pool: &PgPool,
    debate_id: Uuid,
) -> Result<Vec<DebateMessage>, sqlx::Error> {
    sqlx::query_as::<_, DebateMessage>(
        "SELECT * FROM debate_messages WHERE debate_id = $1 ORDER BY round ASC, created_at ASC",
    )
    .bind(debate_id)
    .fetch_all(pool)
    .await
}

pub async fn get_current_round(pool: &PgPool, debate_id: Uuid) -> Result<i32, sqlx::Error> {
    let result: (i32,) = sqlx::query_as(
        "SELECT COALESCE(MAX(round), 0) FROM debate_messages WHERE debate_id = $1",
    )
    .bind(debate_id)
    .fetch_one(pool)
    .await?;
    Ok(result.0)
}

pub async fn update_debate_timestamp(pool: &PgPool, debate_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE debates SET updated_at = NOW() WHERE id = $1")
        .bind(debate_id)
        .execute(pool)
        .await?;
    Ok(())
}