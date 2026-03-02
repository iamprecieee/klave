# KLAVE Heartbeat Playbook

Run this cycle every 2–5 minutes. The agent manages its own vault, rebalances tokens, and reports to the dashboard — all within policy.

This playbook is runtime-agnostic. Whether your agent is driven by OpenClaw, LangChain, cron, or a custom loop, the logic and API calls are the same.

## Rate Limits

60 requests / 60 seconds per API key. A typical cycle uses 3–6 requests. If you hit a `429`, wait 60 seconds.

---

## Flowchart

```
┌─ Check Health ─────────────── GET /health
│   └─ Fails? → log error, abort cycle
│
├─ Check Balance ────────────── GET /agents/{id}/balance
│   ├─ < 0.01 SOL → withdraw from vault if possible, alert operator
│   ├─ > 0.1 SOL  → vault not initialized? → initialize, then deposit 30%
│   ├─ > 0.05 SOL → swap 1/3 SOL → USDC
│   └─ vault > 90% of SOL balance → withdraw 50% of vault
│
├─ Token Rebalance ──────────── GET /agents/{id}/tokens
│   ├─ SOL > 0.05 → swap 1/3 → USDC
│   └─ SOL < USDC → swap 50% USDC → SOL
│
├─ Notify Dashboard ─────────── POST /agents/{id}/notify
│
└─ Update State ─────────────── write heartbeat.json (or equivalent)
```

---

## State Tracking

Track cycle results so your agent knows what it did last. For filesystem agents, write to `$KLAVE_STATE_DIR/heartbeat.json`:

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

If your runtime uses a database, env vars, or in-memory state, store the equivalent fields however makes sense.

---

## API Calls

### Health

```bash
GET /health
```

### SOL + Vault Balance

```bash
GET /api/v1/agents/{id}/balance
X-API-Key: <key>
# → { "sol_lamports": 200000000, "vault_lamports": 50000000 }
```

### Token Balances

```bash
GET /api/v1/agents/{id}/tokens
X-API-Key: <key>
# → [{ "mint": "...", "amount": 1000000, "decimals": 6, "ui_amount": 1.0 }]
```

### Vault Deposit

```bash
POST /api/v1/agents/{id}/transactions
Content-Type: application/json
X-API-Key: <key>

{ "instruction_type": "deposit_to_vault", "amount": 30000000 }
```

### Vault Withdrawal

```bash
POST /api/v1/agents/{id}/transactions
Content-Type: application/json
X-API-Key: <key>

{ "instruction_type": "withdraw_from_vault", "amount": 25000000 }
```

### Swap Tokens

```bash
POST /api/v1/agents/{id}/orca/swap
Content-Type: application/json
X-API-Key: <key>

{ "whirlpool": "<pool-address>", "input_mint": "So11...", "amount": 10000000 }
```

### Notify Dashboard

```bash
POST /api/v1/agents/{id}/notify
X-API-Key: <key>
# → 204 No Content
```

---

## Required Programs

| Action           | `allowed_programs`                                                                 |
| ---------------- | ---------------------------------------------------------------------------------- |
| SOL transfers    | `11111111111111111111111111111111`                                                 |
| Vault operations | `11111111111111111111111111111111`, `H2RojwyiyJ9CqTPoP1SynmutevCfq7YGskwcoPj1C7Ex` |
| Token swaps      | `11111111111111111111111111111111`, `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`  |

---

## Error Handling

**403 Policy Violation** — the transaction touched a program or destination not in the agent's allowlist.

1. Extract the offending ID from the error.
2. Record the violation in `errors` count.
3. Stop that action until an operator updates the policy via `PUT /agents/{id}/policy`.

**429 Rate Limited** — back off for 60 seconds, increment error count.

**5xx Server Error** — log and retry next cycle.

---

## Operator Alerts

Alert when:

- Balance drops below 0.01 SOL
- Policy violation blocks a required action
- 3+ consecutive cycle errors
- Vault operation fails on-chain

Skip alerts for routine deposits, successful swaps, and passing balance checks.
