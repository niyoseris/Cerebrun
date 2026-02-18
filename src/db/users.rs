use sqlx::PgPool;
use uuid::Uuid;

use crate::models::User;

pub async fn upsert_user(
    pool: &PgPool,
    google_id: &str,
    email: &str,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (google_id, email, display_name, avatar_url)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (google_id) DO UPDATE SET
            email = EXCLUDED.email,
            display_name = EXCLUDED.display_name,
            avatar_url = EXCLUDED.avatar_url,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(google_id)
    .bind(email)
    .bind(display_name)
    .bind(avatar_url)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        "INSERT INTO layer0_public (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user.id)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO layer1_context (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user.id)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO layer2_personal (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user.id)
    .execute(pool)
    .await?;

    Ok(user)
}

pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn delete_user(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
