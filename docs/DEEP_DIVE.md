# KLAVE — Deep Dive

A technical deep dive into the design philosophy, security model, and agent interaction patterns behind KLAVE: agentic wallet infrastructure for Solana.

---

## Table of Contents

1. [Design Philosophy](#design-philosophy)
2. [Wallet Architecture](#wallet-architecture)
3. [Agent-Wallet Interaction Model](#agent-wallet-interaction-model)
4. [Security Model](#security-model)
5. [Multi-Agent Isolation](#multi-agent-isolation)
6. [Policy Engine](#policy-engine)
7. [Transaction Pipeline](#transaction-pipeline)
8. [DeFi Integration (Orca Whirlpools)](#defi-integration-orca-whirlpools)
9. [Audit Trail](#audit-trail)
10. [Trade-offs & Design Decisions](#trade-offs--design-decisions)

---

## Design Philosophy

KLAVE was built around a single conviction: **AI agents need wallets the same way humans do, but with stricter guardrails and zero trust.**

Traditional crypto wallets assume a human operator who reads confirmation dialogs, checks addresses, and reasons about risk. Autonomous agents can't do that. They operate in tight loops, make hundreds of decisions per hour, and will happily drain a wallet if their reward signal says so.

KLAVE's answer is **policy-first autonomy**: every agent gets the freedom to transact, but within a policy boundary that the agent itself cannot modify. This creates a three-layer stack:

```
┌──────────────────────────────┐
│  Agent Logic  (Python/LLM)   │  Decides WHAT to do
├──────────────────────────────┤
│  Policy Engine  (Rust)       │  Decides IF it's allowed
├──────────────────────────────┤
│  Wallet Infra  (Solana)      │  Executes the transaction
└──────────────────────────────┘
```

The agent proposes actions. The policy engine gates them. The wallet infrastructure executes what survives. This separation means you can deploy an agent with aggressive trading logic and still rest assured that the policy engine won't let it exceed its budget.

### Core Principles

- **One agent, one keypair.** No shared wallets. Every agent has a unique Solana keypair, its own policy, and its own audit trail.
- **Gasless by default.** Agents shouldn't need SOL for fees. All transactions route through [Kora](https://launch.solana.com/docs/kora/operators), a gasless relayer that co-signs and pays fees.
- **Policy is immutable from the agent's perspective.** Agents can read their policy but cannot modify it. Only the platform operator (via the REST API) can update policies.
- **Every action is audited.** The audit log records every transaction attempt — successful or blocked — with the full policy evaluation result.

---

## Wallet Architecture

Each agent's wallet has two layers:

### 1. Hot Keypair (Agent Wallet)

- Generated at agent creation time via `solana-keygen`
- Encrypted at rest with AES-256-GCM using a server-side master key
- Stored in SQLite (encrypted bytes + nonce)
- Never exposed in API responses — only the public key is returned
- Used to sign all transactions for this agent

### 2. Vault PDA (On-Chain Treasury)

- A Program Derived Address (PDA) from the Anchor treasury program
- Derived deterministically from the agent's pubkey: `seeds = [b"vault", agent_pubkey]`
- Holds SOL in a program-controlled account
- Only the agent (via the treasury program CPI) can deposit/withdraw
- Provides an extra layer of on-chain isolation

```
Agent Keypair (off-chain)         Vault PDA (on-chain)
┌─────────────────────┐          ┌──────────────────────┐
│  Private key (AES)  │──signs──>│  PDA: vault + pubkey │
│  Public key         │          │  SOL balance         │
│  Policy ID          │          │  Program-controlled  │
└─────────────────────┘          └──────────────────────┘
```

### Why Two Layers?

The hot keypair holds SOL for immediate operations. The vault PDA holds reserves that require an explicit deposit/withdraw instruction; you can think of it as a savings account vs. a checking account. An agent's policy can restrict vault withdrawal amounts independently.

---

## Agent-Wallet Interaction Model

Agents interact with KLAVE exclusively through the REST API. There is no direct keypair access.

### Request Flow

```
Agent (Python SDK)
  │
  ├── POST /api/v1/agents/{id}/transactions
  │     Body: { instruction_type: "sol_transfer", amount: 100000, destination: "..." }
  │
  │   ┌─── KLAVE Server ─────────────────────────────────┐
  │   │                                                  │
  │   │  1. Load agent from DB                           │
  │   │  2. Load agent's policy                          │
  │   │  3. PolicyEngine.evaluate(policy, request)       │
  │   │     → DENY? Return 403 + log violation           │
  │   │     → ALLOW? Continue                            │
  │   │  4. Build Solana instruction                     │
  │   │  5. Load encrypted keypair, decrypt              │
  │   │  6. Sign transaction                             │
  │   │  7. Send via Kora (gasless) or fallback RPC      │
  │   │  8. Log to audit trail                           │
  │   │  9. Return tx signature                          │
  │   └──────────────────────────────────────────────────┘
  │
  └── Response: { signature: "...", via_kora: true }
```

### SDK Patterns

The Python SDK provides three integration levels:

**1. Direct client (programmatic):**

```python
async with KlaveClient("http://localhost:3000", api_key="key") as c:
    agent = await c.create_agent("trader", AgentPolicyInput(
        max_lamports_per_tx=500_000_000,
        token_allowlist=["So11...", "4zMM..."],
    ))
    tx = await c.swap_tokens(agent.id, {
        "whirlpool": "...",
        "input_mint": "So11...",
        "amount": 10_000_000,
    })
```

**2. LangChain tools (LLM-driven):**

```python
from klave.tools import build_agent_tools

client = KlaveClient("http://localhost:3000", api_key="key")
tools = build_agent_tools(client)
# Pass `tools` to a LangChain agent — it can autonomously call
# create_agent, get_balance, transfer_sol, swap_tokens, etc.
# but DOES NOT have access to administrative tools.
```

**3. Autonomous loop (hybrid):**

```python
while True:
    balance = await client.get_balance(agent_id)
    if balance.sol_lamports > threshold:
        await client.swap_tokens(agent_id, swap_params)
    await asyncio.sleep(60)
```

---

## Security Model

### Key Management

| Concern        | Approach                                                                                |
| -------------- | --------------------------------------------------------------------------------------- |
| Key generation | `solana-keygen` — standard Ed25519 keypair                                              |
| Storage        | AES-256-GCM encrypted, SQLite                                                           |
| Master key     | Derived from `KLAVE_MASTER_KEY` env var                                                 |
| Key exposure   | Private keys never leave the server process. API responses only include the public key. |
| Key rotation   | Delete agent + create new one. Keys are tied to agent identity.                         |

### Authentication & Authorization

- **Dual API Key Model**:
  - `KLAVE_OPERATOR_API_KEY`: Required for administrative operations (creating agents, deactivating, updating policies). The Python SDK exposes these via `build_operator_tools`.
  - `KLAVE_API_KEY`: Used by agents for runtime operations (transactions, balance checks, history). The Python SDK provides a restricted `build_agent_tools` set for this.
- **Per-agent isolation**: An agent's keypair is loaded only when executing transactions for that specific agent.
- **No cross-agent operations**: Agent A cannot sign with Agent B's key or access Agent B's vault.

### Defense in Depth

```
Layer 1: API Key auth (middleware)
  └─ Layer 2: Policy engine (per-agent rules)
      └─ Layer 3: Kora co-signing (relayer validates)
          └─ Layer 4: On-chain program guards (Anchor treasury)
              └─ Layer 5: Audit trail (immutable log)
```

Each layer is independent. Even if an agent's API key leaks, the policy engine still enforces spend limits. Even if the policy is misconfigured, the on-chain program prevents unauthorized vault access. Every action is logged regardless of outcome.

---

## Multi-Agent Isolation

KLAVE achieves multi-agent isolation through **resource partitioning**, not just namespacing:

| Resource       | Isolation Method                                    |
| -------------- | --------------------------------------------------- |
| Keypair        | Unique Ed25519 pair per agent                       |
| Vault          | Unique PDA per agent (`seeds = [b"vault", pubkey]`) |
| Policy         | Separate policy record per agent                    |
| Audit log      | Agent ID foreign key, queryable per agent           |
| Token accounts | Standard Solana ATAs — owned by agent's pubkey      |

### Scaling Properties

- Agents share no on-chain state. Adding agent N+1 does not affect agents 1..N.
- Each agent can have independent policies (different spend limits, different token allowlists).
- The audit log scales linearly. It is append-only with agent-scoped queries.
- Server resources (SQLite, in-memory policy cache) are shared but operations are stateless per request.

---

## Policy Engine

The policy engine is a **synchronous, stateless evaluator** written in Rust. It receives a policy and a transaction request, and returns either `Ok(())` or `Err(Vec<PolicyViolation>)`.

### Policy Fields

| Field                     | Type          | Effect                                        |
| ------------------------- | ------------- | --------------------------------------------- |
| `allowed_programs`        | `Vec<String>` | Whitelist of program IDs the agent can invoke |
| `max_lamports_per_tx`     | `i64`         | Hard cap on SOL per transaction               |
| `token_allowlist`         | `Vec<String>` | Mints the agent can hold/swap                 |
| `daily_spend_limit_usd`   | `f64`         | Rolling 24h USD spend cap (0 = unlimited)     |
| `daily_swap_volume_usd`   | `f64`         | Rolling 24h swap volume cap (0 = unlimited)   |
| `slippage_bps`            | `i32`         | Max slippage for Orca swaps (basis points)    |
| `withdrawal_destinations` | `Vec<String>` | Addresses the agent can send funds to         |

### Violation Types

- `AgentInactive` — agent has been deactivated
- `ProgramNotAllowed` — instruction targets a non-whitelisted program
- `ExceedsMaxLamports` — transaction amount exceeds per-tx limit
- `TokenNotAllowed` — swap involves a non-whitelisted mint
- `DailySpendExceeded` — 24h spend cap reached
- `DailySwapVolumeExceeded` — 24h swap volume cap reached
- `SlippageExceeded` — requested slippage above policy limit
- `WithdrawalDestinationNotAllowed` — destination not in whitelist

### Defaults

To ensure a seamless developer experience, KLAVE provides **Defaults** during agent creation. If no policy is provided, KLAVE automatically whitelists the System Program, Treasury Program, Token Program, and Orca Whirlpool Program, and sets a $100 daily spend limit and $500 daily swap volume limit. This prevents immediate "Policy Violation" errors for basic operations.

---

## Transaction Pipeline

Every transaction follows a deterministic pipeline:

```
1. Build instruction(s)     — SOL transfer, vault CPI, or Orca swap
2. Fetch latest blockhash   — from Solana RPC
3. Create transaction       — Message + blockhash
4. Sign with agent key      — decrypt keypair, sign
5. Route through Kora       — gasless co-signing
   └─ Fallback: direct RPC  — if Kora unavailable
6. Broadcast to Solana      — submit to network
7. Log to audit trail       — success or failure
8. Return signature          — to caller
```

### Kora Integration

[Kora](https://launch.solana.com/docs/kora/operators) is a Solana gasless relayer. KLAVE sends a partially-signed transaction to Kora's JSON-RPC endpoint. Kora:

1. Validates the transaction
2. Adds a fee-payer signature
3. Returns the fully-signed transaction

KLAVE then broadcasts the fully-signed transaction to the network. If Kora is unavailable or misconfigured, KLAVE falls back to direct RPC submission (which requires the agent to have SOL for fees).

```
Agent signs TX  ──>  Kora adds fee payer  ──>  Broadcast to Solana
                     (gasless)
```

---

## DeFi Integration (Orca Whirlpools)

KLAVE integrates with Orca Whirlpools — Solana's leading concentrated liquidity DEX — for token swaps.

### Endpoints

| Endpoint                         | Method | Description                        |
| -------------------------------- | ------ | ---------------------------------- |
| `/api/v1/agents/{id}/orca/swap`  | POST   | Execute a token swap               |
| `/api/v1/agents/{id}/orca/quote` | POST   | Get a swap quote without executing |
| `/api/v1/orca/pools`             | GET    | Discover available pools           |

### Swap Flow

```
1. Agent requests swap (whirlpool, input_mint, amount, slippage)
2. Policy engine validates:
   - Token mints in allowlist
   - Slippage within bounds
   - Daily swap volume within limit
3. OrcaClient builds swap instructions via orca_whirlpools SDK
4. Transaction signed with agent keypair
5. Routed through Kora (gasless)
6. Audit entry logged with InstructionType::TokenSwap
```

### Pool Discovery

The `GET /orca/pools` endpoint queries the Whirlpool program's on-chain accounts directly. It filters by:

- Devnet whirlpools config
- Non-zero liquidity
- Optional token mint filter

Results are sorted by liquidity (descending) and include computed price from `sqrt_price_x64`.

---

## Audit Trail

Every transaction attempt generates an audit entry, regardless of outcome:

```json
{
  "id": 42,
  "agent_id": "a1b2c3...",
  "timestamp": 1706000000,
  "instruction_type": "sol_transfer",
  "status": "success",
  "tx_signature": "5K7x...",
  "policy_violations": null,
  "metadata": "{\"destination\":\"...\",\"lamports\":100000}"
}
```

For blocked transactions, `status` is `"blocked"` and `policy_violations` contains the violation details. This creates a complete record that answers:

- What did the agent try to do?
- Was it allowed?
- If blocked, why?
- If allowed, what was the on-chain result?

---

## Trade-offs & Design Decisions

### SQLite over Postgres

We chose SQLite for the audit log and agent registry. At the scale of a devnet prototype with dozens of agents, SQLite provides zero-config deployment, atomic file-level backups, and sub-millisecond reads. A production deployment serving thousands of agents would likely migrate to Postgres.

### Kora for Gasless TX

Gasless transactions are non-negotiable for agent wallets. This requires agents to maintain SOL balances just for fees creates an operational overhead that defeats the purpose of autonomy. Kora provides this through fee-payer co-signing.

### Policy Engine as Static Evaluator

The policy engine is deliberately stateless and synchronous. It receives inputs, evaluates rules, returns a verdict. It does not maintain running counters or state. Daily spend/volume limits are computed from the audit log at evaluation time. This makes the engine trivially testable and eliminates race conditions.

### AES-256 Key Encryption

Agent private keys are encrypted at rest rather than stored in a hardware security module (HSM). This is appropriate for a devnet prototype. A production system would integrate with AWS KMS, HashiCorp Vault, or a similar service for key management.

### No Agent-Side Policy Modification

Agents cannot modify their own policies by design. If an agent could relax its own spend limit, the policy system would be meaningless. Only the platform operator (human) can update policies via `PUT /api/v1/agents/{id}/policy`.

---

_Built for the [Superteam DeFi Developer Challenge](https://superteam.fun/earn/listing/defi-developer-challenge-agentic-wallets-for-ai-agents). See the [README](README.md) for setup instructions and [SKILLS.md](SKILLS.md) for the full API reference._
