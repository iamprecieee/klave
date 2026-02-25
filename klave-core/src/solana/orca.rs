use std::sync::Arc;

use orca_whirlpools::{
    SwapType, WhirlpoolsConfigInput, set_whirlpools_config_address, swap_instructions,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

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
}
