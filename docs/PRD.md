# Product Requirements Document

## KLAVE — Agentic Wallet Infrastructure

**Status:** Draft  
**Version:** 1.2.0  
**Context:** DeFi Developer Challenge – Agentic Wallets for AI Agents

---

## 1. Problem Statement

The challenge asks for an agentic wallet. Most submissions will solve this literally: one agent, one wallet, one task. That is not a winning submission in a competition judged on scalability, security, and documentation depth.

The actual unsolved problem is: once you have more than one agent, how do you provision wallets, enforce what each agent is allowed to do, fund them, and keep the entire system auditable? No existing Solana tooling answers this. A developer building a multi-agent trading system today has to solve wallet provisioning, funding, key isolation, policy enforcement, and fee management from scratch for every project.

Token swapping adds a second unsolved problem on top of this: how does an agent autonomously rebalance its portfolio, pay for on-chain services in a specific token, or execute a trading strategy — without holding SOL for gas and without requiring a human to approve each swap?

KLAVE builds that missing infrastructure layer, extended with a gasless swap gateway.

---

## 2. What KLAVE Is

KLAVE is an agentic wallet infrastructure server. It exposes a REST API that any AI agent framework — LangChain, a Python script, a Rust binary — calls to create wallets, fund them, submit transactions, query state, and now execute token swaps and manage liquidity. Internally it enforces per-agent policies before any transaction reaches the Solana network, routes gasless transactions through Kora, assembles complex DeFi instructions via the Orca Whirlpools SDK, and persists an immutable audit log.

It ships with:

- A Rust core service (the wallet registry + gateway + Orca DeFi engine)
- An Anchor program on devnet (a shared treasury agents deposit into and withdraw from)
- A Python SDK with typed tool wrappers for LLM integration (including `orca_swap` and `orca_liquidity` tools)
- A terminal-style dashboard UI for live observation

---

## 3. Users

| User                                 | What They Need                                                                          |
| ------------------------------------ | --------------------------------------------------------------------------------------- |
| Hackathon judges                     | A functional multi-agent demo they can watch run on devnet, with real balances changing |
| AI agent developers (post-hackathon) | A server they can `cargo install` and point their agents at                             |
| Operators                            | Per-agent policy configuration, swap token allowlists, and a full audit trail           |

---

## 4. Functional Requirements

### 4.1 Agent Lifecycle

| ID   | Requirement                                                                            |
| ---- | -------------------------------------------------------------------------------------- |
| F-01 | Create an agent: generates a keypair, stores the agent profile, returns the public key |
| F-02 | Delete an agent: marks the agent inactive, keypair is never reused                     |
| F-03 | List all agents with their current SOL balance and vault balance                       |
| F-04 | Fetch a single agent's transaction history                                             |
| F-05 | Fetch a single agent's live SOL and vault balance via a dedicated balance endpoint     |

### 4.2 Funding

| ID   | Requirement                                                                                                                                                 |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F-06 | After agent creation, KLAVE returns the agent's pubkey; the operator funds it manually via `solana airdrop` (devnet) or the web faucet at faucet.solana.com |
| F-07 | The vault PDA is initialized for each agent before the first deposit instruction is submitted                                                               |
| F-08 | The balance endpoint queries the Solana RPC for the agent wallet lamports and the vault PDA lamports, returning both                                        |

### 4.3 Policy Engine

| ID   | Requirement                                                                                                                                                                                            |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| F-09 | Each agent has an attached policy at creation time                                                                                                                                                     |
| F-10 | Policy defines: `allowed_programs`, `max_lamports_per_tx`, `token_allowlist`, `daily_spend_limit_usd`, `withdrawal_destinations` (allowlist of permitted destination pubkeys for operator withdrawals) |
| F-11 | Any transaction request that violates the policy is rejected before reaching Kora                                                                                                                      |
| F-12 | Policy can be updated by the operator; the change is effective on the next transaction request                                                                                                         |
| F-13 | Blocked transactions are logged with reason                                                                                                                                                            |

