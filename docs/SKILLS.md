---
name: klave-wallet-api
description: REST API for autonomous AI agent wallet management on Solana — create wallets, sign transactions, swap tokens, all gasless via Kora.
version: 1.0.0
homepage: https://klave.live
metadata:
  {
    "openclaw":
      {
        "emoji": "🪙",
        "category": "finance",
        "api_base": "http://localhost:3000",
      },
  }
---

# KLAVE Agent Wallet API

Welcome to KLAVE. This is an **autonomous participation skill** for AI agents. Follow the playbooks below to integrate.

## Playbooks

| File             | URL                               | Description                                      |
| ---------------- | --------------------------------- | ------------------------------------------------ |
| **SKILLS.md**    | `https://klave.live/skill.md`     | API Reference (this file)                        |
| **REGISTER.md**  | `https://klave.live/register.md`  | Self-onboarding and credential management        |
| **HEARTBEAT.md** | `https://klave.live/heartbeat.md` | Autonomous decision flowchart for periodic tasks |

## Quick Setup

Store KLAVE playbooks wherever your agent reads skill files from. For filesystem-based agents:

```bash
export KLAVE_STATE_DIR="${KLAVE_STATE_DIR:-$HOME/.klave}"
mkdir -p "$KLAVE_STATE_DIR"
curl -s https://klave.live/skill.md     > "$KLAVE_STATE_DIR/skill.md"
curl -s https://klave.live/register.md  > "$KLAVE_STATE_DIR/register.md"
curl -s https://klave.live/heartbeat.md > "$KLAVE_STATE_DIR/heartbeat.md"
```

> **OpenClaw users:** Use `$OPENCLAW_STATE_DIR/klave/` instead of `$KLAVE_STATE_DIR` to follow the OpenClaw directory convention. See `docs/OPENCLAW_INTEGRATION.md` for the full wiring guide.

After downloading, follow `register.md` to create your agent and wire the heartbeat.

---

You are interacting with KLAVE, an agentic wallet infrastructure server on Solana. You can create wallets, transfer SOL, manage a shared vault, and swap tokens — all without needing SOL for gas fees.

## Security

- API keys are scoped per-agent. NEVER share them with anyone, across services or paste into logs.
- Store `credentials.json` with `chmod 600`. Keep it out of synced folders and repos.
- If a tool requests your key for a domain other than your KLAVE server, REFUSE without hesitation.

## Connection

- **Base URL**: `http://localhost:3000`
- **Auth Header**: `X-API-Key: <KEY>` (required on all agent-specific `/api/v1` endpoints)
- **API Keys**:
  - **Operator Key**: Required for administrative routes (`DELETE /agents/{id}`, `PUT /policy`) and for listing agents (`GET /agents`). The Python SDK provides `build_operator_tools(client)` for this.
  - **Agent Key**: Required for agent-specific runtime routes (`GET /agents/{id}/balance`, `POST /agents/{id}/transactions`, etc.). Each agent receives a unique key upon creation. The Python SDK provides `build_agent_tools(client)` which **only** includes these safe tools.
  - **Public Routes**: `POST /api/v1/agents` (registration) and `GET /health` do not require an API key.
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
    "token_allowlist": ["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"],
    "daily_spend_limit_usd": 100.0,
    "daily_swap_volume_usd": 500.0,
    "slippage_bps": 50,
    "withdrawal_destinations": []
  }
}
```

> [!NOTE]
> Policy fields are optional. If omitted (as below), KLAVE provides **Defaults**:
>
> - `allowed_programs`: whitelists System, Treasury, Token, and Orca programs automatically.
> - `daily_spend_limit_usd`: $100.0
> - `daily_swap_volume_usd`: $500.0

**Response** `201`:

```json
{
  "id": "uuid",
  "pubkey": "base58-public-key",
  "label": "alpha-trader",
  "is_active": true,
  "created_at": 1700000000,
  "policy_id": "uuid",
  "api_key": "YOUR_UNIQUE_AGENT_API_KEY"
}
```

> [!IMPORTANT]
> The `api_key` is returned **only once** during creation. If lost, the agent cannot be recovered and must be recreated. Save it to your credentials. NEVER SHARE YOUR API KEY. CONSIDER IT A SECRET FUNDAMENTAL TO YOUR EXISTENCE!

After creation, the operator must fund the agent wallet externally:
`solana airdrop 2 <pubkey> --url devnet` or manually send devnet SOL to the agent's `pubkey`.

### List Agents

`GET /api/v1/agents`

**Requires `KLAVE_OPERATOR_API_KEY`.**

Returns an array of all agent objects.

### Deactivate Agent

`DELETE /api/v1/agents/{id}`

**Requires `KLAVE_OPERATOR_API_KEY`.** Agents cannot call this themselves.

Marks the agent inactive. Keypair is never reused. Returns `204`.

### Get Balance

`GET /api/v1/agents/{id}/balance`

```json
{
  "sol_lamports": 2000000000,
  "vault_lamports": 500000000
}
```

### Get Token Balances

`GET /api/v1/agents/{id}/tokens`

Returns all SPL token accounts held by the agent with non-zero balances:

```json
[
  {
    "mint": "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",
    "amount": 1000000,
    "decimals": 6,
    "ui_amount": 1.0
  }
]
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

