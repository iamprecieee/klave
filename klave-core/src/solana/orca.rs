use orca_whirlpools::{
    DecreaseLiquidityParam, IncreaseLiquidityParam, SwapType, WhirlpoolsConfigInput,
    close_position_instructions, decrease_liquidity_instructions, harvest_position_instructions,
    increase_liquidity_instructions, open_full_range_position_instructions,
    set_whirlpools_config_address, swap_instructions,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::sync::Arc;

use crate::error::KlaveError;

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

    pub async fn swap(
        &self,
        whirlpool: Pubkey,
        amount: u64,
        input_mint: Pubkey,
        swap_type: SwapType,
        slippage_bps: u16,
        funder: Pubkey,
    ) -> Result<OrcaInstructionResult, KlaveError> {
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

    pub async fn open_splash_position(
        &self,
        whirlpool: Pubkey,
        token_max_a: u64,
        token_max_b: u64,
        slippage_bps: u16,
        funder: Pubkey,
    ) -> Result<OrcaInstructionResult, KlaveError> {
        let param = IncreaseLiquidityParam {
            token_max_a,
            token_max_b,
        };
        let result = open_full_range_position_instructions(
            &self.rpc_client,
            whirlpool,
            param,
            Some(slippage_bps),
            Some(funder),
        )
        .await
        .map_err(|e| KlaveError::Internal(format!("Orca open splash position error: {}", e)))?;

        Ok(OrcaInstructionResult {
            instructions: result.instructions,
            additional_signers: result.additional_signers,
        })
    }

    pub async fn increase_liquidity(
        &self,
        position_mint: Pubkey,
        token_max_a: u64,
        token_max_b: u64,
        slippage_bps: u16,
        funder: Pubkey,
    ) -> Result<OrcaInstructionResult, KlaveError> {
        let param = IncreaseLiquidityParam {
            token_max_a,
            token_max_b,
        };
        let result = increase_liquidity_instructions(
            &self.rpc_client,
            position_mint,
            param,
            Some(slippage_bps),
            Some(funder),
        )
        .await
        .map_err(|e| KlaveError::Internal(format!("Orca increase liquidity error: {}", e)))?;

        Ok(OrcaInstructionResult {
            instructions: result.instructions,
            additional_signers: result.additional_signers,
        })
    }

    pub async fn decrease_liquidity(
        &self,
        position_mint: Pubkey,
        liquidity: u128,
        slippage_bps: u16,
        funder: Pubkey,
    ) -> Result<OrcaInstructionResult, KlaveError> {
        let param = DecreaseLiquidityParam::Liquidity(liquidity);
        let result = decrease_liquidity_instructions(
            &self.rpc_client,
            position_mint,
            param,
            Some(slippage_bps),
            Some(funder),
        )
        .await
        .map_err(|e| KlaveError::Internal(format!("Orca decrease liquidity error: {}", e)))?;

        Ok(OrcaInstructionResult {
            instructions: result.instructions,
            additional_signers: result.additional_signers,
        })
    }

    pub async fn harvest_position(
        &self,
        position_mint: Pubkey,
        funder: Pubkey,
    ) -> Result<OrcaInstructionResult, KlaveError> {
        let result = harvest_position_instructions(&self.rpc_client, position_mint, Some(funder))
            .await
            .map_err(|e| KlaveError::Internal(format!("Orca harvest error: {}", e)))?;

        Ok(OrcaInstructionResult {
            instructions: result.instructions,
            additional_signers: result.additional_signers,
        })
    }

    pub async fn close_position(
        &self,
        position_mint: Pubkey,
        slippage_bps: u16,
        funder: Pubkey,
    ) -> Result<OrcaInstructionResult, KlaveError> {
        let result = close_position_instructions(
            &self.rpc_client,
            position_mint,
            Some(slippage_bps),
            Some(funder),
        )
        .await
        .map_err(|e| KlaveError::Internal(format!("Orca close position error: {}", e)))?;

        Ok(OrcaInstructionResult {
            instructions: result.instructions,
            additional_signers: result.additional_signers,
        })
    }
}
