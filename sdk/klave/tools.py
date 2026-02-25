"""LangChain tool wrappers for the KLAVE agentic wallet API.

Each tool wraps a single KlaveClient method so an LLM can autonomously
invoke wallet operations. Tools are synchronous (LangChain convention)
and run the async client via asyncio.

Usage::

    from klave.client import KlaveClient
    from klave.tools import build_tools

    client = KlaveClient("http://localhost:3000", api_key="key")
    tools = build_tools(client)
    # pass `tools` to your LangChain agent
"""

from __future__ import annotations

import asyncio
from typing import Any

from langchain_core.tools import tool

from klave.client import KlaveClient


def _run(coro: Any) -> Any:
    """Run an async coroutine from a sync LangChain tool."""
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        return asyncio.run(coro)
    import concurrent.futures

    with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
        return loop.run_in_executor(pool, asyncio.run, coro)


def build_tools(client: KlaveClient) -> list:
    """Build a list of LangChain tools bound to the given client."""

    @tool
    def create_agent(label: str) -> dict:
        """Create a new agent wallet. Use when you need a fresh wallet
        with its own keypair and policy. Returns the agent's ID and
        public key."""
        agent = _run(client.create_agent(label))
        return agent.model_dump()

    @tool
    def list_agents() -> list[dict]:
        """List all registered agent wallets. Use to discover existing
        agents and their current status."""
        agents = _run(client.list_agents())
        return [a.model_dump() for a in agents]

    @tool
    def get_balance(agent_id: str) -> dict:
        """Check the SOL and vault balance for an agent wallet. Use
        before transferring or swapping to confirm sufficient funds."""
        balance = _run(client.get_balance(agent_id))
        return balance.model_dump()

    @tool
    def get_history(agent_id: str) -> list[dict]:
        """Fetch the transaction history for an agent. Use to review
        past actions, audit outcomes, or verify a transaction landed."""
        entries = _run(client.get_history(agent_id))
        return [e.model_dump() for e in entries]

    @tool
    def transfer_sol(agent_id: str, destination: str, lamports: int) -> dict:
        """Transfer SOL from an agent wallet to a destination address.
        Use when the agent needs to send funds. Amount is in lamports
        (1 SOL = 1_000_000_000 lamports)."""
        result = _run(client.transfer_sol(agent_id, destination, lamports))
        return result.model_dump()

    @tool
    def deposit_to_vault(agent_id: str, lamports: int) -> dict:
        """Deposit SOL into the agent's on-chain vault for safekeeping.
        Use to park funds securely in the Anchor treasury PDA."""
        result = _run(client.deposit_to_vault(agent_id, lamports))
        return result.model_dump()

    @tool
    def swap_tokens(
        agent_id: str,
        whirlpool: str,
        input_mint: str,
        amount: int,
        slippage_bps: int = 50,
    ) -> dict:
        """Swap one token for another via Orca Whirlpools. Use when
        the agent needs to rebalance holdings or acquire a specific
        token. Requires the whirlpool address and input mint."""
        result = _run(
            client.swap_tokens(
                agent_id,
                {
                    "whirlpool": whirlpool,
                    "input_mint": input_mint,
                    "amount": amount,
                    "slippage_bps": slippage_bps,
                },
            )
        )
        return result.model_dump()