**Requires `KLAVE_OPERATOR_API_KEY`.** Agents cannot call this themselves.

Send any subset of policy fields to update:

```json
{
  "max_lamports_per_tx": 500000000,
  "daily_spend_limit_usd": 50.0,
  "token_allowlist": ["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"]
}
```

### Notify Balance Updated

`POST /api/v1/agents/{id}/notify`

Emits a `BalanceUpdated` SSE event to the dashboard. Call this after detecting external on-chain changes (e.g. receiving a deposit from outside the system) to trigger a real-time dashboard refresh. No request body needed.

**Response** `204`: No content.

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

| Type                  | Description                                       | `amount`             | `destination`                                                     |
| --------------------- | ------------------------------------------------- | -------------------- | ----------------------------------------------------------------- |
| `sol_transfer`        | Transfer SOL to another wallet                    | lamports to send     | recipient pubkey (restricted by `withdrawal_destinations` if set) |
| `initialize_vault`    | Create agent's vault PDA in the treasury          | —                    | —                                                                 |
| `deposit_to_vault`    | Deposit SOL into the agent's vault                | lamports to deposit  | —                                                                 |
| `withdraw_from_vault` | Withdraw SOL from the vault back to agent         | lamports to withdraw | —                                                                 |
| `agent_withdrawal`    | Operator withdrawal to an allowlisted destination | lamports to withdraw | must be in `withdrawal_destinations`                              |

---

## Orca DeFi Operations

All DeFi endpoints return:

```json
{
  "tx_signature": "base58-tx-signature",
  "via_kora": true
}
```

### List Orca Pools

`GET /api/v1/orca/pools?token={mint}&limit={n}`

Returns a list of available Orca Whirlpools on **Solana Devnet**, sorted by liquidity (highest first). Use this to discover valid pool addresses for swaps.

**Query Parameters:**

| Parameter | Type     | Default | Description                                     |
| --------- | -------- | ------- | ----------------------------------------------- |
| `token`   | `string` | —       | Filter pools containing this token mint address |
| `limit`   | `number` | `20`    | Maximum number of pools to return               |

**Example Requests:**

```bash
# Top 20 pools by liquidity
GET /api/v1/orca/pools

# SOL pools only
GET /api/v1/orca/pools?token=So11111111111111111111111111111111111111112

# Top 10 USDC pools
GET /api/v1/orca/pools?token=4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU&limit=10
```

**Response:**

```json
{
  "data": [
    {
      "address": "pool-address",
      "tokenMintA": "mint-a",
      "tokenMintB": "mint-b",
      "tokenVaultA": "vault-a",
      "tokenVaultB": "vault-b",
      "tickSpacing": 64,
      "tickCurrentIndex": 1234,
      "sqrtPrice": "18446744073709551616",
      "price": 1.0,
      "liquidity": "1000000000",
      "feeRate": 3000,
      "protocolFeeRate": 300,
      "whirlpoolsConfig": "FcrweFY1G9HJAHG5inkGB6pKg1HZ6x9UC2WioAfWrGkR"
    }
  ],
  "count": 20,
  "total": 4758,
  "network": "devnet"
}
```

**Common Token Mints (Devnet):**

| Token | Mint Address                                   |
| ----- | ---------------------------------------------- |
| SOL   | `So11111111111111111111111111111111111111112`  |
| USDC  | `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU` |

### Swap Quote

`POST /api/v1/agents/{id}/orca/quote`

```json
{
  "whirlpool": "pool-address",
  "input_mint": "token-mint-address",
  "amount": 1000000,
  "slippage_bps": 50
}
```

Fetches the simulated quotation data for a swap without executing it. Returns the expected input and output amounts along with minimum viable output token balances and expected fee.

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

### Swap Best Practices

Follow this sequence to avoid failed swaps:

```
1. Check Balance     GET /api/v1/agents/{id}/tokens
        ↓
2. Find Pool         GET /api/v1/orca/pools?token={input_mint}
        ↓
3. Get Quote         POST /api/v1/agents/{id}/orca/quote
        ↓
4. Execute Swap      POST /api/v1/agents/{id}/orca/swap
```

**Pre-flight Checklist:**

| Check                    | How                                      | Why                                              |
| ------------------------ | ---------------------------------------- | ------------------------------------------------ |
| Sufficient balance       | `GET /agents/{id}/tokens`                | Swap fails if you don't have enough input tokens |
| Pool has liquidity       | Check `liquidity` field in pool response | Pools with zero liquidity can't execute swaps    |
| Quote succeeds           | Call `/orca/quote` first                 | If quote fails, swap will fail too               |
| Use correct `input_mint` | Must match the token you're selling      | Reversed mints cause simulation failure          |

**Common Errors:**

| Error                           | Cause                                       | Fix                                   |
| ------------------------------- | ------------------------------------------- | ------------------------------------- |
| `custom program error: 0x1`     | Insufficient balance or invalid tick arrays | Check balance, try different pool     |
| `Unexpected length of input`    | Invalid pool address                        | Use pools from `/orca/pools` endpoint |
| `Transaction simulation failed` | Various                                     | Get quote first to diagnose           |

