//! Database access layer supporting embedded SQLite and external PostgreSQL.

pub mod models;

use std::str::FromStr;

use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{PgPool, Row, SqlitePool};

use crate::error::ServerError;
use models::{
    BranchSummary, FindingRow, HistoryPoint, ProjectRow, ProjectSummary, SnapshotRow, WebhookRow,
};

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

        CREATE TABLE IF NOT EXISTS webhooks (
            id TEXT PRIMARY KEY,
            repo_id TEXT,
            url TEXT NOT NULL,
            secret TEXT,
            events_json TEXT NOT NULL,
            min_health_threshold BIGINT,
            is_active BIGINT NOT NULL DEFAULT 1,
            created_at BIGINT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_webhooks_repo ON webhooks(repo_id);
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
        let sql = "SELECT id, name, created_at, updated_at FROM projects WHERE id = $1";
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(sql)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServerError::Database(format!("Failed to fetch project: {e}")))?;
                match row {
                    Some(r) => Ok(Some(ProjectRow {
                        id: r
                            .try_get("id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        name: r
                            .try_get("name")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        created_at: r
                            .try_get("created_at")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        updated_at: r
                            .try_get("updated_at")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                    })),
                    None => Ok(None),
                }
            }
            DbPool::Postgres(pool) => {
                let row = sqlx::query(sql)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServerError::Database(format!("Failed to fetch project: {e}")))?;
                match row {
                    Some(r) => Ok(Some(ProjectRow {
                        id: r
                            .try_get("id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        name: r
                            .try_get("name")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        created_at: r
                            .try_get("created_at")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        updated_at: r
                            .try_get("updated_at")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                    })),
                    None => Ok(None),
                }
            }
        }
    }

    /// Lists all projects with their most recent snapshot statistics.
    pub async fn list_projects(&self) -> Result<Vec<ProjectSummary>, ServerError> {
        let projects_sql = "SELECT id, name, updated_at FROM projects ORDER BY name ASC";
        let latest_snap_sql = "SELECT branch, commit_hash, health_overall, findings_count, created_at FROM snapshots WHERE repo_id = $1 ORDER BY created_at DESC, id DESC LIMIT 1";

        let projects = match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(projects_sql)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| ServerError::Database(e.to_string()))?;
                let mut list = Vec::with_capacity(rows.len());
                for r in rows {
                    let id: String = r
                        .try_get("id")
                        .map_err(|e| ServerError::Database(e.to_string()))?;
                    let name: String = r
                        .try_get("name")
                        .map_err(|e| ServerError::Database(e.to_string()))?;
                    let updated_at: i64 = r
                        .try_get("updated_at")
                        .map_err(|e| ServerError::Database(e.to_string()))?;

                    let snap_row = sqlx::query(latest_snap_sql)
                        .bind(&id)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| ServerError::Database(e.to_string()))?;

                    let (latest_branch, latest_commit, latest_health, findings_count) =
                        match snap_row {
                            Some(sr) => (
                                Some(
                                    sr.try_get("branch")
                                        .map_err(|e| ServerError::Database(e.to_string()))?,
                                ),
                                Some(
                                    sr.try_get("commit_hash")
                                        .map_err(|e| ServerError::Database(e.to_string()))?,
                                ),
                                Some(
                                    sr.try_get::<i64, _>("health_overall")
                                        .map_err(|e| ServerError::Database(e.to_string()))?
                                        as u8,
                                ),
                                Some(
                                    sr.try_get::<i64, _>("findings_count")
                                        .map_err(|e| ServerError::Database(e.to_string()))?
                                        as usize,
                                ),
                            ),
                            None => (None, None, None, None),
                        };

                    list.push(ProjectSummary {
                        repo_id: id,
                        name,
                        latest_branch,
                        latest_commit,
                        latest_health,
                        findings_count,
                        updated_at,
                    });
                }
                list
            }
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(projects_sql)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| ServerError::Database(e.to_string()))?;
                let mut list = Vec::with_capacity(rows.len());
                for r in rows {
                    let id: String = r
                        .try_get("id")
                        .map_err(|e| ServerError::Database(e.to_string()))?;
                    let name: String = r
                        .try_get("name")
                        .map_err(|e| ServerError::Database(e.to_string()))?;
                    let updated_at: i64 = r
                        .try_get("updated_at")
                        .map_err(|e| ServerError::Database(e.to_string()))?;

                    let snap_row = sqlx::query(latest_snap_sql)
                        .bind(&id)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| ServerError::Database(e.to_string()))?;

                    let (latest_branch, latest_commit, latest_health, findings_count) =
                        match snap_row {
                            Some(sr) => (
                                Some(
                                    sr.try_get("branch")
                                        .map_err(|e| ServerError::Database(e.to_string()))?,
                                ),
                                Some(
                                    sr.try_get("commit_hash")
                                        .map_err(|e| ServerError::Database(e.to_string()))?,
                                ),
                                Some(
                                    sr.try_get::<i64, _>("health_overall")
                                        .map_err(|e| ServerError::Database(e.to_string()))?
                                        as u8,
                                ),
                                Some(
                                    sr.try_get::<i64, _>("findings_count")
                                        .map_err(|e| ServerError::Database(e.to_string()))?
                                        as usize,
                                ),
                            ),
                            None => (None, None, None, None),
                        };

                    list.push(ProjectSummary {
                        repo_id: id,
                        name,
                        latest_branch,
                        latest_commit,
                        latest_health,
                        findings_count,
                        updated_at,
                    });
                }
                list
            }
        };

        Ok(projects)
    }

    /// Lists distinct branches and their latest snapshot for a project.
    pub async fn get_project_branches(
        &self,
        repo_id: &str,
    ) -> Result<Vec<BranchSummary>, ServerError> {
        let branch_sql =
            "SELECT DISTINCT branch FROM snapshots WHERE repo_id = $1 ORDER BY branch ASC";
        let latest_snap_sql = "SELECT commit_hash, health_overall, findings_count, created_at FROM snapshots WHERE repo_id = $1 AND branch = $2 ORDER BY created_at DESC, id DESC LIMIT 1";

        let branches = match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(branch_sql)
                    .bind(repo_id)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| ServerError::Database(e.to_string()))?;
                let mut list = Vec::with_capacity(rows.len());
                for r in rows {
                    let branch: String = r
                        .try_get("branch")
                        .map_err(|e| ServerError::Database(e.to_string()))?;
                    if let Some(sr) = sqlx::query(latest_snap_sql)
                        .bind(repo_id)
                        .bind(&branch)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| ServerError::Database(e.to_string()))?
                    {
                        list.push(BranchSummary {
                            branch,
                            latest_commit: sr
                                .try_get("commit_hash")
                                .map_err(|e| ServerError::Database(e.to_string()))?,
                            latest_health: sr
                                .try_get::<i64, _>("health_overall")
                                .map_err(|e| ServerError::Database(e.to_string()))?
                                as u8,
                            findings_count: sr
                                .try_get::<i64, _>("findings_count")
                                .map_err(|e| ServerError::Database(e.to_string()))?
                                as usize,
                            updated_at: sr
                                .try_get("created_at")
                                .map_err(|e| ServerError::Database(e.to_string()))?,
                        });
                    }
                }
                list
            }
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(branch_sql)
                    .bind(repo_id)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| ServerError::Database(e.to_string()))?;
                let mut list = Vec::with_capacity(rows.len());
                for r in rows {
                    let branch: String = r
                        .try_get("branch")
                        .map_err(|e| ServerError::Database(e.to_string()))?;
                    if let Some(sr) = sqlx::query(latest_snap_sql)
                        .bind(repo_id)
                        .bind(&branch)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| ServerError::Database(e.to_string()))?
                    {
                        list.push(BranchSummary {
                            branch,
                            latest_commit: sr
                                .try_get("commit_hash")
                                .map_err(|e| ServerError::Database(e.to_string()))?,
                            latest_health: sr
                                .try_get::<i64, _>("health_overall")
                                .map_err(|e| ServerError::Database(e.to_string()))?
                                as u8,
                            findings_count: sr
                                .try_get::<i64, _>("findings_count")
                                .map_err(|e| ServerError::Database(e.to_string()))?
                                as usize,
                            updated_at: sr
                                .try_get("created_at")
                                .map_err(|e| ServerError::Database(e.to_string()))?,
                        });
                    }
                }
                list
            }
        };

        Ok(branches)
    }

    /// Fetches chronological snapshot history for a given project branch.
    pub async fn get_branch_history(
        &self,
        repo_id: &str,
        branch: &str,
        limit: i64,
    ) -> Result<Vec<HistoryPoint>, ServerError> {
        let sql = "SELECT commit_hash, branch, created_at, health_overall, findings_count, health_json FROM snapshots WHERE repo_id = $1 AND branch = $2 ORDER BY created_at ASC, id ASC LIMIT $3";
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(sql)
                    .bind(repo_id)
                    .bind(branch)
                    .bind(limit)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| ServerError::Database(e.to_string()))?;
                let mut points = Vec::with_capacity(rows.len());
                for r in rows {
                    points.push(HistoryPoint {
                        commit_hash: r
                            .try_get("commit_hash")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        branch: r
                            .try_get("branch")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        timestamp_secs: r
                            .try_get("created_at")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        overall_health: r
                            .try_get::<i64, _>("health_overall")
                            .map_err(|e| ServerError::Database(e.to_string()))?
                            as u8,
                        findings_count: r
                            .try_get::<i64, _>("findings_count")
                            .map_err(|e| ServerError::Database(e.to_string()))?
                            as usize,
                        health_json: r
                            .try_get("health_json")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                    });
                }
                Ok(points)
            }
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(sql)
                    .bind(repo_id)
                    .bind(branch)
                    .bind(limit)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| ServerError::Database(e.to_string()))?;
                let mut points = Vec::with_capacity(rows.len());
                for r in rows {
                    points.push(HistoryPoint {
                        commit_hash: r
                            .try_get("commit_hash")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        branch: r
                            .try_get("branch")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        timestamp_secs: r
                            .try_get("created_at")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        overall_health: r
                            .try_get::<i64, _>("health_overall")
                            .map_err(|e| ServerError::Database(e.to_string()))?
                            as u8,
                        findings_count: r
                            .try_get::<i64, _>("findings_count")
                            .map_err(|e| ServerError::Database(e.to_string()))?
                            as usize,
                        health_json: r
                            .try_get("health_json")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                    });
                }
                Ok(points)
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
        let snap_sql = "SELECT id, repo_id, branch, commit_hash, content_hash, tool_version, health_overall, health_json, findings_count, modules_json, metadata_json, created_at FROM snapshots WHERE repo_id = $1 AND branch = $2 ORDER BY created_at DESC, id DESC LIMIT 1";
        let find_sql = "SELECT id, snapshot_id, finding_id, rule_id, category, severity, confidence, title, finding_json FROM findings WHERE snapshot_id = $1";

        match &self.pool {
            DbPool::Sqlite(pool) => {
                let snap_row = sqlx::query(snap_sql)
                    .bind(repo_id)
                    .bind(branch)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        ServerError::Database(format!("Failed to fetch latest snapshot: {e}"))
                    })?;

                let Some(r) = snap_row else {
                    return Ok(None);
                };

                let snapshot = SnapshotRow {
                    id: r
                        .try_get("id")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    repo_id: r
                        .try_get("repo_id")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    branch: r
                        .try_get("branch")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    commit_hash: r
                        .try_get("commit_hash")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    content_hash: r
                        .try_get("content_hash")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    tool_version: r
                        .try_get("tool_version")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    health_overall: r
                        .try_get("health_overall")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    health_json: r
                        .try_get("health_json")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    findings_count: r
                        .try_get("findings_count")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    modules_json: r
                        .try_get("modules_json")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    metadata_json: r
                        .try_get("metadata_json")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    created_at: r
                        .try_get("created_at")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                };

                let findings_db = sqlx::query(find_sql)
                    .bind(&snapshot.id)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        ServerError::Database(format!("Failed to fetch snapshot findings: {e}"))
                    })?;

                let mut findings = Vec::with_capacity(findings_db.len());
                for f in findings_db {
                    findings.push(FindingRow {
                        id: f
                            .try_get("id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        snapshot_id: f
                            .try_get("snapshot_id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        finding_id: f
                            .try_get("finding_id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        rule_id: f
                            .try_get("rule_id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        category: f
                            .try_get("category")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        severity: f
                            .try_get("severity")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        confidence: f
                            .try_get("confidence")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        title: f
                            .try_get("title")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        finding_json: f
                            .try_get("finding_json")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                    });
                }

                Ok(Some((snapshot, findings)))
            }
            DbPool::Postgres(pool) => {
                let snap_row = sqlx::query(snap_sql)
                    .bind(repo_id)
                    .bind(branch)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        ServerError::Database(format!("Failed to fetch latest snapshot: {e}"))
                    })?;

                let Some(r) = snap_row else {
                    return Ok(None);
                };

                let snapshot = SnapshotRow {
                    id: r
                        .try_get("id")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    repo_id: r
                        .try_get("repo_id")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    branch: r
                        .try_get("branch")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    commit_hash: r
                        .try_get("commit_hash")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    content_hash: r
                        .try_get("content_hash")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    tool_version: r
                        .try_get("tool_version")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    health_overall: r
                        .try_get("health_overall")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    health_json: r
                        .try_get("health_json")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    findings_count: r
                        .try_get("findings_count")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    modules_json: r
                        .try_get("modules_json")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    metadata_json: r
                        .try_get("metadata_json")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                    created_at: r
                        .try_get("created_at")
                        .map_err(|e| ServerError::Database(e.to_string()))?,
                };

                let findings_db = sqlx::query(find_sql)
                    .bind(&snapshot.id)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        ServerError::Database(format!("Failed to fetch snapshot findings: {e}"))
                    })?;

                let mut findings = Vec::with_capacity(findings_db.len());
                for f in findings_db {
                    findings.push(FindingRow {
                        id: f
                            .try_get("id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        snapshot_id: f
                            .try_get("snapshot_id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        finding_id: f
                            .try_get("finding_id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        rule_id: f
                            .try_get("rule_id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        category: f
                            .try_get("category")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        severity: f
                            .try_get("severity")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        confidence: f
                            .try_get("confidence")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        title: f
                            .try_get("title")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        finding_json: f
                            .try_get("finding_json")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                    });
                }

                Ok(Some((snapshot, findings)))
            }
        }
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
        let sql = "SELECT id, name, key_hash, created_at, expires_at, is_active FROM api_keys WHERE key_hash = $1 AND is_active = 1";
        let expires_at_opt: Option<Option<i64>> = match &self.pool {
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(sql)
                    .bind(key_hash)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServerError::Database(format!("Failed to query API key: {e}")))?;
                row.map(|r| {
                    r.try_get("expires_at")
                        .map_err(|e| ServerError::Database(e.to_string()))
                })
                .transpose()?
            }
            DbPool::Postgres(pool) => {
                let row = sqlx::query(sql)
                    .bind(key_hash)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServerError::Database(format!("Failed to query API key: {e}")))?;
                row.map(|r| {
                    r.try_get("expires_at")
                        .map_err(|e| ServerError::Database(e.to_string()))
                })
                .transpose()?
            }
        };

        if let Some(exp_opt) = expires_at_opt {
            if let Some(exp) = exp_opt {
                if now > exp {
                    return Ok(false);
                }
            }
            return Ok(true);
        }

        Ok(false)
    }

    /// Creates or updates a webhook subscription.
    pub async fn create_webhook(&self, webhook: &WebhookRow) -> Result<(), ServerError> {
        let sql = r#"
        INSERT INTO webhooks (id, repo_id, url, secret, events_json, min_health_threshold, is_active, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT(id) DO UPDATE SET
            repo_id = excluded.repo_id,
            url = excluded.url,
            secret = excluded.secret,
            events_json = excluded.events_json,
            min_health_threshold = excluded.min_health_threshold,
            is_active = excluded.is_active
        "#;

        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(sql)
                    .bind(&webhook.id)
                    .bind(&webhook.repo_id)
                    .bind(&webhook.url)
                    .bind(&webhook.secret)
                    .bind(&webhook.events_json)
                    .bind(webhook.min_health_threshold)
                    .bind(webhook.is_active)
                    .bind(webhook.created_at)
                    .execute(pool)
                    .await
                    .map_err(|e| ServerError::Database(format!("Failed to create webhook: {e}")))?;
            }
            DbPool::Postgres(pool) => {
                sqlx::query(sql)
                    .bind(&webhook.id)
                    .bind(&webhook.repo_id)
                    .bind(&webhook.url)
                    .bind(&webhook.secret)
                    .bind(&webhook.events_json)
                    .bind(webhook.min_health_threshold)
                    .bind(webhook.is_active)
                    .bind(webhook.created_at)
                    .execute(pool)
                    .await
                    .map_err(|e| ServerError::Database(format!("Failed to create webhook: {e}")))?;
            }
        }
        Ok(())
    }

    /// Lists active webhooks, optionally filtered by `repo_id`.
    pub async fn list_webhooks(
        &self,
        repo_id: Option<&str>,
    ) -> Result<Vec<WebhookRow>, ServerError> {
        let (sql, bind_repo) = if let Some(r) = repo_id {
            (
                "SELECT id, repo_id, url, secret, events_json, min_health_threshold, is_active, created_at FROM webhooks WHERE is_active = 1 AND (repo_id IS NULL OR repo_id = $1) ORDER BY created_at DESC",
                Some(r),
            )
        } else {
            (
                "SELECT id, repo_id, url, secret, events_json, min_health_threshold, is_active, created_at FROM webhooks WHERE is_active = 1 ORDER BY created_at DESC",
                None,
            )
        };

        match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = if let Some(r) = bind_repo {
                    sqlx::query(sql)
                        .bind(r)
                        .fetch_all(pool)
                        .await
                        .map_err(|e| ServerError::Database(e.to_string()))?
                } else {
                    sqlx::query(sql)
                        .fetch_all(pool)
                        .await
                        .map_err(|e| ServerError::Database(e.to_string()))?
                };
                let mut list = Vec::with_capacity(rows.len());
                for r in rows {
                    list.push(WebhookRow {
                        id: r
                            .try_get("id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        repo_id: r
                            .try_get("repo_id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        url: r
                            .try_get("url")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        secret: r
                            .try_get("secret")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        events_json: r
                            .try_get("events_json")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        min_health_threshold: r
                            .try_get("min_health_threshold")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        is_active: r
                            .try_get("is_active")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        created_at: r
                            .try_get("created_at")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                    });
                }
                Ok(list)
            }
            DbPool::Postgres(pool) => {
                let rows = if let Some(r) = bind_repo {
                    sqlx::query(sql)
                        .bind(r)
                        .fetch_all(pool)
                        .await
                        .map_err(|e| ServerError::Database(e.to_string()))?
                } else {
                    sqlx::query(sql)
                        .fetch_all(pool)
                        .await
                        .map_err(|e| ServerError::Database(e.to_string()))?
                };
                let mut list = Vec::with_capacity(rows.len());
                for r in rows {
                    list.push(WebhookRow {
                        id: r
                            .try_get("id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        repo_id: r
                            .try_get("repo_id")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        url: r
                            .try_get("url")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        secret: r
                            .try_get("secret")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        events_json: r
                            .try_get("events_json")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        min_health_threshold: r
                            .try_get("min_health_threshold")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        is_active: r
                            .try_get("is_active")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                        created_at: r
                            .try_get("created_at")
                            .map_err(|e| ServerError::Database(e.to_string()))?,
                    });
                }
                Ok(list)
            }
        }
    }

    /// Deactivates a webhook by ID.
    pub async fn delete_webhook(&self, id: &str) -> Result<bool, ServerError> {
        let sql = "UPDATE webhooks SET is_active = 0 WHERE id = $1";
        let affected = match &self.pool {
            DbPool::Sqlite(pool) => sqlx::query(sql)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| ServerError::Database(e.to_string()))?
                .rows_affected(),
            DbPool::Postgres(pool) => sqlx::query(sql)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| ServerError::Database(e.to_string()))?
                .rows_affected(),
        };
        Ok(affected > 0)
    }
}
