use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub agent_id: String,
    pub timestamp: i64,
    pub instruction_type: String,
    pub status: String,
    pub tx_signature: Option<String>,
    pub policy_violations: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewAuditEntry {
    pub agent_id: String,
    pub instruction_type: String,
    pub status: String,
    pub tx_signature: Option<String>,
    pub policy_violations: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

pub struct AuditStore {
    pool: SqlitePool,
}

impl AuditStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn append(&self, entry: &NewAuditEntry) -> Result<i64> {
        let now = Utc::now().timestamp();
        let violations_json = entry
            .policy_violations
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let metadata_json = entry
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        let result = sqlx::query(
            "INSERT INTO audit_log \
             (agent_id, timestamp, instruction_type, status, tx_signature, \
              policy_violations, metadata) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&entry.agent_id)
        .bind(now)
        .bind(&entry.instruction_type)
        .bind(&entry.status)
        .bind(&entry.tx_signature)
        .bind(&violations_json)
        .bind(&metadata_json)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn list_by_agent(
        &self,
        agent_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditEntry>> {
        let rows = sqlx::query_as::<_, AuditEntryRow>(
            "SELECT id, agent_id, timestamp, instruction_type, status, \
             tx_signature, policy_violations, metadata \
             FROM audit_log WHERE agent_id = ? ORDER BY timestamp DESC LIMIT ? OFFSET ?",
        )
        .bind(agent_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(AuditEntryRow::into_entry).collect())
    }

    pub async fn sum_daily_spend(&self, agent_id: &str) -> Result<f64> {
        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("valid midnight timestamp")
            .and_utc()
            .timestamp();

        let row: (f64,) = sqlx::query_as(
            "SELECT CAST(COALESCE(SUM( \
                 CASE WHEN json_valid(metadata) \
                      THEN COALESCE(json_extract(metadata, '$.usd_value'), 0) \
                      ELSE 0 END \
             ), 0) AS REAL) as total \
             FROM audit_log \
             WHERE agent_id = ? AND status = 'confirmed' AND timestamp >= ?",
        )
        .bind(agent_id)
        .bind(today_start)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    pub async fn sum_swap_volume(&self, agent_id: &str) -> Result<f64> {
        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("valid midnight timestamp")
            .and_utc()
            .timestamp();

        let row: (f64,) = sqlx::query_as(
            "SELECT CAST(COALESCE(SUM( \
                 CASE WHEN json_valid(metadata) \
                      THEN json_extract(metadata, '$.usd_volume') \
                      ELSE 0 END \
             ), 0) AS REAL) as total \
             FROM audit_log \
             WHERE agent_id = ? AND instruction_type = 'token_swap' AND status = 'confirmed' AND timestamp >= ?",
        )
        .bind(agent_id)
        .bind(today_start)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }
}

#[derive(sqlx::FromRow)]
struct AuditEntryRow {
    id: i64,
    agent_id: String,
    timestamp: i64,
    instruction_type: String,
    status: String,
    tx_signature: Option<String>,
    policy_violations: Option<String>,
    metadata: Option<String>,
}

impl AuditEntryRow {
    fn into_entry(self) -> AuditEntry {
        AuditEntry {
            id: self.id,
            agent_id: self.agent_id,
            timestamp: self.timestamp,
            instruction_type: self.instruction_type,
            status: self.status,
            tx_signature: self.tx_signature,
            policy_violations: self.policy_violations,
            metadata: self.metadata,
        }
    }
}
