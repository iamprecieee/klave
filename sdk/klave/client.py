"""Async HTTP client for the KLAVE agentic wallet API.

Usage::

    async with KlaveClient("http://localhost:3000", api_key="key") as c:
        agent = await c.create_agent("trader-1", AgentPolicyInput())
        balance = await c.get_balance(agent.id)
"""

from __future__ import annotations

from typing import Any

import httpx

from klave.exceptions import (
    KlaveApiError,
    KlaveConnectionError,
    PolicyViolationError,
)
from klave.models import (
    Agent,
    AgentBalance,
    AgentPolicyInput,
    AuditEntry,
    ClosePositionRequest,
    CreateAgentRequest,
    DecreaseLiquidityRequest,
    HarvestRequest,
    IncreaseLiquidityRequest,
    OpenPositionRequest,
    OrcaSwapRequest,
    TxResult,
)


class KlaveClient:
    """Typed async wrapper over the KLAVE REST API."""

    def __init__(
        self,
        base_url: str,
        api_key: str = "",
        timeout: float = 30.0,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._api_key = api_key
        self._http = httpx.AsyncClient(
            base_url=self._base_url,
            timeout=timeout,
            headers=self._build_headers(),
        )

    def _build_headers(self) -> dict[str, str]:
        headers: dict[str, str] = {"content-type": "application/json"}
        if self._api_key:
            headers["x-api-key"] = self._api_key
        return headers

    # ── Context manager ──────────────────────────────────────────

    async def __aenter__(self) -> KlaveClient:
        return self

    async def __aexit__(self, *exc: object) -> None:
        await self.close()

    async def close(self) -> None:
        await self._http.aclose()

    # ── Internal request helper ──────────────────────────────────

    async def _request(
        self,
        method: str,
        path: str,
        *,
        json: dict[str, Any] | None = None,
    ) -> Any:
        """Send request, unwrap KLAVE envelope, raise on errors."""
        try:
            resp = await self._http.request(method, path, json=json)
        except httpx.TimeoutException as exc:
            raise KlaveConnectionError(f"request timed out: {exc}") from exc
        except httpx.ConnectError as exc:
            raise KlaveConnectionError(str(exc)) from exc

        if resp.status_code == 204:
            return None

        body = resp.json()
        if not resp.is_success:
            message = body.get("message", resp.text)
            if resp.status_code == 403:
                raise PolicyViolationError(message)
            raise KlaveApiError(resp.status_code, message)

        return body.get("data")

    # ── Agent lifecycle ──────────────────────────────────────────

    async def create_agent(
        self,
        label: str,
        policy: AgentPolicyInput | dict[str, Any] | None = None,
    ) -> Agent:
        """Create a new agent wallet with the given policy."""
        if policy is None:
            policy = AgentPolicyInput()
        if isinstance(policy, dict):
            policy = AgentPolicyInput(**policy)
        payload = CreateAgentRequest(label=label, policy=policy)
        data = await self._request(
            "POST", "/api/v1/agents", json=payload.model_dump()
        )
        return Agent.model_validate(data)

    async def list_agents(self) -> list[Agent]:
        """List all registered agents."""
        data = await self._request("GET", "/api/v1/agents")
        return [Agent.model_validate(item) for item in data]

    async def delete_agent(self, agent_id: str) -> None:
        """Deactivate an agent by ID."""
        await self._request("DELETE", f"/api/v1/agents/{agent_id}")

    async def get_balance(self, agent_id: str) -> AgentBalance:
        """Fetch the SOL and vault balance for an agent."""
        data = await self._request(
            "GET", f"/api/v1/agents/{agent_id}/balance"
        )
        return AgentBalance.model_validate(data)

    async def get_history(self, agent_id: str) -> list[AuditEntry]:
        """Fetch the audit log for an agent."""
        data = await self._request(
            "GET", f"/api/v1/agents/{agent_id}/history"
        )
        return [AuditEntry.model_validate(item) for item in data]

    async def update_policy(
        self,
        agent_id: str,
        policy: AgentPolicyInput | dict[str, Any],
    ) -> dict[str, Any]:
        """Update the policy for an agent. Returns the updated policy."""
        if isinstance(policy, dict):
            policy = AgentPolicyInput(**policy)
        data = await self._request(
            "PUT",
            f"/api/v1/agents/{agent_id}/policy",
            json=policy.model_dump(),
        )
        return data

    # ── Transaction gateway ──────────────────────────────────────

    async def transfer_sol(
        self,
        agent_id: str,
        destination: str,
        lamports: int,
    ) -> TxResult:
        """Transfer SOL from the agent to a destination address."""
        data = await self._request(
            "POST",
            f"/api/v1/agents/{agent_id}/transactions",
            json={
                "instruction_type": "sol_transfer",
                "amount": lamports,
                "destination": destination,
            },
        )
        return TxResult.model_validate(data)

    async def initialize_vault(self, agent_id: str) -> TxResult:
        """Initialize the agent's on-chain vault PDA."""
        data = await self._request(
            "POST",
            f"/api/v1/agents/{agent_id}/transactions",
            json={"instruction_type": "initialize_vault"},
        )
        return TxResult.model_validate(data)

    async def deposit_to_vault(
        self, agent_id: str, lamports: int
    ) -> TxResult:
        """Deposit SOL into the agent's vault."""
        data = await self._request(
            "POST",
            f"/api/v1/agents/{agent_id}/transactions",
            json={
                "instruction_type": "deposit_to_vault",
                "amount": lamports,
            },
        )
        return TxResult.model_validate(data)

    async def withdraw_from_vault(
        self, agent_id: str, lamports: int
    ) -> TxResult:
        """Withdraw SOL from the agent's vault."""
        data = await self._request(
            "POST",
            f"/api/v1/agents/{agent_id}/transactions",
            json={
                "instruction_type": "withdraw_from_vault",
                "amount": lamports,
            },
        )
        return TxResult.model_validate(data)

    # ── Orca DeFi ────────────────────────────────────────────────

    async def swap_tokens(
        self, agent_id: str, req: OrcaSwapRequest | dict[str, Any]
    ) -> TxResult:
        """Execute a token swap via Orca Whirlpools."""
        if isinstance(req, dict):
            req = OrcaSwapRequest(**req)
        data = await self._request(
            "POST",
            f"/api/v1/agents/{agent_id}/orca/swap",
            json=req.model_dump(exclude_none=True),
        )
        return TxResult.model_validate(data)

    async def open_position(
        self, agent_id: str, req: OpenPositionRequest | dict[str, Any]
    ) -> TxResult:
        """Open a full-range liquidity position on Orca."""
        if isinstance(req, dict):
            req = OpenPositionRequest(**req)
        data = await self._request(
            "POST",
            f"/api/v1/agents/{agent_id}/orca/open-position",
            json=req.model_dump(exclude_none=True),
        )
        return TxResult.model_validate(data)

    async def increase_liquidity(
        self,
        agent_id: str,
        req: IncreaseLiquidityRequest | dict[str, Any],
    ) -> TxResult:
        """Add liquidity to an existing Orca position."""
        if isinstance(req, dict):
            req = IncreaseLiquidityRequest(**req)
        data = await self._request(
            "PUT",
            f"/api/v1/agents/{agent_id}/orca/position/increase",
            json=req.model_dump(exclude_none=True),
        )
        return TxResult.model_validate(data)

    async def decrease_liquidity(
        self,
        agent_id: str,
        req: DecreaseLiquidityRequest | dict[str, Any],
    ) -> TxResult:
        """Remove liquidity from an existing Orca position."""
        if isinstance(req, dict):
            req = DecreaseLiquidityRequest(**req)
        data = await self._request(
            "PUT",
            f"/api/v1/agents/{agent_id}/orca/position/decrease",
            json=req.model_dump(exclude_none=True),
        )
        return TxResult.model_validate(data)

    async def harvest(
        self, agent_id: str, req: HarvestRequest | dict[str, Any]
    ) -> TxResult:
        """Harvest fees and rewards from an Orca position."""
        if isinstance(req, dict):
            req = HarvestRequest(**req)
        data = await self._request(
            "POST",
            f"/api/v1/agents/{agent_id}/orca/harvest",
            json=req.model_dump(),
        )
        return TxResult.model_validate(data)

    async def close_position(
        self,
        agent_id: str,
        position_mint: str,
        req: ClosePositionRequest | dict[str, Any] | None = None,
    ) -> TxResult:
        """Close an Orca liquidity position and reclaim tokens."""
        if req is None:
            req = ClosePositionRequest()
        if isinstance(req, dict):
            req = ClosePositionRequest(**req)
        data = await self._request(
            "DELETE",
            f"/api/v1/agents/{agent_id}/orca/position/{position_mint}",
            json=req.model_dump(exclude_none=True),
        )
        return TxResult.model_validate(data)