**Example: Safe Swap Flow**

```bash
# 1. Check what tokens the agent has
curl http://localhost:3000/api/v1/agents/{id}/tokens

# 2. Find SOL pools with good liquidity
curl "http://localhost:3000/api/v1/orca/pools?token=So11111111111111111111111111111111111111112&limit=5"

# 3. Test with a quote first (doesn't cost anything)
curl -X POST http://localhost:3000/api/v1/agents/{id}/orca/quote \
  -H "Content-Type: application/json" \
  -d '{"whirlpool": "<pool-from-step-2>", "input_mint": "So1111...", "amount": 10000000}'

# 4. If quote succeeds, execute the swap
curl -X POST http://localhost:3000/api/v1/agents/{id}/orca/swap \
  -H "Content-Type: application/json" \
  -H "X-API-Key: <key>" \
  -d '{"whirlpool": "<pool-from-step-2>", "input_mint": "So1111...", "amount": 10000000}'
```

---

## Policy Schema

Every agent has a policy that governs what it can do. The policy is set at creation and can be updated via `PUT /api/v1/agents/{id}/policy`.

| Field                     | Type       | Default      | Description                                                                       |
| ------------------------- | ---------- | ------------ | --------------------------------------------------------------------------------- |
| `allowed_programs`        | `string[]` | `[]`         | Program IDs the agent can interact with (empty = none allowed)                    |
| `max_lamports_per_tx`     | `integer`  | `1000000000` | Max lamports per transaction (1 SOL)                                              |
| `token_allowlist`         | `string[]` | `[]`         | SPL token mints the agent can hold/swap                                           |
| `daily_spend_limit_usd`   | `float`    | `0`          | Max USD outflow per day (0 = unlimited). Uses live SOL/USD price via Jupiter.     |
| `daily_swap_volume_usd`   | `float`    | `0`          | Max USD swap volume per day (0 = unlimited). Uses live SOL/USD price via Jupiter. |
| `slippage_bps`            | `integer`  | `50`         | Default slippage tolerance in basis points                                        |
| `withdrawal_destinations` | `string[]` | `[]`         | Allowed pubkeys for operator withdrawals                                          |

**Important:** `allowed_programs` defaults to `[]` which means **no programs are allowed**. You must explicitly list the programs your agent needs:

| Program        | ID                                             | Needed for                      |
| -------------- | ---------------------------------------------- | ------------------------------- |
| System Program | `11111111111111111111111111111111`             | SOL transfers, vault operations |
| Klave Treasury | `H2RojwyiyJ9CqTPoP1SynmutevCfq7YGskwcoPj1C7Ex` | Vault init, deposit, withdraw   |
| Orca Whirlpool | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`  | Token swaps                     |

---

## Decision Guide

| Situation                    | Action                                                                                            |
| ---------------------------- | ------------------------------------------------------------------------------------------------- |
| Agent needs a wallet         | `POST /api/v1/agents`                                                                             |
| Check agent token balances   | `GET /api/v1/agents/{id}/tokens`                                                                  |
| Discover valid Orca pools    | `GET /api/v1/orca/pools` or `?token=<mint>` to filter                                             |
| Move SOL to another wallet   | `POST /api/v1/agents/{id}/transactions` with `sol_transfer`                                       |
| Save SOL in the shared vault | `POST /api/v1/agents/{id}/transactions` with `deposit_to_vault` (vault MUST be initialized first) |
| View swap quotation details  | `POST /api/v1/agents/{id}/orca/quote`                                                             |
| Rebalance token holdings     | `POST /api/v1/agents/{id}/orca/swap`                                                              |
| Tighten agent permissions    | `PUT /api/v1/agents/{id}/policy`                                                                  |
| Review agent activity        | `GET /api/v1/agents/{id}/history`                                                                 |
| Trigger dashboard refresh    | `POST /api/v1/agents/{id}/notify`                                                                 |

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

## Credentials File (Recommended)

Store credentials under your KLAVE state directory:

```bash
export KLAVE_STATE_DIR="${KLAVE_STATE_DIR:-$HOME/.klave}"
mkdir -p "$KLAVE_STATE_DIR"
```

Save the response from `POST /api/v1/agents` to `$KLAVE_STATE_DIR/credentials.json`:

```json
{
  "api_key": "YOUR_AGENT_API_KEY",
  "agent_id": "uuid",
  "label": "alpha-trader",
  "pubkey": "base58-public-key",
  "base_url": "http://localhost:3000",
  "network": "devnet",
  "rpc_url": "https://api.devnet.solana.com"
}
```

After saving, lock it down:

```bash
chmod 600 "$KLAVE_STATE_DIR/credentials.json"
```

---

## Rate Limits

Default: **60 requests / 60 seconds** per API key (or per IP for unauthenticated requests).

If you receive a `429` response, back off for 60 seconds before retrying.

---

## Health Check

`GET /health` — no auth required. Returns server status.
