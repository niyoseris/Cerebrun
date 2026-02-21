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
