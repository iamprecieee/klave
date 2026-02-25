"""Pydantic models matching the KLAVE server's JSON payloads.

Every model corresponds to a request or response struct in the Rust server.
Frozen where the data is read-only (responses). Mutable for request inputs.
"""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict


# ── Agent lifecycle ──────────────────────────────────────────────


class AgentPolicyInput(BaseModel):
    """Mutable input for creating/updating an agent's policy."""

    allowed_programs: list[str] = []
    max_lamports_per_tx: int = 1_000_000_000
    token_allowlist: list[str] = []
    daily_spend_limit_usd: float = 0.0
    daily_swap_volume_usd: float = 0.0
    slippage_bps: int = 50
    withdrawal_destinations: list[str] = []


class CreateAgentRequest(BaseModel):
    label: str
    policy: AgentPolicyInput


class Agent(BaseModel):
    """Immutable agent returned by the server."""

    model_config = ConfigDict(frozen=True)

    id: str
    pubkey: str
    label: str
    is_active: bool
    created_at: int
    policy_id: str


class AgentBalance(BaseModel):
    model_config = ConfigDict(frozen=True)

    sol_lamports: int
    vault_lamports: int


# ── Audit ────────────────────────────────────────────────────────


class AuditEntry(BaseModel):
    model_config = ConfigDict(frozen=True)

    id: int
    agent_id: str
    timestamp: int
    instruction_type: str
    status: str
    tx_signature: str | None = None
    policy_violations: str | None = None
    metadata: str | None = None


# ── Transaction gateway ─────────────────────────────────────────


class TxResult(BaseModel):
    """Result of a gateway or Orca transaction."""

    model_config = ConfigDict(frozen=True)

    signature: str
    via_kora: bool


# ── Orca DeFi ────────────────────────────────────────────────────


class OrcaSwapRequest(BaseModel):
    whirlpool: str
    input_mint: str
    amount: int
    slippage_bps: int | None = None
