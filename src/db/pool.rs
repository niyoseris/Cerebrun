use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

async fn table_exists(pool: &PgPool, table: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT FROM information_schema.tables \
         WHERE table_schema = current_schema() AND table_name = $1)",
    )
    .bind(table)
    .fetch_one(pool)
    .await
}

async fn column_exists(pool: &PgPool, table: &str, column: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT FROM information_schema.columns \
         WHERE table_schema = current_schema() AND table_name = $1 AND column_name = $2)",
    )
    .bind(table)
    .bind(column)
    .fetch_one(pool)
    .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    let migration_001 = include_str!("../../migrations/001_initial.sql");
    let migration_002 = include_str!("../../migrations/002_llm_gateway.sql");
    let migration_003 = include_str!("../../migrations/003_knowledge_base.sql");
    let migration_004 = include_str!("../../migrations/004_vector_embeddings.sql");
    let migration_005 = include_str!("../../migrations/005_system_tables.sql");
    let migration_006 = include_str!("../../migrations/006_debates.sql");
    let migration_007 = include_str!("../../migrations/007_version_history.sql");
    let migration_008 = include_str!("../../migrations/008_schema_drift.sql");

    if !table_exists(pool, "users").await? {
        run_sql(pool, migration_001).await?;
        tracing::info!("Migration 001 applied");
    }

    if !table_exists(pool, "llm_provider_keys").await? {
        run_sql(pool, migration_002).await?;
        tracing::info!("Migration 002 (LLM gateway) applied");
    }

    if !table_exists(pool, "knowledge_entries").await? {
        run_sql(pool, migration_003).await?;
        tracing::info!("Migration 003 (knowledge base) applied");
    }

    // 004 is partially idempotent, so also run it when only one of its two
    // effects is missing (older deployments may have added the table by hand).
    if !table_exists(pool, "context_embeddings").await?
        || !column_exists(pool, "knowledge_entries", "embedding").await?
        || !column_exists(pool, "conversations", "inject_context").await?
    {
        run_sql(pool, migration_004).await?;
        tracing::info!("Migration 004 (vector embeddings) applied");
    }

    if !table_exists(pool, "system_settings").await? {
        run_sql(pool, migration_005).await?;
        tracing::info!("Migration 005 (system tables) applied");
    }

    if !table_exists(pool, "debates").await? {
        run_sql(pool, migration_006).await?;
        tracing::info!("Migration 006 (debates) applied");
    }

    if !table_exists(pool, "content_versions").await?
        || !column_exists(pool, "knowledge_entries", "deleted_at").await?
        || !column_exists(pool, "debate_messages", "deleted_at").await?
    {
        run_sql(pool, migration_007).await?;
        tracing::info!("Migration 007 (version history) applied");
    }

    if !column_exists(pool, "users", "is_admin").await?
        || !column_exists(pool, "conversations", "use_tools").await?
    {
        run_sql(pool, migration_008).await?;
        tracing::info!("Migration 008 (schema drift) applied");
    }

    tracing::info!("All migrations up to date");
    Ok(())
}

async fn run_sql(pool: &PgPool, sql: &str) -> Result<(), sqlx::Error> {
    for statement in sql.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            sqlx::query(trimmed).execute(pool).await?;
        }
    }
    Ok(())
}
