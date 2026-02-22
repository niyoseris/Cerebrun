use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Conversation, ConversationMessage};

pub async fn create_conversation(
    pool: &PgPool,
    user_id: Uuid,
    provider: &str,
    model: &str,
    title: Option<&str>,
    system_prompt: Option<&str>,
    forked_from: Option<Uuid>,
    fork_point: Option<Uuid>,
) -> Result<Conversation, sqlx::Error> {
    sqlx::query_as::<_, Conversation>(
        r#"
        INSERT INTO conversations (user_id, provider, model, title, system_prompt, forked_from, fork_point_message_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(provider)
    .bind(model)
    .bind(title)
    .bind(system_prompt)
    .bind(forked_from)
    .bind(fork_point)
    .fetch_one(pool)
    .await
}

pub async fn list_conversations(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Conversation>, sqlx::Error> {
    sqlx::query_as::<_, Conversation>(
        "SELECT * FROM conversations WHERE user_id = $1 ORDER BY updated_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn get_conversation(
    pool: &PgPool,
    conv_id: Uuid,
    user_id: Uuid,
) -> Result<Option<Conversation>, sqlx::Error> {
    sqlx::query_as::<_, Conversation>(
        "SELECT * FROM conversations WHERE id = $1 AND user_id = $2",
    )
    .bind(conv_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn add_message(
    pool: &PgPool,
    conversation_id: Uuid,
    role: &str,
    content: &str,
    provider: Option<&str>,
    model: Option<&str>,
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
) -> Result<ConversationMessage, sqlx::Error> {
    let msg = sqlx::query_as::<_, ConversationMessage>(
        r#"
        INSERT INTO conversation_messages (conversation_id, role, content, provider, model, prompt_tokens, completion_tokens, total_tokens)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(conversation_id)
    .bind(role)
    .bind(content)
    .bind(provider)
    .bind(model)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total_tokens)
    .fetch_one(pool)
    .await?;

    sqlx::query("UPDATE conversations SET updated_at = NOW() WHERE id = $1")
        .bind(conversation_id)
        .execute(pool)
        .await?;

    Ok(msg)
}

pub async fn get_messages(
    pool: &PgPool,
    conversation_id: Uuid,
) -> Result<Vec<ConversationMessage>, sqlx::Error> {
    sqlx::query_as::<_, ConversationMessage>(
        "SELECT * FROM conversation_messages WHERE conversation_id = $1 ORDER BY created_at ASC",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
}

pub async fn get_messages_up_to(
    pool: &PgPool,
    conversation_id: Uuid,
    message_id: Uuid,
) -> Result<Vec<ConversationMessage>, sqlx::Error> {
    sqlx::query_as::<_, ConversationMessage>(
        r#"
        SELECT * FROM conversation_messages 
        WHERE conversation_id = $1 
          AND created_at <= (SELECT created_at FROM conversation_messages WHERE id = $2)
        ORDER BY created_at ASC
        "#,
    )
    .bind(conversation_id)
    .bind(message_id)
    .fetch_all(pool)
    .await
}

pub async fn delete_conversation(
    pool: &PgPool,
    conv_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM conversations WHERE id = $1 AND user_id = $2")
        .bind(conv_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn search_conversations(
    pool: &PgPool,
    user_id: Uuid,
    query: &str,
    provider_filter: Option<&str>,
    limit: i64,
) -> Result<Vec<(Conversation, Vec<ConversationMessage>)>, sqlx::Error> {
    let search_pattern = format!("%{}%", query);

    let matching_convs = if let Some(prov) = provider_filter {
        sqlx::query_as::<_, Conversation>(
            r#"
            SELECT DISTINCT c.* FROM conversations c
            LEFT JOIN conversation_messages m ON m.conversation_id = c.id
            WHERE c.user_id = $1
              AND c.provider = $4
              AND (c.title ILIKE $2 OR m.content ILIKE $2)
            ORDER BY c.updated_at DESC
            LIMIT $3
            "#,
        )
        .bind(user_id)
        .bind(&search_pattern)
        .bind(limit)
        .bind(prov)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, Conversation>(
            r#"
            SELECT DISTINCT c.* FROM conversations c
            LEFT JOIN conversation_messages m ON m.conversation_id = c.id
            WHERE c.user_id = $1
              AND (c.title ILIKE $2 OR m.content ILIKE $2)
            ORDER BY c.updated_at DESC
            LIMIT $3
            "#,
        )
        .bind(user_id)
        .bind(&search_pattern)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    let mut results = Vec::new();
    for conv in matching_convs {
        let messages = get_messages(pool, conv.id).await?;
        results.push((conv, messages));
    }
    Ok(results)
}

pub async fn get_recent_conversations(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<Conversation>, sqlx::Error> {
    sqlx::query_as::<_, Conversation>(
        "SELECT * FROM conversations WHERE user_id = $1 ORDER BY updated_at DESC LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_recent_messages_from_other_conversations(
    pool: &PgPool,
    user_id: Uuid,
    current_conv_id: Uuid,
    max_conversations: i64,
) -> Result<Vec<ConversationMessage>, sqlx::Error> {
    sqlx::query_as::<_, ConversationMessage>(
        r#"
        WITH recent_convs AS (
            SELECT id FROM conversations
            WHERE user_id = $1 AND id != $2
            ORDER BY updated_at DESC
            LIMIT $3
        )
        SELECT m.* FROM conversation_messages m
        WHERE m.conversation_id IN (SELECT id FROM recent_convs)
        ORDER BY m.conversation_id, m.created_at ASC
        "#,
    )
    .bind(user_id)
    .bind(current_conv_id)
    .bind(max_conversations)
    .fetch_all(pool)
    .await
}
