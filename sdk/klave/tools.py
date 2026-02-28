"""LangChain tool wrappers for the KLAVE agentic wallet API.

Each tool wraps a single KlaveClient method so an LLM can autonomously
invoke wallet operations. Tools are natively async to ensure compatibility
with modern LangChain agents and async event loops.

Usage::

    from klave.client import KlaveClient
    from klave.tools import build_agent_tools

    client = KlaveClient("http://localhost:3000", api_key="key")
    tools = build_agent_tools(client)
    # pass `tools` to your LangChain agent
"""

from __future__ import annotations

import json
from pathlib import Path

from langchain_core.tools import tool

from klave.client import KlaveClient


def build_agent_tools(client: KlaveClient) -> list:
    """Build tools for an autonomous agent (excludes administrative tasks)."""
    tools = build_operator_tools(client)
    # Filter out admin-only tools
    admin_tools = {"update_policy", "delete_agent", "list_agents"}
    return [t for t in tools if t.name not in admin_tools]


def build_operator_tools(client: KlaveClient) -> list:
    """Build the full set of tools for a KLAVE operator."""

    @tool
    async def create_agent(label: str) -> dict:
        """Create a new agent wallet. Use to create a fresh wallet
        with its own keypair and policy. Returns the agent's ID and
        public key."""
        try:
            agent = await client.create_agent(label)
            return agent.model_dump()
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def list_agents() -> list[dict]:
        """List all registered agent wallets. Use to discover existing
        agents and their current status."""
        try:
            agents = await client.list_agents()
            return [agent.model_dump() for agent in agents]
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def get_balance(agent_id: str) -> dict:
        """Check the SOL and vault balance for an agent wallet. Use
        before transferring or swapping to confirm sufficient funds."""
        try:
            balance = await client.get_balance(agent_id)
            return balance.model_dump()
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def get_history(agent_id: str) -> list[dict]:
        """Fetch the transaction history for an agent. Use to review
        past actions, audit outcomes, or verify a transaction landed."""
        try:
            entries = await client.get_history(agent_id)
            return [e.model_dump() for e in entries]
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def transfer_sol(agent_id: str, destination: str, lamports: int) -> dict:
        """Transfer SOL from an agent wallet to a destination address.
        Use when the agent needs to send funds. Amount is in lamports
        (1 SOL = 1_000_000_000 lamports)."""
        try:
            result = await client.transfer_sol(agent_id, destination, lamports)
            return result.model_dump()
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def deposit_to_vault(agent_id: str, lamports: int) -> dict:
        """Deposit SOL into the agent's on-chain vault for safekeeping.
        Use to park funds securely in the Anchor treasury PDA."""
        try:
            result = await client.deposit_to_vault(agent_id, lamports)
            return result.model_dump()
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def swap_tokens(
        agent_id: str,
        whirlpool: str,
        input_mint: str,
        amount: int,
        slippage_bps: int = 50,
    ) -> dict:
        """Swap one token for another via Orca Whirlpools. Use when
        the agent needs to rebalance holdings or acquire a specific
        token. Requires the whirlpool address and input mint."""
        try:
            result = await client.swap_tokens(
                agent_id,
                {
                    "whirlpool": whirlpool,
                    "input_mint": input_mint,
                    "amount": amount,
                    "slippage_bps": slippage_bps,
                },
            )
            return result.model_dump()
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def initialize_vault(agent_id: str) -> dict:
        """Initialize the agent's on-chain vault PDA in the Anchor treasury
        program. Must be called once before any deposit or withdraw."""
        try:
            result = await client.initialize_vault(agent_id)
            return result.model_dump()
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def withdraw_from_vault(agent_id: str, lamports: int) -> dict:
        """Withdraw SOL from the agent's on-chain vault back to their
        wallet. Amount is in lamports (1 SOL = 1_000_000_000 lamports)."""
        try:
            result = await client.withdraw_from_vault(agent_id, lamports)
            return result.model_dump()
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def delete_agent(agent_id: str) -> str:
        """Deactivate an agent wallet. The keypair is never reused.
        Use when an agent should be permanently retired."""
        try:
            await client.delete_agent(agent_id)
            return "agent deactivated"
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def update_policy(
        agent_id: str,
        allowed_programs: list[str] | None = None,
        max_lamports_per_tx: int | None = None,
        token_allowlist: list[str] | None = None,
        daily_spend_limit_usd: float | None = None,
        daily_swap_volume_usd: float | None = None,
        slippage_bps: int | None = None,
        withdrawal_destinations: list[str] | None = None,
    ) -> dict:
        """Update the policy for an agent. Only the provided fields are
        changed; omitted fields keep their current values. Use to tighten
        or loosen agent permissions."""
        try:
            fields = {
                "allowed_programs": allowed_programs,
                "max_lamports_per_tx": max_lamports_per_tx,
                "token_allowlist": token_allowlist,
                "daily_spend_limit_usd": daily_spend_limit_usd,
                "daily_swap_volume_usd": daily_swap_volume_usd,
                "slippage_bps": slippage_bps,
                "withdrawal_destinations": withdrawal_destinations,
            }
            policy = {k: v for k, v in fields.items() if v is not None}
            result = await client.update_policy(agent_id, policy)
            return result
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def list_pools(token: str | None = None, limit: int = 20) -> dict:
        """List available Orca Whirlpools for discoverability. Use to find
        valid pool addresses for a specific token mint."""
        try:
            result = await client.list_pools(token, limit)
            return result
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def get_quote(
        agent_id: str,
        whirlpool: str,
        input_mint: str,
        amount: int,
        slippage_bps: int = 50,
    ) -> dict:
        """Fetch a simulated swap quote from Orca. Use BEFORE swapping
        to verify expected output and minimum received amount."""
        try:
            result = await client.get_quote(
                agent_id,
                {
                    "whirlpool": whirlpool,
                    "input_mint": input_mint,
                    "amount": amount,
                    "slippage_bps": slippage_bps,
                },
            )
            return result
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def get_health() -> dict[str, str]:
        """Check the system health status. Use to verify that the
        KLAVE infrastructure is operational before performing actions."""
        try:
            result = await client.get_health()
            return result
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def list_tokens(agent_id: str) -> list[dict]:
        """Fetch SPL token balances for an agent. Use to discover
        current token holdings before rebalancing."""
        try:
            result = await client.list_tokens(agent_id)
            return result
        except Exception as e:
            return f"[ERROR] {str(e)}"

    @tool
    async def save_heartbeat(
        sol_lamports: int,
        vault_lamports: int,
        token_count: int,
        action_taken: str,
        errors: int = 0,
    ) -> str:
        """Records the outcome of the current heartbeat cycle to a local
        state file. Use at the END of every cycle to maintain persistence."""
        try:
            state = {
                "last_check": int(Path(__file__).stat().st_mtime),
                "sol_lamports": sol_lamports,
                "vault_lamports": vault_lamports,
                "token_count": token_count,
                "action_taken": action_taken,
                "errors": errors,
            }
            path = Path.cwd() / "heartbeat.json"
            path.write_text(json.dumps(state, indent=2))
            return f"State saved to {path}"
        except Exception as e:
            return f"[ERROR] Failed to save state: {str(e)}"

    @tool
    async def wait_for_manual_funding(address: str) -> str:
        """Pause execution and prompt the user to fund the given Solana
        address. Use after creating a new agent or when funds are low.
        The user will notify you once funding is complete."""
        print(f"\n{'-'*40}")
        print(f"FUNDING REQUIRED: Please send devnet SOL to:")
        print(f"Address: {address}")
        print(f"{'-'*40}\n")
        input("Press Enter once you have funded the address...")
        return "User confirmed funding. Please check balance to verify."

    return [
        create_agent,
        list_agents,
        get_balance,
        get_history,
        transfer_sol,
        deposit_to_vault,
        swap_tokens,
        initialize_vault,
        withdraw_from_vault,
        delete_agent,
        update_policy,
        list_pools,
        get_quote,
        get_health,
        list_tokens,
        save_heartbeat,
        wait_for_manual_funding,
    ]
