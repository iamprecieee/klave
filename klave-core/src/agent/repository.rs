use uuid::Uuid;

use chrono::Utc;
use rand::distr::{Alphanumeric, SampleString};
use sha2::{Digest, Sha256};
use solana_sdk::signature::{Keypair, Signer};
use sqlx::SqlitePool;

use crate::{
    agent::model::{Agent, AgentPolicy, AgentPolicyInput, CreateAgentRequest, default_programs},
    crypto,
    error::{KlaveError, Result},
};

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

    fn generate_api_key() -> String {
        Alphanumeric.sample_string(&mut rand::rng(), 32)
    }

    fn hash_api_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub async fn create(&self, req: &CreateAgentRequest) -> Result<Agent> {
        let agent_id = Uuid::new_v4().to_string();
        let policy_id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        let api_key = Self::generate_api_key();
        let api_key_hash = Self::hash_api_key(&api_key);
        let encrypted_api_key = crypto::encrypt(api_key.as_bytes(), &self.encryption_key)?;

        let keypair = Keypair::new();
        let pubkey = keypair.pubkey().to_string();
        let encrypted_keypair = crypto::encrypt(&keypair.to_bytes(), &self.encryption_key)?;

        let mut allowed_programs = req.policy.allowed_programs.clone();
        if allowed_programs.is_empty() {
            allowed_programs = default_programs();
        }

        let mut daily_spend_limit_usd = req.policy.daily_spend_limit_usd;
        if daily_spend_limit_usd == 0.0 {
            daily_spend_limit_usd = 100.0;
        }

        let mut daily_swap_volume_usd = req.policy.daily_swap_volume_usd;
        if daily_swap_volume_usd == 0.0 {
            daily_swap_volume_usd = 500.0;
        }

        let allowed_programs_json = serde_json::to_string(&allowed_programs)?;
        let token_allowlist_json = serde_json::to_string(&req.policy.token_allowlist)?;
        let withdrawal_destinations_json =
            serde_json::to_string(&req.policy.withdrawal_destinations)?;

        sqlx::query(
            "INSERT INTO agents (id, pubkey, label, is_active, created_at, policy_id, encrypted_keypair, api_key_hash, encrypted_api_key) \
             VALUES (?, ?, ?, 1, ?, ?, ?, ?, ?)",
        )
        .bind(&agent_id)
        .bind(&pubkey)
        .bind(&req.label)
        .bind(now)
        .bind(&policy_id)
        .bind(&encrypted_keypair)
        .bind(&api_key_hash)
        .bind(&encrypted_api_key)
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
        .bind(daily_spend_limit_usd)
        .bind(daily_swap_volume_usd)
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
            api_key: Some(api_key),
        })
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<Agent>> {
        let row = sqlx::query_as::<_, AgentRow>(
            "SELECT id, pubkey, label, is_active, created_at, policy_id \
             FROM agents WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(AgentRow::into_agent))
    }

    pub async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Agent>> {
        let rows = sqlx::query_as::<_, AgentRow>(
            "SELECT id, pubkey, label, is_active, created_at, policy_id FROM agents ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(AgentRow::into_agent).collect())
    }

    pub async fn deactivate(&self, id: &str) -> Result<()> {
        let result = sqlx::query("UPDATE agents SET is_active = 0 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(KlaveError::AgentNotFound(id.to_string()));
        }

        Ok(())
    }

    pub async fn find_policy(&self, agent_id: &str) -> Result<Option<AgentPolicy>> {
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
    ) -> Result<AgentPolicy> {
        let now = Utc::now().timestamp();
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

    pub async fn get_keypair(&self, id: &str) -> Result<Vec<u8>> {
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

    pub async fn verify_agent_key(&self, agent_id: &str, api_key: &str) -> Result<bool> {
        let hash = Self::hash_api_key(api_key);
        let row: Option<(String,)> =
            sqlx::query_as("SELECT id FROM agents WHERE id = ? AND api_key_hash = ?")
                .bind(agent_id)
                .bind(hash)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.is_some())
    }

    pub async fn find_by_key_hash(&self, api_key: &str) -> Result<Option<Agent>> {
        let hash = Self::hash_api_key(api_key);
        let row = sqlx::query_as::<_, AgentRow>(
            "SELECT id, pubkey, label, is_active, created_at, policy_id FROM agents WHERE api_key_hash = ?",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(AgentRow::into_agent))
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
            api_key: None, // API key should not be retrieved from DB in plain text
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
    fn into_policy(self) -> Result<AgentPolicy> {
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
