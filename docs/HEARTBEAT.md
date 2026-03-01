# KLAVE Heartbeat Playbook

Execute this cycle every 2-5 minutes to maintain your autonomous presence and infrastructure health.

## Decision Flowchart

```
1. Check Sync
   └─ credentials.json missing? → Run docs/REGISTER.md
          │
2. Check Infrastructure
   └─ GET /health fails? → Alert operator, abort cycle
          │
3. Check Balance
   └─ GET /api/v1/agents/{id}/balance
          │
4. Analyze Funds
   ├─ sol_lamports > 0.1 SOL?
   │     └─ Vault not initialized? → POST transactions { instruction_type: "initialize_vault" }
   │     └─ Deposit 30% into vault → POST transactions { instruction_type: "deposit_to_vault", amount: ... }
   ├─ sol_lamports > 0.05 SOL?
   │     └─ Swap 1/3 of SOL balance into USDC
   ├─ sol_lamports < 0.01 SOL?
   │     └─ Withdraw 30% from vault → POST transactions { instruction_type: "withdraw_from_vault", amount: ... }
   ├─ vault_lamports > 90% of sol_lamports?
   │     └─ Withdraw 50% of vault → POST transactions { instruction_type: "withdraw_from_vault", amount: ... }
   └─ (Else) → Proceed to Token Rebalancing
          │
5. Check Token Positions
   └─ GET /api/v1/agents/{id}/tokens
   └─ Rebalance needed? (Strategy: Core Stability)
         ├─ SOL balance > 0.05 SOL? → Swap 1/3 of SOL balance into USDC
         ├─ SOL balance < USDC balance? → Swap 50% of USDC back to SOL
         └─ Action Cycle: Discover Pool → Get Quote → Swap
          │
6. Log
   └─ Record action in $STATE_DIR/heartbeat.json
```

## API Calls

### Check health

```bash
GET /health
```

### Check SOL + vault balance

```bash
GET /api/v1/agents/{id}/balance
X-API-Key: <key>
# → { "sol_lamports": 200000000, "vault_lamports": 50000000 }
```

### Check token balances

```bash
GET /api/v1/agents/{id}/tokens
X-API-Key: <key>
# → [{ "mint": "...", "amount": 1000000, "decimals": 6, "ui_amount": 1.0 }]
```

### Deposit to vault

```bash
POST /api/v1/agents/{id}/transactions
Content-Type: application/json
X-API-Key: <key>

{ "instruction_type": "deposit_to_vault", "amount": 30000000 }
```

### Withdraw from vault

```bash
POST /api/v1/agents/{id}/transactions
Content-Type: application/json
X-API-Key: <key>

{ "instruction_type": "withdraw_from_vault", "amount": 25000000 }
```

### Swap tokens

```bash
POST /api/v1/agents/{id}/orca/swap
Content-Type: application/json
X-API-Key: <key>

{ "whirlpool": "<pool-address>", "input_mint": "So11...", "amount": 10000000 }
```

### Notify dashboard (after external deposits)

```bash
POST /api/v1/agents/{id}/notify
X-API-Key: <key>
# → 204 No Content — triggers dashboard refresh
```

## Prerequisites

Your agent's policy must include the programs needed for each action:

| Action           | Required `allowed_programs`                                                        |
| ---------------- | ---------------------------------------------------------------------------------- |
| SOL transfers    | `11111111111111111111111111111111`                                                 |
| Vault operations | `11111111111111111111111111111111`, `H2RojwyiyJ9CqTPoP1SynmutevCfq7YGskwcoPj1C7Ex` |
| Token swaps      | `11111111111111111111111111111111`, `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`  |

## Error Handling

If an action returns a **403 Policy Violation**, the agent must:

1.  **Identify**: Extract the missing program ID or destination from the error message (e.g., `program not in allowlist: GCU8...`).
2.  **Alert**: Record the violation in the `errors` count in `heartbeat.json` and notify the operator system.
3.  **Halt**: Suspend this part of the cycle until an operator updates the policy via the **Operator API**.

## State Schema (`heartbeat.json`)

```json
{
  "last_check": 1700000000,
  "sol_lamports": 200000000,
  "vault_lamports": 50000000,
  "token_count": 1,
  "action_taken": "deposit_to_vault",
  "errors": 0
}
```
