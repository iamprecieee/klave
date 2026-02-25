# KLAVE — Implementation Plan

**Version:** 1.0.0  
**Status:** Active  
**PRD Version:** 1.2.0

---

## Overview

This document translates the PRD into a sequenced, component-level build plan. The four phases are ordered by dependency: infrastructure before features, features before demo integration. Each phase is shippable at its boundary — the system is in a working state after every phase, not only at the end.

The primary implementation language is Rust (Tokio runtime, Axum HTTP framework). The secondary language is Python 3.11+ (SDK and demo script). The on-chain program is Anchor 0.32.x.

---

## Phase 1 — Core Infrastructure

Build the server skeleton, agent registry, policy engine, and audit log. No Solana transactions in this phase — only the data model and HTTP interface.

### 1.1 Workspace Layout

```
klave/
├── Cargo.toml                  # workspace
├── klave-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── agent/              # registry, keypair generation
│       │   ├── mod.rs
│       │   ├── model.rs        # Agent, AgentPolicy structs
│       │   └── repository.rs  # SQLite persistence layer
│       ├── policy/             # enforcement engine
│       │   ├── mod.rs
│       │   └── engine.rs
│       ├── audit/              # append-only log
│       │   ├── mod.rs
│       │   └── store.rs
│       ├── error.rs            # thiserror enum
│       └── db.rs               # sqlx pool init, migrations
├── klave-server/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs           # server config from env
│       ├── router.rs           # Axum route definitions
│       ├── handlers/
│       │   ├── mod.rs
│       │   ├── agents.rs       # CRUD handlers (F-01 to F-05)
│       │   ├── health.rs
│       │   ├── orca.rs         # Orca DeFi handlers (F-33 to F-40)
│       │   └── transactions.rs # gateway handlers (F-14 to F-19)
│       └── state.rs            # shared AppState (pool, config, rpc)
├── klave-anchor/               # Anchor workspace
│   ├── Anchor.toml
│   ├── Cargo.toml
│   └── programs/
│       └── klave-treasury/
│           └── src/
│               ├── lib.rs
│               ├── instructions/
│               │   ├── mod.rs
│               │   ├── initialize_vault.rs
│               │   ├── deposit.rs
│               │   └── withdraw.rs
│               ├── state/
│               │   └── mod.rs
│               └── error.rs
├── sdk/                        # Python SDK
│   ├── pyproject.toml
│   ├── klave/
│   │   ├── __init__.py
│   │   ├── client.py
│   │   ├── models.py
│   │   ├── tools.py            # LangChain tool wrappers
│   │   └── exceptions.py
│   └── demo/
│       └── multi_agent_demo.py
└── dashboard/
    └── index.html
```

### 1.2 Data Model

**Agent** (agents table):

| Column        | Type    | Notes                              |
| ------------- | ------- | ---------------------------------- |
| id            | TEXT PK | UUID v4                            |
| pubkey        | TEXT    | Base58 encoded                     |
| keypair_bytes | BLOB    | Encrypted at rest using OS keyring |
| label         | TEXT    | Human-readable name                |
| is_active     | BOOLEAN |                                    |
| created_at    | INTEGER | Unix timestamp                     |
| policy_id     | TEXT FK |                                    |

**AgentPolicy** (agent_policies table):

| Column                  | Type    | Notes                                                                       |
| ----------------------- | ------- | --------------------------------------------------------------------------- |
| id                      | TEXT PK | UUID v4                                                                     |
| agent_id                | TEXT FK |                                                                             |
| allowed_programs        | TEXT    | JSON array of base58 pubkeys                                                |
| max_lamports_per_tx     | INTEGER |                                                                             |
| token_allowlist         | TEXT    | JSON array of mint addresses                                                |
| daily_spend_limit_usd   | REAL    | 0 = unlimited                                                               |
| daily_swap_volume_usd   | REAL    | 0 = unlimited                                                               |
| slippage_bps            | INTEGER | Default 50 (0.5%)                                                           |
| withdrawal_destinations | TEXT    | JSON array of allowed destination pubkeys; empty = no withdrawals permitted |
| updated_at              | INTEGER | Unix timestamp                                                              |

**AuditLog** (audit_log table):

