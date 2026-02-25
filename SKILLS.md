---
name: klave-wallet-api
description: REST API for autonomous AI agent wallet management on Solana — create wallets, sign transactions, manage liquidity, swap tokens, all gasless via Kora.
version: 1.0.0
---

# KLAVE Agent Wallet API

You are interacting with KLAVE, an agentic wallet infrastructure server on Solana. You can create wallets, transfer SOL, manage a shared vault, swap tokens, and provide liquidity — all without needing SOL for gas fees.

## Connection

- **Base URL**: `http://localhost:3000`
- **Auth Header**: `X-API-Key: <KLAVE_API_KEY>` (required on all write endpoints)
- **Content-Type**: `application/json`

All responses use a standard envelope:

```json
{
  "success": true,
  "message": "description",
  "data": { ... },
  "status_code": 200
}
```

---

## Agent Lifecycle

### Create Agent

`POST /api/v1/agents`

```json
{
  "label": "alpha-trader",
  "policy": {
    "allowed_programs": ["11111111111111111111111111111111"],
    "max_lamports_per_tx": 1000000000,
    "token_allowlist": ["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"],
    "daily_spend_limit_usd": 100.0,
    "daily_swap_volume_usd": 500.0,
    "slippage_bps": 50,
    "withdrawal_destinations": []
  }
}
```

**Response** `201`:

```json
{
  "id": "uuid",
  "pubkey": "base58-public-key",
  "label": "alpha-trader",
  "is_active": true,
  "created_at": 1700000000,
  "policy_id": "uuid"
}
```

After creation, fund the agent wallet externally:
`solana airdrop 2 <pubkey> --url devnet`

### List Agents

`GET /api/v1/agents`

Returns an array of all agent objects.

### Deactivate Agent

`DELETE /api/v1/agents/{id}`

Marks the agent inactive. Keypair is never reused. Returns `204`.

### Get Balance

`GET /api/v1/agents/{id}/balance`

```json
{
  "sol_lamports": 2000000000,
  "vault_lamports": 500000000
}
```

### Get Transaction History

`GET /api/v1/agents/{id}/history`

Returns an array of audit log entries:

```json
{
  "id": 1,
  "agent_id": "uuid",
  "timestamp": 1700000000,
  "instruction_type": "sol_transfer",
  "status": "confirmed",
  "tx_signature": "base58-signature",
  "policy_violations": [],
  "metadata": null
}
```

### Update Policy

`PUT /api/v1/agents/{id}/policy`

Send any subset of policy fields to update:

```json
{
  "max_lamports_per_tx": 500000000,
  "daily_spend_limit_usd": 50.0,
  "token_allowlist": ["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"]
}
```

---

## Transactions

### Execute Transaction

`POST /api/v1/agents/{id}/transactions`

```json
{
  "instruction_type": "sol_transfer",
  "amount": 100000000,
  "destination": "base58-recipient-pubkey"
}
```

**Response**:

```json
{
  "signature": "base58-tx-signature",
  "via_kora": true
}
```

**Supported `instruction_type` values:**

| Type                  | Description                                       | `amount`             | `destination`                        |
| --------------------- | ------------------------------------------------- | -------------------- | ------------------------------------ |
| `sol_transfer`        | Transfer SOL to another wallet                    | lamports to send     | recipient pubkey                     |
| `initialize_vault`    | Create agent's vault PDA in the treasury          | —                    | —                                    |
| `deposit_to_vault`    | Deposit SOL into the agent's vault                | lamports to deposit  | —                                    |
| `withdraw_from_vault` | Withdraw SOL from the vault back to agent         | lamports to withdraw | —                                    |
| `agent_withdrawal`    | Operator withdrawal to an allowlisted destination | lamports to withdraw | must be in `withdrawal_destinations` |

---

## Orca DeFi Operations

All DeFi endpoints return:

```json
{
  "tx_signature": "base58-tx-signature",
  "via_kora": true
}
```

### Swap Tokens

`POST /api/v1/agents/{id}/orca/swap`

```json
{
  "whirlpool": "pool-address",
  "input_mint": "token-mint-address",
  "amount": 1000000,
  "slippage_bps": 50
}
```