### 4.4 Transaction Gateway

| ID   | Requirement                                                                                                                                                                                                                                                        |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| F-14 | Agents submit transaction requests by instruction type — the server assembles the full transaction                                                                                                                                                                 |
| F-15 | Supported instruction types: `sol_transfer`, `initialize_vault`, `deposit_to_vault`, `withdraw_from_vault`, `agent_withdrawal`                                                                                                                                     |
| F-16 | The gateway routes through Kora for gasless execution when Kora is available                                                                                                                                                                                       |
| F-17 | The gateway falls back to direct RPC broadcast when Kora is unavailable                                                                                                                                                                                            |
| F-18 | Every transaction (success or failure) is written to the audit log                                                                                                                                                                                                 |
| F-19 | The transaction response includes `via_kora: bool` indicating which path was used                                                                                                                                                                                  |
| F-41 | All write endpoints (`POST`, `PUT`, `DELETE`) require an `X-API-Key` header matched against `KLAVE_API_KEY`; requests without a valid key are rejected with 401 before any handler logic executes                                                                  |
| F-42 | Operators can withdraw SOL from an agent wallet via `POST /api/v1/agents/{id}/withdraw`; the destination address must be present in the agent's `withdrawal_destinations` policy allowlist or the request is rejected with 403 before any transaction is assembled |
| F-43 | Agent withdrawal requests are subject to policy enforcement (`max_lamports_per_tx`, `daily_spend_limit_usd`) and written to the audit log with `instruction_type: agent_withdrawal`                                                                                |

### 4.5 Audit Log

| ID   | Requirement                                                                                              |
| ---- | -------------------------------------------------------------------------------------------------------- |
| F-20 | Append-only SQLite table: agent_id, timestamp, instruction_type, status, tx_signature, policy_violations |
| F-21 | Queryable by agent                                                                                       |

### 4.6 Test dApp (Anchor Program)

| ID   | Requirement                                                                  |
| ---- | ---------------------------------------------------------------------------- |
| F-22 | A shared treasury program where agents can `deposit` and `withdraw` lamports |
| F-23 | Each agent has an isolated vault account (PDA derived from agent public key) |
| F-24 | The treasury program is deployed on devnet                                   |
| F-25 | Balance changes in the treasury are observable from the dashboard            |

### 4.7 Python SDK

| ID   | Requirement                                                                                                                                                                                                             |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F-26 | Typed `KlaveClient` class wrapping all REST endpoints                                                                                                                                                                   |
| F-27 | LangChain-compatible tool definitions for: `create_agent`, `check_balance`, `deposit_to_vault`, `withdraw_from_vault`, `transfer_sol`, `swap_tokens`                                                                    |
| F-28 | A runnable demo script that spawns three agents with different policies, waits for the operator to confirm manual funding, initializes their vaults, then runs an autonomous decision loop using real on-chain balances |

### 4.8 Dashboard

| ID   | Requirement                                                                      |
| ---- | -------------------------------------------------------------------------------- |
| F-29 | Single-page HTML dashboard, no build step required                               |
| F-30 | Live agent list with real SOL balance, vault balance, policy summary, and status |
| F-31 | Transaction feed with real-time polling                                          |
| F-32 | Retro brutalist visual design per SIGNATURE.md                                   |

### 4.9 Orca DeFi Engine

Beyond simple transfers, agents can autonomously participate in the Orca DeFi ecosystem. KLAVE integrates the Orca Whirlpools SDK for low-latency, concentrated liquidity operations and token swaps, with Kora handling gasless execution.

