use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    let migration_sql = include_str!("../../migrations/001_initial.sql");

    let has_users = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'users')",
    )
    .fetch_one(pool)
    .await?;

    if !has_users {
        for statement in migration_sql.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
        tracing::info!("Database migrations applied successfully");
    } else {
        tracing::info!("Database already initialized, skipping migrations");
    }

    Ok(())
}
