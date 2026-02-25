use solana_sdk::signature::{Keypair, Signer};
use sqlx::SqlitePool;

use crate::agent::model::{Agent, AgentPolicy, AgentPolicyInput, CreateAgentRequest};
use crate::crypto;
use crate::error::KlaveError;

pub struct AgentRepository {
    pool: SqlitePool,
    encryption_key: [u8; 32],
}

impl AgentRepository {
    pub fn new(pool: SqlitePool, encryption_key: [u8; 32]) -> Self {
        Self {
            pool,
            encryption_key,
        }
    }

    pub async fn create(&self, req: &CreateAgentRequest) -> Result<Agent, KlaveError> {
        let agent_id = uuid::Uuid::new_v4().to_string();
        let policy_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        let keypair = Keypair::new();
        let pubkey = keypair.pubkey().to_string();
        let encrypted_keypair = crypto::encrypt(&keypair.to_bytes(), &self.encryption_key)?;

        let allowed_programs_json = serde_json::to_string(&req.policy.allowed_programs)?;
        let token_allowlist_json = serde_json::to_string(&req.policy.token_allowlist)?;
        let withdrawal_destinations_json =
            serde_json::to_string(&req.policy.withdrawal_destinations)?;

        sqlx::query(
            "INSERT INTO agents (id, pubkey, label, is_active, created_at, policy_id, encrypted_keypair) \
             VALUES (?, ?, ?, 1, ?, ?, ?)",
        )
        .bind(&agent_id)
        .bind(&pubkey)
        .bind(&req.label)
        .bind(now)
        .bind(&policy_id)
        .bind(&encrypted_keypair)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO agent_policies \
             (id, agent_id, allowed_programs, max_lamports_per_tx, token_allowlist, \
              daily_spend_limit_usd, daily_swap_volume_usd, slippage_bps, \
              withdrawal_destinations, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&policy_id)
        .bind(&agent_id)
        .bind(&allowed_programs_json)
        .bind(req.policy.max_lamports_per_tx)
        .bind(&token_allowlist_json)
        .bind(req.policy.daily_spend_limit_usd)
        .bind(req.policy.daily_swap_volume_usd)
        .bind(req.policy.slippage_bps)
        .bind(&withdrawal_destinations_json)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(Agent {
            id: agent_id,
            pubkey,
            label: req.label.clone(),
            is_active: true,
            created_at: now,
            policy_id,
        })
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<Agent>, KlaveError> {
        let row = sqlx::query_as::<_, AgentRow>(
            "SELECT id, pubkey, label, is_active, created_at, policy_id \
             FROM agents WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(AgentRow::into_agent))
    }

    pub async fn list_all(&self) -> Result<Vec<Agent>, KlaveError> {
        let rows = sqlx::query_as::<_, AgentRow>(
            "SELECT id, pubkey, label, is_active, created_at, policy_id FROM agents",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(AgentRow::into_agent).collect())
    }

    pub async fn deactivate(&self, id: &str) -> Result<(), KlaveError> {
        let result = sqlx::query("UPDATE agents SET is_active = 0 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(KlaveError::AgentNotFound(id.to_string()));
        }

        Ok(())
    }

    pub async fn find_policy(&self, agent_id: &str) -> Result<Option<AgentPolicy>, KlaveError> {
        let row = sqlx::query_as::<_, AgentPolicyRow>(
            "SELECT id, agent_id, allowed_programs, max_lamports_per_tx, token_allowlist, \
             daily_spend_limit_usd, daily_swap_volume_usd, slippage_bps, \
             withdrawal_destinations, updated_at \
             FROM agent_policies WHERE agent_id = ?",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.into_policy()?)),
            None => Ok(None),
        }
    }

    pub async fn update_policy(
        &self,
        agent_id: &str,
        input: &AgentPolicyInput,
    ) -> Result<AgentPolicy, KlaveError> {
        let now = chrono::Utc::now().timestamp();
        let allowed_programs_json = serde_json::to_string(&input.allowed_programs)?;
        let token_allowlist_json = serde_json::to_string(&input.token_allowlist)?;
        let withdrawal_destinations_json = serde_json::to_string(&input.withdrawal_destinations)?;

        let result = sqlx::query(
            "UPDATE agent_policies SET \
             allowed_programs = ?, max_lamports_per_tx = ?, token_allowlist = ?, \
             daily_spend_limit_usd = ?, daily_swap_volume_usd = ?, slippage_bps = ?, \
             withdrawal_destinations = ?, updated_at = ? \
             WHERE agent_id = ?",
        )
        .bind(&allowed_programs_json)
        .bind(input.max_lamports_per_tx)
        .bind(&token_allowlist_json)
        .bind(input.daily_spend_limit_usd)
        .bind(input.daily_swap_volume_usd)
        .bind(input.slippage_bps)
        .bind(&withdrawal_destinations_json)
        .bind(now)
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(KlaveError::PolicyNotFound(agent_id.to_string()));
        }

        self.find_policy(agent_id)
            .await?
            .ok_or_else(|| KlaveError::PolicyNotFound(agent_id.to_string()))
    }

    pub async fn get_keypair(&self, id: &str) -> Result<Vec<u8>, KlaveError> {
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT encrypted_keypair FROM agents WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some((blob,)) => crypto::decrypt(&blob, &self.encryption_key),
            None => Err(KlaveError::AgentNotFound(id.to_string())),
        }
    }
}

#[derive(sqlx::FromRow)]
struct AgentRow {
    id: String,
    pubkey: String,
    label: String,
    is_active: bool,
    created_at: i64,
    policy_id: String,
}

impl AgentRow {
    fn into_agent(self) -> Agent {
        Agent {
            id: self.id,
            pubkey: self.pubkey,
            label: self.label,
            is_active: self.is_active,
            created_at: self.created_at,
            policy_id: self.policy_id,
        }
    }
}

#[derive(sqlx::FromRow)]
struct AgentPolicyRow {
    id: String,
    agent_id: String,
    allowed_programs: String,
    max_lamports_per_tx: i64,
    token_allowlist: String,
    daily_spend_limit_usd: f64,
    daily_swap_volume_usd: f64,
    slippage_bps: i32,
    withdrawal_destinations: String,
    updated_at: i64,
}

impl AgentPolicyRow {
    fn into_policy(self) -> Result<AgentPolicy, KlaveError> {
        Ok(AgentPolicy {
            id: self.id,
            agent_id: self.agent_id,
            allowed_programs: serde_json::from_str(&self.allowed_programs)?,
            max_lamports_per_tx: self.max_lamports_per_tx,
            token_allowlist: serde_json::from_str(&self.token_allowlist)?,
            daily_spend_limit_usd: self.daily_spend_limit_usd,
            daily_swap_volume_usd: self.daily_swap_volume_usd,
            slippage_bps: self.slippage_bps,
            withdrawal_destinations: serde_json::from_str(&self.withdrawal_destinations)?,
            updated_at: self.updated_at,
        })
    }
}
