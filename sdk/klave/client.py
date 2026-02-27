"""Async HTTP client for the KLAVE agentic wallet API.

Usage::

    async with KlaveClient("http://localhost:3000", api_key="key") as client:
        agent = await client.create_agent("trader-1", AgentPolicyInput())
        balance = await client.get_balance(agent.id)
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
    CreateAgentRequest,
    OrcaSwapRequest,
    TxResult,
)


class KlaveClient:
    """Typed async wrapper over the KLAVE REST API."""

    def __init__(
        self,
        base_url: str,
        api_key: str = "",
        operator_api_key: str = "",
        timeout: float = 30.0,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._api_key = api_key
        self._operator_api_key = operator_api_key
        self._timeout = timeout
        self._http = httpx.AsyncClient(
            base_url=self._base_url,
            timeout=timeout,
        )

    def _build_headers(self, use_operator_key: bool = False) -> dict[str, str]:
        headers: dict[str, str] = {"content-type": "application/json"}
        key = self._operator_api_key if use_operator_key else self._api_key
        if key:
            headers["x-api-key"] = key
        return headers

    async def __aenter__(self) -> KlaveClient:
        return self

    async def __aexit__(self, *exc: object) -> None:
        await self.close()

    async def close(self) -> None:
        await self._http.aclose()

    async def _request(
        self,
        method: str,
        path: str,
        *,
        json: dict[str, Any] | None = None,
        use_operator_key: bool = False,
    ) -> Any:
        """Send request, unwrap KLAVE envelope, raise on errors."""
        try:
            headers = self._build_headers(use_operator_key=use_operator_key)
            response = await self._http.request(
                method, path, json=json, headers=headers
            )
        except httpx.TimeoutException as exc:
            raise KlaveConnectionError(f"request timed out: {exc}") from exc
        except httpx.ConnectError as exc:
            raise KlaveConnectionError(str(exc)) from exc

        if response.status_code == 204:
            return None

        try:
            body = response.json()
        except Exception:
            if not response.is_success:
                raise KlaveApiError(response.status_code, response.text)
            raise KlaveConnectionError(f"non-JSON response: {response.text[:200]}")

        if not response.is_success:
            message = body.get("message", response.text)
            if response.status_code == 403:
                raise PolicyViolationError(message)
            raise KlaveApiError(response.status_code, message)

        return body.get("data")

    async def get_health(self) -> dict[str, str]:
        """Check the system health status."""
        return await self._request("GET", "/health")

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
            "POST", "/api/v1/agents", json=payload.model_dump(), use_operator_key=False
        )
        return Agent.model_validate(data)

    async def list_agents(self) -> list[Agent]:
        """List all registered agents."""
        data = await self._request("GET", "/api/v1/agents", use_operator_key=False)
        return [Agent.model_validate(item) for item in data]

    async def delete_agent(self, agent_id: str) -> None:
        """Deactivate an agent by ID."""
        await self._request(
            "DELETE", f"/api/v1/agents/{agent_id}", use_operator_key=True
        )

    async def get_history(self, agent_id: str) -> list[AuditEntry]:
        """Fetch the audit log for an agent."""
        data = await self._request("GET", f"/api/v1/agents/{agent_id}/history")
        return [AuditEntry.model_validate(item) for item in data]

    async def get_balance(self, agent_id: str) -> AgentBalance:
        """Fetch the SOL and vault balance for an agent."""
        data = await self._request("GET", f"/api/v1/agents/{agent_id}/balance")
        return AgentBalance.model_validate(data)

    async def list_tokens(self, agent_id: str) -> list[dict[str, Any]]:
        """Fetch SPL token balances for an agent."""
        data = await self._request("GET", f"/api/v1/agents/{agent_id}/tokens")
        return data

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
            use_operator_key=True,
        )
        return data

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

    async def deposit_to_vault(self, agent_id: str, lamports: int) -> TxResult:
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

    async def withdraw_from_vault(self, agent_id: str, lamports: int) -> TxResult:
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

    async def list_pools(
        self, token: str | None = None, limit: int = 20
    ) -> dict[str, Any]:
        """List available Orca Whirlpools, optionally filtered by token mint."""
        params = {}
        if token:
            params["token"] = token
        if limit:
            params["limit"] = limit

        query = "&".join(f"{k}={v}" for k, v in params.items())
        path = f"/api/v1/orca/pools?{query}" if query else "/api/v1/orca/pools"
        return await self._request("GET", path)

    async def get_quote(
        self, agent_id: str, req: OrcaSwapRequest | dict[str, Any]
    ) -> dict[str, Any]:
        """Fetch a simulated swap quote from Orca."""
        if isinstance(req, dict):
            req = OrcaSwapRequest(**req)
        return await self._request(
            "POST",
            f"/api/v1/agents/{agent_id}/orca/quote",
            json=req.model_dump(exclude_none=True),
        )

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
