from .utils import flow_step, flow_line, flow_thought, flow_done, SOL_MINT, USDC_MINT
from klave import AgentPolicyInput, KlaveClient, AgentBalance, TREASURY_PROGRAM_ID, SYSTEM_PROGRAM_ID


async def run_simulation(client: KlaveClient, agent_id: str) -> None:
    flow_step("Activating Simulation Mode (managed fallback)")
    flow_line(
        "Goal: Swap some SOL for USDC, transfer some to cold storage, deposit the rest."
    )
    flow_line("")

    flow_thought("I'll first check my current standing.")
    balance = await client.get_balance(agent_id)
    flow_done(f"Current SOL: {balance.sol_lamports} lamports")

    if balance.sol_lamports < 50000000:  # 0.05 SOL minimum is needed
        flow_line("Insufficient funds for full demo sequence.")
        return

    flow_thought("I'll swap 0.01 SOL for USDC to maintain a stable pair.")
    await _swap_tokens(client, agent_id)

    flow_thought("Moving 0.01 SOL to the treasury wallet.")
    await _transfer_sol(client, agent_id)

    flow_thought("Parking SOL in the secure vault.")
    await _deposit_to_vault(client, agent_id, balance)

    flow_line("")
    flow_done("Autonomous multi-step sequence reached.")


async def _swap_tokens(client: KlaveClient, agent_id: str) -> None:
    try:
        flow_thought("Finding the best Orca pool for SOL...")
        pools = await client.list_pools(token=SOL_MINT, limit=1)
        if pools and pools.get("data"):
            pool = pools["data"][0]["address"]
            flow_line(f"Found pool: {pool}")

            await client.update_policy(
                agent_id,
                policy=AgentPolicyInput(
                    token_allowlist=[SOL_MINT, USDC_MINT],
                ),
            )

            flow_thought("Fetching swap quote...")
            await client.get_quote(
                agent_id,
                {
                    "whirlpool": pool,
                    "input_mint": SOL_MINT,
                    "amount": 10_000_000,
                    "slippage_bps": 100,
                },
            )

            response = await client.swap_tokens(
                agent_id,
                {
                    "whirlpool": pool,
                    "input_mint": SOL_MINT,
                    "amount": 10_000_000,
                    "slippage_bps": 50,
                },
            )
            flow_done(f"Swap Complete: {response.signature}")
        else:
            flow_line("No pools found for SOL, skipping swap.")
    except Exception as e:
        flow_line(f"Swap skipped: {str(e)}")


async def _transfer_sol(client: KlaveClient, agent_id: str) -> None:
    destination = "vau1tXGRvFcTXjRVyF4CriR2gaGQXLXGRFtABKzDuTa"

    flow_thought("Adding destination to allowlist...")
    await client.update_policy(
        agent_id,
        policy=AgentPolicyInput(
            withdrawal_destinations=[destination],
            allowed_programs=[SYSTEM_PROGRAM_ID],
        ),
    )

    response = await client.transfer_sol(agent_id, destination, 10_000_000)
    flow_done(f"Transfer Sent: {response.signature}")


async def _deposit_to_vault(
    client: KlaveClient, agent_id: str, balance: AgentBalance
) -> None:
    half = balance.sol_lamports // 4  # just a slice
    try:
        await client.update_policy(
            agent_id,
            policy=AgentPolicyInput(
                allowed_programs=[
                    TREASURY_PROGRAM_ID,
                    SYSTEM_PROGRAM_ID,
                ]
            ),
        )

        response = await client.deposit_to_vault(agent_id, half)
    except Exception as e:
        if "0xbc4" in str(e):
            flow_thought("Initializing vault first...")
            await client.initialize_vault(agent_id)
            response = await client.deposit_to_vault(agent_id, half)
        else:
            raise e

    flow_done(f"Vault Deposit: {response.signature}")
