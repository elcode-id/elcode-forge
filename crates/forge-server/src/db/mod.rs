//! Database access layer supporting embedded SQLite and external PostgreSQL.

pub mod models;

use std::str::FromStr;

use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{PgPool, SqlitePool};

use crate::error::ServerError;
use models::{ApiKeyRow, FindingRow, ProjectRow, SnapshotRow};

/// Dual database connection pool.
#[derive(Clone)]
pub enum DbPool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

/// Thread-safe database handle.
#[derive(Clone)]
pub struct Db {
    pool: DbPool,
    backend_name: &'static str,
}

impl Db {
    /// Connects to database based on URI scheme (`sqlite:` or `postgres:` / `postgresql:`).
    pub async fn connect(url: &str) -> Result<Self, ServerError> {
        if url.starts_with("postgres:") || url.starts_with("postgresql:") {
            let pool = PgPoolOptions::new()
                .max_connections(20)
                .connect(url)
                .await
                .map_err(|e| {
                    ServerError::Database(format!("Failed to connect to PostgreSQL: {e}"))
                })?;
            Ok(Self {
                pool: DbPool::Postgres(pool),
                backend_name: "postgres",
            })
        } else {
            let options = SqliteConnectOptions::from_str(url)
                .map_err(|e| ServerError::Database(format!("Invalid SQLite URL '{url}': {e}")))?
                .create_if_missing(true);

            let pool = SqlitePoolOptions::new()
                .max_connections(20)
                .connect_with(options)
                .await
                .map_err(|e| ServerError::Database(format!("Failed to connect to SQLite: {e}")))?;
            Ok(Self {
                pool: DbPool::Sqlite(pool),
                backend_name: "sqlite",
            })
        }
    }

