use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    let migration_001 = include_str!("../../migrations/001_initial.sql");
    let migration_002 = include_str!("../../migrations/002_llm_gateway.sql");

    let has_users = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'users')",
    )
    .fetch_one(pool)
    .await?;

    if !has_users {
        run_sql(pool, migration_001).await?;
        tracing::info!("Migration 001 applied");
    }

    let has_llm_keys = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'llm_provider_keys')",
    )
    .fetch_one(pool)
    .await?;

    if !has_llm_keys {
        run_sql(pool, migration_002).await?;
        tracing::info!("Migration 002 (LLM gateway) applied");
    }

    let migration_003 = include_str!("../../migrations/003_knowledge_base.sql");
    let has_knowledge = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'knowledge_entries')",
    )
    .fetch_one(pool)
    .await?;

    if !has_knowledge {
        run_sql(pool, migration_003).await?;
        tracing::info!("Migration 003 (knowledge base) applied");
    }

    let migration_004 = include_str!("../../migrations/004_vector_embeddings.sql");
    let has_context_embeddings = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'context_embeddings')",
    )
    .fetch_one(pool)
    .await?;

    if !has_context_embeddings {
        run_sql(pool, migration_004).await?;
        tracing::info!("Migration 004 (vector embeddings) applied");
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