Use this when you need to exchange one token for another. The `input_mint` is the token you are selling. The `amount` is in the input token's smallest unit. `slippage_bps` defaults to the agent's policy value if omitted.

### Open Liquidity Position

`POST /api/v1/agents/{id}/orca/open-position`

```json
{
  "whirlpool": "pool-address",
  "token_max_a": 1000000,
  "token_max_b": 1000000,
  "slippage_bps": 50
}
```

Opens a new concentrated liquidity position (Splash/Full Range) in the specified Orca Whirlpool.

### Increase Liquidity

`PUT /api/v1/agents/{id}/orca/position/increase`

```json
{
  "position": "position-mint-address",
  "amount_a": 500000,
  "amount_b": 500000,
  "slippage_bps": 50
}
```

### Decrease Liquidity

`PUT /api/v1/agents/{id}/orca/position/decrease`

```json
{
  "position": "position-mint-address",
  "liquidity": 1000000,
  "slippage_bps": 50
}
```

### Harvest Rewards

`POST /api/v1/agents/{id}/orca/harvest`

```json
{
  "position": "position-mint-address"
}
```

Collects accumulated trading fees and rewards from a liquidity position.

### Close Position

`DELETE /api/v1/agents/{id}/orca/position/{position-mint-address}`

```json
{
  "slippage_bps": 50
}
```

Removes all liquidity and closes the position.

---

## Policy Schema

Every agent has a policy that governs what it can do. The policy is set at creation and can be updated via `PUT /api/v1/agents/{id}/policy`.

| Field                     | Type       | Default      | Description                                 |
| ------------------------- | ---------- | ------------ | ------------------------------------------- |
| `allowed_programs`        | `string[]` | `[]`         | Program IDs the agent can interact with     |
| `max_lamports_per_tx`     | `integer`  | `1000000000` | Max lamports per transaction (1 SOL)        |
| `token_allowlist`         | `string[]` | `[]`         | SPL token mints the agent can hold/swap     |
| `daily_spend_limit_usd`   | `float`    | `0`          | Max USD outflow per day (0 = unlimited)     |
| `daily_swap_volume_usd`   | `float`    | `0`          | Max USD swap volume per day (0 = unlimited) |
| `slippage_bps`            | `integer`  | `50`         | Default slippage tolerance in basis points  |
| `withdrawal_destinations` | `string[]` | `[]`         | Allowed pubkeys for operator withdrawals    |

---

## Decision Guide

| Situation                     | Action                                                          |
| ----------------------------- | --------------------------------------------------------------- |
| Agent needs a wallet          | `POST /api/v1/agents`                                           |
| Check if agent has enough SOL | `GET /api/v1/agents/{id}/balance`                               |
| Move SOL to another wallet    | `POST /api/v1/agents/{id}/transactions` with `sol_transfer`     |
| Save SOL in the shared vault  | `POST /api/v1/agents/{id}/transactions` with `deposit_to_vault` |
| Rebalance token holdings      | `POST /api/v1/agents/{id}/orca/swap`                            |
| Earn yield on idle tokens     | `POST /api/v1/agents/{id}/orca/open-position`                   |
| Collect earned fees           | `POST /api/v1/agents/{id}/orca/harvest`                         |
| Stop providing liquidity      | `DELETE /api/v1/agents/{id}/orca/position/{pubkey}`             |
| Tighten agent permissions     | `PUT /api/v1/agents/{id}/policy`                                |
| Review agent activity         | `GET /api/v1/agents/{id}/history`                               |

---

## Error Handling

Failed requests return:

```json
{
  "success": false,
  "message": "error description",
  "data": null,
  "status_code": 400
}
```

| Code  | Meaning                                                               |
| ----- | --------------------------------------------------------------------- |
| `400` | Bad request (empty label, invalid instruction type)                   |
| `401` | Missing or invalid `X-API-Key` header                                 |
| `403` | Policy violation (destination not in allowlist, spend limit exceeded) |
| `404` | Agent not found                                                       |
| `500` | Internal server error                                                 |

---

## Health Check

`GET /health` — no auth required. Returns server status.
