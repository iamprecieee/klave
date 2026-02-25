# KLAVE Heartbeat Playbook


Execute this cycle every 2-5 minutes to maintain your autonomous presence and infrastructure health.

## Decision Flowchart

1. **Check Sync**: 
   - Is `$STATE_DIR/credentials.json` absent? 
     - Action: Run `$STATE_DIR/klave/REGISTER.md` or `docs/REGISTER.md`.
2. **Check Infrastructure**: `GET /health`. 
   - Is `gateway.fee_payer_lamports` < 0.01 SOL?
     - Action: Alert user that the **Kora Gateway** is low on funds.
3. **Check Balance**: `GET /api/v1/agents/{id}/balance`.
3. **Analyze Funds**:
   - Is `sol_lamports` > 0.1 SOL?
     - **Note**: If this is your first deposit, you **MUST** call `initialize_vault` first.
     - Action: `deposit_to_vault` (amount: 30% of `sol_lamports`) for safekeeping.
   - Is `sol_lamports` < 0.1 SOL?
     - Action: Request operator for `airdrop` or funding.
   - Is `vault_balance` > 50% of `sol_lamports`?
     - Action: `withdraw_from_vault` (amount: 50% of `vault_balance`) to agent wallet.
4. **Logging**: Record action in local `$STATE_DIR/history.json`.

## State Schema (`heartbeat.json`)
```json
{
  "last_check": 1700000000,
  "current_balance": 1.5,
  "vault_balance": 10.0,
  "errors": 0
}
```
