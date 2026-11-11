use sqlx::PgPool;
use uuid::Uuid;

use crate::crypto::vault;

#[derive(Debug, Default, Clone, Copy)]
pub struct MigrationStats {
    pub provider_keys_migrated: u64,
    pub vaults_migrated: u64,
    pub skipped: u64,
    pub failed: u64,
}

/// Re-encrypt secrets written with the legacy global key into the per-user
/// versioned (CBR2) format. The operation is idempotent and runs inside a
/// transaction guarded by a PostgreSQL advisory lock, so concurrent app
/// instances cannot race each other.
///
/// Rows that cannot be decrypted with the current session secret are left
/// untouched and reported in `failed`; the application can still read them
/// through the legacy fallback.
pub async fn migrate_secrets_to_v2(
    pool: &PgPool,
    session_secret: &str,
) -> Result<MigrationStats, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(918273645)")
        .execute(&mut *tx)
        .await?;

    let mut stats = MigrationStats::default();
    let legacy_key = vault::derive_legacy_key(session_secret);

    let provider_rows: Vec<(Uuid, Uuid, Vec<u8>)> =
        sqlx::query_as("SELECT id, user_id, encrypted_key FROM llm_provider_keys")
            .fetch_all(&mut *tx)
            .await?;

    for (id, user_id, encrypted) in provider_rows {
        if vault::is_v2(&encrypted) {
            stats.skipped += 1;
            continue;
        }

        match vault::decrypt_legacy(&encrypted, &legacy_key) {
            Ok(plain) => match vault::encrypt_for_user(&plain, session_secret, user_id, "llm") {
                Ok(new_value) => {
                    sqlx::query("UPDATE llm_provider_keys SET encrypted_key = $1 WHERE id = $2")
                        .bind(new_value)
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                    stats.provider_keys_migrated += 1;
                }
                Err(e) => {
                    tracing::warn!("Could not re-encrypt provider key {}: {}", id, e);
                    stats.failed += 1;
                }
            },
            Err(e) => {
                tracing::warn!(
                    "Could not decrypt legacy provider key {}; leaving it unchanged: {}",
                    id,
                    e
                );
                stats.failed += 1;
            }
        }
    }

    let vault_rows: Vec<(Uuid, Vec<u8>)> =
        sqlx::query_as("SELECT user_id, encrypted_data FROM layer3_vault")
            .fetch_all(&mut *tx)
            .await?;

    for (user_id, encrypted) in vault_rows {
        if vault::is_v2(&encrypted) {
            stats.skipped += 1;
            continue;
        }

        match vault::decrypt_legacy(&encrypted, &legacy_key) {
            Ok(plain) => match vault::encrypt_for_user(&plain, session_secret, user_id, "vault") {
                Ok(new_value) => {
                    let key_hash = crate::crypto::hash::sha256_hash(&hex::encode(
                        vault::derive_user_key(session_secret, user_id, "vault"),
                    ));
                    sqlx::query(
                        "UPDATE layer3_vault SET encrypted_data = $1, encryption_key_hash = $2, updated_at = NOW() WHERE user_id = $3",
                    )
                    .bind(new_value)
                    .bind(key_hash)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
                    stats.vaults_migrated += 1;
                }
                Err(e) => {
                    tracing::warn!("Could not re-encrypt vault for user {}: {}", user_id, e);
                    stats.failed += 1;
                }
            },
            Err(e) => {
                tracing::warn!(
                    "Could not decrypt legacy vault for user {}; leaving it unchanged: {}",
                    user_id,
                    e
                );
                stats.failed += 1;
            }
        }
    }

    tx.commit().await?;
    Ok(stats)
}
