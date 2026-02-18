use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    Layer0Public, Layer0Update, Layer1Context, Layer1Update, Layer2Personal, Layer2Update,
};

pub async fn get_layer0(pool: &PgPool, user_id: Uuid) -> Result<Option<Layer0Public>, sqlx::Error> {
    sqlx::query_as::<_, Layer0Public>("SELECT * FROM layer0_public WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn update_layer0(
    pool: &PgPool,
    user_id: Uuid,
    data: &Layer0Update,
) -> Result<Layer0Public, sqlx::Error> {
    sqlx::query_as::<_, Layer0Public>(
        r#"
        UPDATE layer0_public SET
            language = COALESCE($2, language),
            timezone = COALESCE($3, timezone),
            output_format = COALESCE($4, output_format),
            blocked_topics = COALESCE($5, blocked_topics),
            communication_style = COALESCE($6, communication_style),
            updated_at = NOW()
        WHERE user_id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&data.language)
    .bind(&data.timezone)
    .bind(&data.output_format)
    .bind(&data.blocked_topics)
    .bind(&data.communication_style)
    .fetch_one(pool)
    .await
}

pub async fn get_layer1(pool: &PgPool, user_id: Uuid) -> Result<Option<Layer1Context>, sqlx::Error> {
    sqlx::query_as::<_, Layer1Context>("SELECT * FROM layer1_context WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn update_layer1(
    pool: &PgPool,
    user_id: Uuid,
    data: &Layer1Update,
) -> Result<Layer1Context, sqlx::Error> {
    sqlx::query_as::<_, Layer1Context>(
        r#"
        UPDATE layer1_context SET
            active_projects = COALESCE($2, active_projects),
            recent_conversations = COALESCE($3, recent_conversations),
            working_directories = COALESCE($4, working_directories),
            current_goals = COALESCE($5, current_goals),
            pinned_memories = COALESCE($6, pinned_memories),
            updated_at = NOW()
        WHERE user_id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&data.active_projects)
    .bind(&data.recent_conversations)
    .bind(&data.working_directories)
    .bind(&data.current_goals)
    .bind(&data.pinned_memories)
    .fetch_one(pool)
    .await
}

pub async fn get_layer2(pool: &PgPool, user_id: Uuid) -> Result<Option<Layer2Personal>, sqlx::Error> {
    sqlx::query_as::<_, Layer2Personal>("SELECT * FROM layer2_personal WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn update_layer2(
    pool: &PgPool,
    user_id: Uuid,
    data: &Layer2Update,
) -> Result<Layer2Personal, sqlx::Error> {
    sqlx::query_as::<_, Layer2Personal>(
        r#"
        UPDATE layer2_personal SET
            display_name = COALESCE($2, display_name),
            location = COALESCE($3, location),
            interests = COALESCE($4, interests),
            contact_preferences = COALESCE($5, contact_preferences),
            relationship_notes = COALESCE($6, relationship_notes),
            updated_at = NOW()
        WHERE user_id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&data.display_name)
    .bind(&data.location)
    .bind(&data.interests)
    .bind(&data.contact_preferences)
    .bind(&data.relationship_notes)
    .fetch_one(pool)
    .await
}
