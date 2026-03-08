use std::{str::FromStr, sync::Arc};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use orca_whirlpools::{
    SwapType, WhirlpoolsConfigInput, set_whirlpools_config_address, swap_instructions,
};
use orca_whirlpools_client::Whirlpool;
use solana_account_decoder::{UiAccountData, UiAccountEncoding};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair};

use crate::{
    agent::model::SwapQuote,
    error::{KlaveError, Result},
};

const WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
const DEVNET_WHIRLPOOLS_CONFIG: &str = "FcrweFY1G9HJAHG5inkGB6pKg1HZ6x9UC2WioAfWrGkR";
const WHIRLPOOL_ACCOUNT_SIZE: u64 = 653;

pub struct OrcaInstructionResult {
    pub instructions: Vec<Instruction>,
    pub additional_signers: Vec<Keypair>,
}

pub struct OrcaClient {
    rpc_client: Arc<RpcClient>,
}

impl OrcaClient {
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        let _ = set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet);

        Self { rpc_client }
    }

    #[tracing::instrument(skip(self))]
    pub async fn swap(
        &self,
        whirlpool: Pubkey,
        amount: u64,
        input_mint: Pubkey,
        swap_type: SwapType,
        slippage_bps: u16,
        funder: Pubkey,
    ) -> Result<OrcaInstructionResult> {
        let result = swap_instructions(
            &self.rpc_client,
            whirlpool,
            amount,
            input_mint,
            swap_type,
            Some(slippage_bps),
            Some(funder),
        )
        .await
        .map_err(|e| KlaveError::Internal(format!("Orca swap error: {}", e)))?;

        Ok(OrcaInstructionResult {
            instructions: result.instructions,
            additional_signers: result.additional_signers,
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn fetch_quote(
        &self,
        whirlpool: Pubkey,
        amount: u64,
        input_mint: Pubkey,
        swap_type: SwapType,
        slippage_bps: u16,
        funder: Option<Pubkey>,
    ) -> Result<SwapQuote> {
        // Simulate the swap and gets the quote.
        let result = swap_instructions(
            &self.rpc_client,
            whirlpool,
            amount,
            input_mint,
            swap_type.clone(),
            Some(slippage_bps),
            funder,
        )
        .await
        .map_err(|e| KlaveError::Internal(format!("Orca quote error: {}", e)))?;

        let (input_amount, output_amount, min_output_amount, fee_amount) = match result.quote {
            orca_whirlpools::SwapQuote::ExactIn(x) => {
                (x.token_in, x.token_est_out, x.token_min_out, x.trade_fee)
            }
            orca_whirlpools::SwapQuote::ExactOut(x) => {
                (x.token_est_in, x.token_out, x.token_out, x.trade_fee)
            }
        };

        Ok(SwapQuote {
            input_amount,
            output_amount,
            min_output_amount,
            price_impact_bps: 0, // Quote doesn't directly expose price impact
            fee_amount,
        })
    }

    /// Lists Whirlpool accounts from devnet by querying the program directly.
    /// This replaces the mainnet REST API since Orca has no devnet API equivalent.
    ///
    /// # Arguments
    /// * `token_filter` - Optional token mint to filter pools (returns pools containing this token)
    /// * `limit` - Maximum number of pools to return (default 20, sorted by liquidity desc)
    #[tracing::instrument(skip(self))]
    pub async fn list_pools(
        &self,
        token_filter: Option<String>,
        limit: Option<usize>,
    ) -> Result<serde_json::Value> {
        let limit = limit.unwrap_or(20);
        let whirlpool_program_id = Pubkey::from_str(WHIRLPOOL_PROGRAM_ID)
            .map_err(|e| KlaveError::Internal(format!("Invalid program ID: {}", e)))?;

        // Whirlpool account discriminator (first 8 bytes of sha256("account:Whirlpool"))
        let whirlpool_discriminator: [u8; 8] = [63, 149, 209, 12, 225, 128, 99, 9];

        // Devnet whirlpools config pubkey bytes (at offset 8, after discriminator)
        let devnet_config = Pubkey::from_str(DEVNET_WHIRLPOOLS_CONFIG)
            .map_err(|e| KlaveError::Internal(format!("Invalid devnet config: {}", e)))?;

        let filters = vec![
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, whirlpool_discriminator.to_vec())),
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(8, devnet_config.to_bytes().to_vec())),
            RpcFilterType::DataSize(WHIRLPOOL_ACCOUNT_SIZE),
        ];

        let config = RpcProgramAccountsConfig {
            filters: Some(filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self
            .rpc_client
            .get_program_ui_accounts_with_config(&whirlpool_program_id, config)
            .await
            .map_err(|e| {
                let err = format!("Failed to fetch whirlpool accounts: {}", e);
                tracing::error!("{}", err);
                KlaveError::Internal(err)
            })?;

        let mut pools: Vec<(u128, serde_json::Value)> = Vec::new();

        for (pubkey, account) in accounts {
            let data_bytes = match &account.data {
                UiAccountData::Binary(b64_data, UiAccountEncoding::Base64) => {
                    match BASE64.decode(b64_data) {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            tracing::warn!("Failed to decode base64 for {}: {}", pubkey, e);
                            continue;
                        }
                    }
                }
                _ => {
                    tracing::warn!("Unexpected account data encoding for {}", pubkey);
                    continue;
                }
            };

            match Whirlpool::from_bytes(&data_bytes) {
                Ok(whirlpool) => {
                    if whirlpool.liquidity == 0 {
                        continue;
                    }

                    let sqrt_price_x64 = whirlpool.sqrt_price;
                    let price = if sqrt_price_x64 > 0 {
                        let sqrt_price_f64 = sqrt_price_x64 as f64 / (1u128 << 64) as f64;
                        sqrt_price_f64 * sqrt_price_f64
                    } else {
                        0.0
                    };

                    let token_a = whirlpool.token_mint_a.to_string();
                    let token_b = whirlpool.token_mint_b.to_string();

                    // Apply token filter if specified
                    if let Some(ref filter) = token_filter
                        && &token_a != filter
                        && &token_b != filter
                    {
                        continue;
                    }

                    pools.push((
                        whirlpool.liquidity,
                        serde_json::json!({
                            "address": pubkey.to_string(),
                            "tokenMintA": token_a,
                            "tokenMintB": token_b,
                            "tokenVaultA": whirlpool.token_vault_a.to_string(),
                            "tokenVaultB": whirlpool.token_vault_b.to_string(),
                            "tickSpacing": whirlpool.tick_spacing,
                            "tickCurrentIndex": whirlpool.tick_current_index,
                            "sqrtPrice": sqrt_price_x64.to_string(),
                            "price": price,
                            "liquidity": whirlpool.liquidity.to_string(),
                            "feeRate": whirlpool.fee_rate,
                            "protocolFeeRate": whirlpool.protocol_fee_rate,
                            "whirlpoolsConfig": whirlpool.whirlpools_config.to_string(),
                        }),
                    ));
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize whirlpool account {}: {}", pubkey, e);
                }
            }
        }

        // Sort by liquidity descending
        pools.sort_by(|a, b| b.0.cmp(&a.0));

        // Take top N pools
        let total_found = pools.len();
        let pools: Vec<serde_json::Value> = pools.into_iter().take(limit).map(|(_, v)| v).collect();

        tracing::info!(
            "Found {} pools on devnet, returning top {} by liquidity",
            total_found,
            pools.len()
        );

        Ok(serde_json::json!({
            "data": pools,
            "count": pools.len(),
            "total": total_found,
            "network": "devnet"
        }))
    }
}