    /// Creates an in-memory SQLite database instance (useful for unit & integration testing).
    pub async fn connect_in_memory() -> Result<Self, ServerError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(|e| {
                ServerError::Database(format!("Failed to configure in-memory SQLite: {e}"))
            })?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| {
                ServerError::Database(format!("Failed to initialize in-memory SQLite: {e}"))
            })?;

        let db = Self {
            pool: DbPool::Sqlite(pool),
            backend_name: "sqlite",
        };
        db.migrate().await?;
        Ok(db)
    }

    /// Returns human-readable database backend name ("sqlite" or "postgres").
    pub fn backend_name(&self) -> &'static str {
        self.backend_name
    }

    /// Runs idempotent database schema migrations.
    pub async fn migrate(&self) -> Result<(), ServerError> {
        let migration_sql = r#"
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS snapshots (
            id TEXT PRIMARY KEY,
            repo_id TEXT NOT NULL,
            branch TEXT NOT NULL,
            commit_hash TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            tool_version TEXT NOT NULL,
            health_overall BIGINT NOT NULL,
            health_json TEXT NOT NULL,
            findings_count BIGINT NOT NULL,
            modules_json TEXT,
            metadata_json TEXT,
            created_at BIGINT NOT NULL,
            FOREIGN KEY (repo_id) REFERENCES projects(id)
        );

        CREATE INDEX IF NOT EXISTS idx_snapshots_lookup ON snapshots(repo_id, branch, created_at DESC);

        CREATE TABLE IF NOT EXISTS findings (
            id TEXT PRIMARY KEY,
            snapshot_id TEXT NOT NULL,
            finding_id TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            category TEXT NOT NULL,
            severity TEXT NOT NULL,
            confidence TEXT NOT NULL,
            title TEXT NOT NULL,
            finding_json TEXT NOT NULL,
            FOREIGN KEY (snapshot_id) REFERENCES snapshots(id)
        );

        CREATE INDEX IF NOT EXISTS idx_findings_snapshot ON findings(snapshot_id);

        CREATE TABLE IF NOT EXISTS api_keys (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            key_hash TEXT NOT NULL UNIQUE,
            created_at BIGINT NOT NULL,
            expires_at BIGINT,
            is_active BIGINT NOT NULL DEFAULT 1
        );
        "#;

        match &self.pool {
            DbPool::Sqlite(pool) => {
                for statement in migration_sql.split(';') {
                    let trimmed = statement.trim();
                    if !trimmed.is_empty() {
                        sqlx::query(trimmed).execute(pool).await.map_err(|e| {
                            ServerError::Database(format!("SQLite migration statement failed: {e}"))
                        })?;
                    }
                }
            }
            DbPool::Postgres(pool) => {
                for statement in migration_sql.split(';') {
                    let trimmed = statement.trim();
                    if !trimmed.is_empty() {
                        sqlx::query(trimmed).execute(pool).await.map_err(|e| {
                            ServerError::Database(format!(
                                "Postgres migration statement failed: {e}"
                            ))
                        })?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Upserts project record.
    pub async fn upsert_project(&self, id: &str, name: &str, now: i64) -> Result<(), ServerError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO projects (id, name, created_at, updated_at)
                    VALUES ($1, $2, $3, $3)
                    ON CONFLICT(id) DO UPDATE SET name = excluded.name, updated_at = excluded.updated_at
                    "#,
                )
                .bind(id)
                .bind(name)
                .bind(now)
                .execute(pool)
                .await
                .map_err(|e| ServerError::Database(format!("Failed to upsert project: {e}")))?;
            }
            DbPool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO projects (id, name, created_at, updated_at)
                    VALUES ($1, $2, $3, $3)
                    ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name, updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(id)
                .bind(name)
                .bind(now)
                .execute(pool)
                .await
                .map_err(|e| ServerError::Database(format!("Failed to upsert project: {e}")))?;
            }
        }
        Ok(())
    }

    /// Fetches project by ID.
    pub async fn get_project(&self, id: &str) -> Result<Option<ProjectRow>, ServerError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let row = sqlx::query_as::<_, ProjectRow>("SELECT * FROM projects WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServerError::Database(format!("Failed to fetch project: {e}")))?;
                Ok(row)
            }
            DbPool::Postgres(pool) => {
                let row = sqlx::query_as::<_, ProjectRow>("SELECT * FROM projects WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServerError::Database(format!("Failed to fetch project: {e}")))?;
                Ok(row)
            }
        }
    }

    /// Stores snapshot and all associated findings within a transaction.
    pub async fn save_snapshot(
        &self,
        snapshot: &SnapshotRow,
        findings: &[FindingRow],
    ) -> Result<(), ServerError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let mut tx = pool.begin().await.map_err(|e| {
                    ServerError::Database(format!("Failed to start transaction: {e}"))
                })?;

                sqlx::query(
                    r#"
                    INSERT INTO snapshots (
                        id, repo_id, branch, commit_hash, content_hash,
                        tool_version, health_overall, health_json, findings_count,
                        modules_json, metadata_json, created_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    "#,
                )
                .bind(&snapshot.id)
                .bind(&snapshot.repo_id)
                .bind(&snapshot.branch)
                .bind(&snapshot.commit_hash)
                .bind(&snapshot.content_hash)
                .bind(&snapshot.tool_version)
                .bind(snapshot.health_overall)
                .bind(&snapshot.health_json)
                .bind(snapshot.findings_count)
                .bind(&snapshot.modules_json)
                .bind(&snapshot.metadata_json)
                .bind(snapshot.created_at)
                .execute(&mut *tx)
                .await
                .map_err(|e| ServerError::Database(format!("Failed to insert snapshot: {e}")))?;

                for f in findings {
                    sqlx::query(
                        r#"
                        INSERT INTO findings (
                            id, snapshot_id, finding_id, rule_id, category,
                            severity, confidence, title, finding_json
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                        "#,
                    )
                    .bind(&f.id)
                    .bind(&f.snapshot_id)
                    .bind(&f.finding_id)
                    .bind(&f.rule_id)
                    .bind(&f.category)
                    .bind(&f.severity)
                    .bind(&f.confidence)
                    .bind(&f.title)
                    .bind(&f.finding_json)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| ServerError::Database(format!("Failed to insert finding: {e}")))?;
                }

                tx.commit().await.map_err(|e| {
                    ServerError::Database(format!("Failed to commit transaction: {e}"))
                })?;
            }
            DbPool::Postgres(pool) => {
                let mut tx = pool.begin().await.map_err(|e| {
                    ServerError::Database(format!("Failed to start transaction: {e}"))
                })?;

                sqlx::query(
                    r#"
                    INSERT INTO snapshots (
                        id, repo_id, branch, commit_hash, content_hash,
                        tool_version, health_overall, health_json, findings_count,
                        modules_json, metadata_json, created_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    "#,
                )
                .bind(&snapshot.id)
                .bind(&snapshot.repo_id)
                .bind(&snapshot.branch)
                .bind(&snapshot.commit_hash)
                .bind(&snapshot.content_hash)
                .bind(&snapshot.tool_version)
                .bind(snapshot.health_overall)
                .bind(&snapshot.health_json)
                .bind(snapshot.findings_count)
                .bind(&snapshot.modules_json)
                .bind(&snapshot.metadata_json)
                .bind(snapshot.created_at)
                .execute(&mut *tx)
                .await
                .map_err(|e| ServerError::Database(format!("Failed to insert snapshot: {e}")))?;

                for f in findings {
                    sqlx::query(
                        r#"
                        INSERT INTO findings (
                            id, snapshot_id, finding_id, rule_id, category,
                            severity, confidence, title, finding_json
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                        "#,
                    )
                    .bind(&f.id)
                    .bind(&f.snapshot_id)
                    .bind(&f.finding_id)
                    .bind(&f.rule_id)
                    .bind(&f.category)
                    .bind(&f.severity)
                    .bind(&f.confidence)
                    .bind(&f.title)
                    .bind(&f.finding_json)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| ServerError::Database(format!("Failed to insert finding: {e}")))?;
                }

                tx.commit().await.map_err(|e| {
                    ServerError::Database(format!("Failed to commit transaction: {e}"))
                })?;
            }
        }

        Ok(())
    }

    /// Fetches the latest snapshot and its findings for given repo and branch.
    pub async fn get_latest_snapshot(
        &self,
        repo_id: &str,
        branch: &str,
    ) -> Result<Option<(SnapshotRow, Vec<FindingRow>)>, ServerError> {
        let snapshot_opt: Option<SnapshotRow> = match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query_as::<_, SnapshotRow>(
                    "SELECT * FROM snapshots WHERE repo_id = $1 AND branch = $2 ORDER BY created_at DESC LIMIT 1",
                )
                .bind(repo_id)
                .bind(branch)
                .fetch_optional(pool)
                .await
                .map_err(|e| ServerError::Database(format!("Failed to fetch latest snapshot: {e}")))?
            }
            DbPool::Postgres(pool) => {
                sqlx::query_as::<_, SnapshotRow>(
                    "SELECT * FROM snapshots WHERE repo_id = $1 AND branch = $2 ORDER BY created_at DESC LIMIT 1",
                )
                .bind(repo_id)
                .bind(branch)
                .fetch_optional(pool)
                .await
                .map_err(|e| ServerError::Database(format!("Failed to fetch latest snapshot: {e}")))?
            }
        };

        let Some(snapshot) = snapshot_opt else {
            return Ok(None);
        };

        let findings = match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query_as::<_, FindingRow>("SELECT * FROM findings WHERE snapshot_id = $1")
                    .bind(&snapshot.id)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        ServerError::Database(format!("Failed to fetch snapshot findings: {e}"))
                    })?
            }
            DbPool::Postgres(pool) => {
                sqlx::query_as::<_, FindingRow>("SELECT * FROM findings WHERE snapshot_id = $1")
                    .bind(&snapshot.id)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        ServerError::Database(format!("Failed to fetch snapshot findings: {e}"))
                    })?
            }
        };

        Ok(Some((snapshot, findings)))
    }

    /// Stores an API key hash.
    pub async fn create_api_key(
        &self,
        id: &str,
        name: &str,
        key_hash: &str,
        now: i64,
        expires_at: Option<i64>,
    ) -> Result<(), ServerError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO api_keys (id, name, key_hash, created_at, expires_at, is_active)
                    VALUES ($1, $2, $3, $4, $5, 1)
                    "#,
                )
                .bind(id)
                .bind(name)
                .bind(key_hash)
                .bind(now)
                .bind(expires_at)
                .execute(pool)
                .await
                .map_err(|e| ServerError::Database(format!("Failed to insert API key: {e}")))?;
            }
            DbPool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO api_keys (id, name, key_hash, created_at, expires_at, is_active)
                    VALUES ($1, $2, $3, $4, $5, 1)
                    "#,
                )
                .bind(id)
                .bind(name)
                .bind(key_hash)
                .bind(now)
                .bind(expires_at)
                .execute(pool)
                .await
                .map_err(|e| ServerError::Database(format!("Failed to insert API key: {e}")))?;
            }
        }
        Ok(())
    }

    /// Checks if a given key hash is active and unexpired.
    pub async fn validate_api_key_hash(
        &self,
        key_hash: &str,
        now: i64,
    ) -> Result<bool, ServerError> {
        let key_opt: Option<ApiKeyRow> = match &self.pool {
            DbPool::Sqlite(pool) => sqlx::query_as::<_, ApiKeyRow>(
                "SELECT * FROM api_keys WHERE key_hash = $1 AND is_active = 1",
            )
            .bind(key_hash)
            .fetch_optional(pool)
            .await
            .map_err(|e| ServerError::Database(format!("Failed to query API key: {e}")))?,
            DbPool::Postgres(pool) => sqlx::query_as::<_, ApiKeyRow>(
                "SELECT * FROM api_keys WHERE key_hash = $1 AND is_active = 1",
            )
            .bind(key_hash)
            .fetch_optional(pool)
            .await
            .map_err(|e| ServerError::Database(format!("Failed to query API key: {e}")))?,
        };

        if let Some(key) = key_opt {
            if let Some(exp) = key.expires_at {
                if now > exp {
                    return Ok(false);
                }
            }
            return Ok(true);
        }

        Ok(false)
    }
}