| ID   | Requirement                                                                                                                                               |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F-33 | Agents can execute token swaps via Orca Whirlpools with automated price discovery.                                                                        |
| F-34 | Agents can open concentrated liquidity positions (Splash or Full Range) in any supported Orca pool.                                                       |
| F-35 | Agents can autonomously `increase`, `decrease`, or `harvest` rewards from their liquidity positions.                                                      |
| F-36 | DeFi transactions are versioned transactions (v0); the gateway handles instruction assembly and co-signing correctly.                                     |
| F-37 | Policy enforcement applies to DeFi: `token_allowlist` gates pool involvement; `max_lamports_per_tx` gates estimated fees.                                 |
| F-38 | A `daily_spend_limit_usd` policy field caps total USD-denominated outflow (including swaps and liquidity deposits) to prevent catastrophic logic failure. |
| F-39 | Every DeFi operation is written to the audit log with rich metadata (pool address, instruction type, input/output amounts, and on-chain signature).       |
| F-40 | Responses include `tx_signature` and `via_kora: bool` to verify gasless execution.                                                                        |

---

## 5. Non-Functional Requirements

| Category      | Requirement                                                                                            |
| ------------- | ------------------------------------------------------------------------------------------------------ |
| Security      | All private key material stays inside the Rust process; never returned via API                         |
| Security      | Policy enforcement happens before any external call                                                    |
| Security      | Jupiter quote and swap API calls use HTTPS; responses are validated before transaction deserialization |
| Performance   | API response latency under 200ms for non-Solana operations                                             |
| Performance   | DeFi execution latency under 300ms; agents can act during high volatility                              |
| Reliability   | If Kora is unavailable, fallback to direct RPC; no silent failures                                     |
| Reliability   | If Jupiter is unavailable, the swap endpoint returns a 503 with a clear error message                  |
| Portability   | Zero external services required to run locally (SQLite, local Kora; Jupiter uses devnet endpoints)     |
| Observability | Structured JSON logging (`tracing` crate), `/health` endpoint                                          |

---

## 6. Out of Scope

- Production key management (Vault, Turnkey, HSM). Architecture is designed for it; devnet demo uses memory signers.
- Multi-region replication of the agent registry.
- Frontend authentication. The dashboard is read-only and runs locally.
- Custom AMM routing beyond what Orca Whirlpools expose.

---

## 7. Success Criteria for Hackathon Submission

| Criterion                                  | How KLAVE Satisfies It                                                                                         |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| Functional autonomous agent wallet         | Three agents on devnet, each funded, making independent decisions using real on-chain balances                 |
| Security and proper key management         | Policy engine; keys never exposed through API; solana-keychain isolation                                       |
| Clear documentation and deep dive          | PRD, IMPLEMENTATION, System Design, ADRs, Runbook, deep-dive written explanation                               |
| Scalability: multiple agents independently | Architecture supports N agents from day one by design                                                          |
| Token swapping & Liquidity Provision       | Agents autonomously swap tokens and manage concentrated liquidity via Orca with gasless execution through Kora |
| SKILLS.md for agents                       | Provided, covering all API endpoints, policy schema, and swap parameters                                       |

---

## 8. Competitive Differentiation

The infrastructure framing is the core differentiator. Every other submission answers "how does one agent get a wallet?" KLAVE answers "how does a team of 100 agents each get a wallet, with different permissions, real on-chain balances, observable in real time, without any of them needing SOL for gas?"

The Kora integration is the second differentiator. The challenge resource links directly to Kora operator docs. Using Kora is the intended path. Using it correctly, with a policy layer above it, demonstrates depth.

The Orca DeFi engine is the third differentiator. It moves agents beyond simple token swaps into active market participation (LPing). An agent that can autonomously manage a concentrated liquidity position — within operator-defined pools and volume limits — is a materially more powerful primitive. Using the Orca Whirlpools SDK directly for instruction assembly, combined with KLAVE's policy enforcement and Kora's gasless routing, forms a sophisticated and secure integration.

The Python SDK bridging a Rust service is the fourth differentiator. It shows the two halves of the stack working together — Rust for correctness and performance in the critical path, Python for the ergonomic AI agent tooling layer.