| Column            | Type    | Notes                                           |
| ----------------- | ------- | ----------------------------------------------- |
| id                | INTEGER | Auto-increment PK                               |
| agent_id          | TEXT    |                                                 |
| timestamp         | INTEGER | Unix timestamp                                  |
| instruction_type  | TEXT    | One of the defined instruction type enum values |
| status            | TEXT    | `submitted`, `confirmed`, `failed`, `rejected`  |
| tx_signature      | TEXT    | Base58 or NULL on policy rejection              |
| policy_violations | TEXT    | JSON array of violation descriptions            |
| metadata          | TEXT    | JSON blob for swap-specific fields              |

### 1.3 HTTP API Surface (Phase 1)

All responses follow the standard envelope:

> Write endpoints (`POST`, `PUT`, `DELETE`) require an `X-API-Key` header matching `KLAVE_API_KEY`. Requests without a valid key are rejected with 401 before handler logic executes. This is an Axum middleware layer on the `/api/v1` router.

```json
{
  "success": true,
  "message": "...",
  "data": {},
  "status_code": 200
}
```

| Method | Path                         | FR         | Description         |
| ------ | ---------------------------- | ---------- | ------------------- |
| POST   | /api/v1/agents               | F-01       | Create agent        |
| DELETE | /api/v1/agents/{id}          | F-02       | Deactivate agent    |
| GET    | /api/v1/agents               | F-03       | List agents         |
| GET    | /api/v1/agents/{id}/history  | F-04       | Transaction history |
| GET    | /api/v1/agents/{id}/balance  | F-05       | SOL + vault balance |
| PUT    | /api/v1/agents/{id}/policy   | F-12       | Update policy       |
| POST   | /api/v1/agents/{id}/withdraw | F-41, F-42 | Operator withdrawal |
| GET    | /health                      | —          | Health check        |

### 1.4 Policy Engine

The `PolicyEngine` struct takes an `AgentPolicy` and an incoming `TransactionRequest`. It evaluates rules in order:

1. Check `is_active` on the agent. Reject if false.
2. Verify all program IDs in the request appear in `allowed_programs`.
3. Check estimated lamport cost against `max_lamports_per_tx`.
4. For token operations, verify all mint addresses appear in `token_allowlist`.
5. Aggregate today's USD spend from `audit_log`; reject if adding this transaction would exceed `daily_spend_limit_usd`.

Any failing check returns a `PolicyViolation` enum variant, appended to the rejection log entry.

### 1.5 Deliverables

- Working HTTP server with in-memory state passing CI
- SQLite schema with forward-only migrations via `sqlx`
- Policy engine with unit tests covering all violation cases
- `/health` endpoint returning build metadata

---

## Phase 2 — Solana Integration and Transaction Gateway

Wire the Rust service to devnet: keypair-backed signing, Kora gasless routing, anchor program deployment, and the full transaction gateway.

### 2.1 Signing Layer

Use `solana-keychain` (memory feature only) for devnet. The `AgentSigner` service wraps a `Signer::Memory` per agent, loaded from the stored keypair bytes on demand and evicted after use to minimize in-memory key exposure.

```
AgentSigner::load(agent_id) -> Result<Signer, KlaveError>
AgentSigner::sign_and_release(signer, tx) -> Result<Signature, KlaveError>
```

### 2.2 Kora Gateway

The `KoraGateway` struct implements a two-path routing strategy:

1. **Primary path:** Send the assembled transaction to Kora's `signAndSendTransaction` JSON-RPC method. Kora validates its own policy, co-signs as fee payer, and broadcasts.
2. **Fallback path:** If Kora is unreachable or returns a non-2xx response, sign directly with the agent's keypair and broadcast via the Solana RPC connection.

The response always includes `via_kora: bool`.

### 2.3 Anchor Treasury Program

Instructions:

- `initialize_vault`: Creates a PDA account (`seeds = [b"vault", agent_pubkey]`) to hold agent lamports in the shared treasury.
- `deposit`: Transfers lamports from agent wallet to the vault PDA.
- `withdraw`: Transfers lamports from vault PDA back to agent wallet. Signer check enforces that only the agent's keypair can authorize.

The program is deployed to devnet. The IDL is generated by `anchor build` and committed to `klave-anchor/target/idl/`.

### 2.4 Transaction Gateway — Instruction Types

The gateway exposes a single `POST /api/v1/agents/{id}/transactions` endpoint. The `instruction_type` field in the request body routes to the correct builder:

| `sol_transfer` | System Program `transfer` instruction |
| `initialize_vault` | CPI to `klave-treasury` `initialize_vault` |
| `deposit_to_vault` | CPI to `klave-treasury` `deposit` |
| `withdraw_from_vault` | CPI to `klave-treasury` `withdraw` |
| `agent_withdrawal` | System Program `transfer` from agent wallet to operator destination |

Each builder constructs a `Transaction` or `VersionedTransaction`, applies the policy engine check, then hands the transaction to `KoraGateway`.

### 2.5 Balance Endpoint

The `/balance` endpoint fires two concurrent RPC calls:

1. `getBalance` on the agent's pubkey for native SOL lamports.
2. `getAccountInfo` on the vault PDA for the stored lamport amount.

Both calls use `commitment: confirmed`. Results are returned together in a single response.

### 2.6 Deliverables

- Anchor treasury program deployed on devnet with tests passing
- Kora gateway with fallback routing
- Full gateway handling all four instruction types
- Audit log entries written on every gateway call
- Integration test: create agent wallet → fund manually → deposit → withdraw → check balance

---

## Phase 3 — Orca DeFi Engine

Integrate the Orca Whirlpools SDK for high-performance DeFi execution. This phase adds the Orca client, extends the policy engine with pool-specific checks, and wires Kora's gasless path for complex multi-signer transactions.

### 3.1 Orca Client

The `OrcaClient` struct encapsulates all logic for instruction assembly using the `whirlpool` crate. It handles route generation (for swaps) and instruction building for liquidity provision.

**Configuration:**

```toml
[solana]
rpc_url = "https://api.devnet.solana.com"
network = "Devnet"
```

**Methods:**

```rust
OrcaClient::swap(...) -> Result<OrcaInstructionResult, KlaveError>
OrcaClient::open_splash_position(...) -> Result<OrcaInstructionResult, KlaveError>
OrcaClient::increase_liquidity(...) -> Result<OrcaInstructionResult, KlaveError>
OrcaClient::harvest_rewards(...) -> Result<OrcaInstructionResult, KlaveError>
```

### 3.2 DeFi Policy Extension

The policy engine gains additional checks for Orca interactions:

1.  **Token allowlist:** For swaps and liquidity provision, all involved mints must appear in `token_allowlist`.
2.  **Daily spend limit:** Calculate the SOL/USD value of the transaction (tokens sent + rent). Reject if today's aggregate outflow exceeds `daily_spend_limit_usd`.

### 3.3 DeFi Handlers

`POST /api/v1/agents/{id}/orca/swap`
`POST /api/v1/agents/{id}/orca/open-position`
`PUT /api/v1/agents/{id}/orca/position/increase`
`DELETE /api/v1/agents/{id}/orca/position/{pubkey}`

Execution sequence:

1.  Load agent and policy. Run pre-checks.
2.  Call `OrcaClient` to generate instructions and identify `additional_signers`.
3.  Assemble `VersionedTransaction`. Replace fee payer with Kora's signer address.
4.  Submit via `KoraGateway`. Kora co-signs as fee payer.
5.  Write to audit log with rich metadata.

### 3.5 Deliverables

- Orca client supporting swaps and liquidity management
- Policy checks for pool interactions and USD spend limits
- Suite of Orca handlers integrated with KoraGateway
- Audit log entries for every on-chain event
- Integration test on devnet: Agent swaps SOL for USDC and opens a concentrated liquidity position

---

## Phase 4 — SDK, Dashboard, and Demo

### 4.1 Python SDK

`KlaveClient` is a thin typed wrapper over the KLAVE REST API. All methods are async (`httpx.AsyncClient`).

```python
class KlaveClient:
    def __init__(self, base_url: str, timeout: float = 10.0) -> None: ...

    async def create_agent(self, label: str, policy: AgentPolicy) -> Agent: ...
    async def delete_agent(self, agent_id: str) -> None: ...
    async def list_agents(self) -> list[Agent]: ...
    async def get_balance(self, agent_id: str) -> AgentBalance: ...
    async def get_history(self, agent_id: str) -> list[AuditEntry]: ...
    async def transfer_sol(self, agent_id: str, destination: str, lamports: int) -> TxResult: ...
    async def deposit_to_vault(self, agent_id: str, lamports: int) -> TxResult: ...
    async def withdraw_from_vault(self, agent_id: str, lamports: int) -> TxResult: ...
    async def swap_tokens(self, agent_id: str, req: SwapRequest) -> SwapResult: ...
    async def orca_open_position(self, agent_id: str, req: OpenPositionRequest) -> TxResult: ...
    async def orca_harvest(self, agent_id: str) -> TxResult: ...
```

**LangChain tools** in `klave/tools.py` use `@tool` decorator from `langchain_core.tools`. Each tool wraps one `KlaveClient` method, provides a docstring describing when the LLM should invoke it, and maps the structured input/output to Python dataclasses.

```python
@tool
def swap_tokens(
    agent_id: str,
    input_mint: str,
    output_mint: str,
    amount: int,
    slippage_bps: int = 50,
) -> SwapResult:
    """
    Swap one SPL token for another using the agent's wallet.
    Use this when the agent needs to rebalance its token holdings
    or acquire a specific token to execute a strategy.
    """
    ...
```

### 4.2 Demo Script

`demo/multi_agent_demo.py` runs three agents concurrently:

- **Agent Alpha**: Conservative policy. Daily swap volume capped at $100 USD. Allowed tokens: USDC, USDT.
- **Agent Beta**: Moderate policy. Daily swap volume capped at $500 USD. Allowed tokens: USDC, USDT, BONK.
- **Agent Gamma**: Aggressive policy. Daily swap volume capped at $2000 USD. Allowed tokens: any in the global allowlist.

Each agent runs a decision loop every 30 seconds:

1. Fetch its balance.
2. If USDC balance exceeds a threshold, swap a portion to USDT.
3. If USDT balance exceeds a threshold, deposit to vault.
4. Log all decisions and outcomes to stdout.

The script uses `asyncio.gather` to run all three loops concurrently. It prompts the operator to confirm manual funding is complete before starting the decision loop. Output is structured JSON for dashboard consumption.

### 4.3 Dashboard

Single-file `dashboard/index.html`. No build step, no external dependencies beyond CDN fonts.

Panels:

- **Agent Grid**: Card per agent. Shows pubkey (truncated), SOL balance, vault balance, policy summary, status indicator (online/offline based on last activity).
- **Transaction Feed**: Scrolling list, newest first. Each entry shows: agent label, instruction type, status badge, signature link to Solana Explorer (devnet), timestamp.
- **Swap Activity**: Dedicated section showing swap entries with input → output token pair, amounts, price impact, execution price.

Polling interval: 3 seconds. Implementation: `setInterval` calling the KLAVE `/api/v1/agents` and their history endpoints. Design: retro brutalist per SIGNATURE.md (burgundy/cream palette, monospace, heavy borders, CRT scanline overlay).

### 4.4 Deliverables

- Python SDK with full type coverage
- All LangChain tools including `swap_tokens`
- Running multi-agent demo on devnet with at least one confirmed swap
- Dashboard showing live agent state and swap activity
- `SKILLS.md` updated to document the swap endpoint and policy fields

---

## Dependency Reference

| Crate                | Version | Purpose                              |
| -------------------- | ------- | ------------------------------------ |
| axum                 | 0.7     | HTTP server                          |
| tokio                | 1       | Async runtime                        |
| sqlx                 | 0.8     | SQLite async ORM                     |
| serde                | 1       | Serialization                        |
| serde_json           | 1       | JSON handling                        |
| thiserror            | 2       | Library error types                  |
| anyhow               | 1       | Application error handling           |
| tracing              | 0.1     | Structured logging                   |
| tracing-subscriber   | 0.3     | Log output formatting                |
| reqwest              | 0.12    | HTTP client                          |
| whirlpool            | 0.10    | Orca Whirlpools instruction assembly |
| orca-whirlpools-core | 0.10    | Orca core types                      |
| solana-sdk           | 2.1     | Transaction types, pubkeys           |
| solana-client        | 2.1     | RPC client                           |
| solana-keychain      | 0.2.1   | Signer abstraction                   |
| anchor-client        | 0.32.1  | Anchor CPI calls                     |
| bincode              | 1       | VersionedTransaction deserialization |
| uuid                 | 1       | Agent ID generation                  |
| base64               | 0.22    | Transaction encoding                 |
| wiremock             | 0.6     | HTTP mock server for tests           |

| Python Package | Version | Purpose                     |
| -------------- | ------- | --------------------------- |
| httpx          | 0.27    | Async HTTP client           |
| pydantic       | 2       | Request/response validation |
| langchain-core | 0.2     | Tool decorators             |
| asyncio        | stdlib  | Concurrency                 |
