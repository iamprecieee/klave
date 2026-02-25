#!/usr/bin/env python3
"""KLAVE Multi-Agent Demo — three autonomous agents on Solana devnet.

Each agent has a different risk profile and runs an independent decision
loop: check balance → decide action → execute → log → sleep → repeat.

Prerequisites:
    1. klave start --with-kora --dashboard  (server running)
    2. cd sdk && pip install -e .
    3. Agents will be auto-funded via devnet airdrop.

Usage:
    python sdk/demo/multi_agent_demo.py
"""

from __future__ import annotations

import asyncio
import os
import sys
import time
import subprocess

from klave.client import KlaveClient
from klave.models import AgentPolicyInput
from klave.exceptions import KlaveApiError, PolicyViolationError

# ── Config ────────────────────────────────────────────────────────

KLAVE_URL = os.getenv("KLAVE_URL", "http://localhost:3000")
KLAVE_API_KEY = os.getenv("KLAVE_API_KEY", "")
LOOP_INTERVAL = int(os.getenv("DEMO_INTERVAL", "10"))  # seconds between decisions
MAX_ROUNDS = int(os.getenv("DEMO_ROUNDS", "5"))  # decision rounds per agent

# Load API key from .env if not set
if not KLAVE_API_KEY:
    env_path = os.path.join(os.path.dirname(__file__), "..", "..", ".env")
    if os.path.exists(env_path):
        with open(env_path) as f:
            for line in f:
                if line.startswith("KLAVE_API_KEY="):
                    KLAVE_API_KEY = line.strip().split("=", 1)[1]
                    break

# USDC and WSOL mints on devnet
USDC_MINT = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
WSOL_MINT = "So11111111111111111111111111111111111111112"

# ── Agent profiles ────────────────────────────────────────────────

AGENTS = [
    {
        "label": "alpha-conservative",
        "policy": AgentPolicyInput(
            max_lamports_per_tx=500_000_000,  # 0.5 SOL
            daily_spend_limit_usd=100.0,
            token_allowlist=[USDC_MINT, WSOL_MINT],
            slippage_bps=30,
        ),
        "strategy": "saver",  # deposits excess SOL into vault
    },
    {
        "label": "beta-moderate",
        "policy": AgentPolicyInput(
            max_lamports_per_tx=1_000_000_000,  # 1 SOL
            daily_spend_limit_usd=500.0,
            token_allowlist=[USDC_MINT, WSOL_MINT],
            slippage_bps=50,
        ),
        "strategy": "trader",  # transfers SOL between agents
    },
    {
        "label": "gamma-aggressive",
        "policy": AgentPolicyInput(
            max_lamports_per_tx=2_000_000_000,  # 2 SOL
            daily_spend_limit_usd=2000.0,
            token_allowlist=[USDC_MINT, WSOL_MINT],
            slippage_bps=100,
        ),
        "strategy": "whale",  # deposits heavily, withdraws strategically
    },
]

# ── Helpers ───────────────────────────────────────────────────────

SOL = 1_000_000_000  # lamports per SOL


def log(agent_label: str, msg: str) -> None:
    ts = time.strftime("%H:%M:%S")
    color = {
        "alpha-conservative": "\033[36m",  # cyan
        "beta-moderate": "\033[33m",  # yellow
        "gamma-aggressive": "\033[35m",  # magenta
    }.get(agent_label, "\033[0m")
    print(f"  {color}[{ts}] [{agent_label}]\033[0m {msg}")


def sol_fmt(lamports: int) -> str:
    return f"{lamports / SOL:.4f} SOL"


async def airdrop(pubkey: str, amount_sol: int = 2) -> bool:
    """Request a devnet airdrop via the Solana CLI."""
    try:
        proc = await asyncio.create_subprocess_exec(
            "solana", "airdrop", str(amount_sol), pubkey, "--url", "devnet",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()
        return proc.returncode == 0
    except FileNotFoundError:
        return False


# ── Decision loops ────────────────────────────────────────────────

async def saver_loop(client: KlaveClient, agent_id: str, label: str) -> None:
    """Conservative: deposits excess SOL into vault for safekeeping."""
    for round_num in range(1, MAX_ROUNDS + 1):
        log(label, f"round {round_num}/{MAX_ROUNDS}")
        try:
            balance = await client.get_balance(agent_id)
            log(label, f"wallet: {sol_fmt(balance.sol_lamports)} | vault: {sol_fmt(balance.vault_lamports)}")

            # If wallet has more than 0.5 SOL, deposit half the excess into vault
            threshold = int(0.5 * SOL)
            if balance.sol_lamports > threshold:
                deposit_amount = (balance.sol_lamports - threshold) // 2
                if deposit_amount > 10_000:  # minimum meaningful deposit
                    log(label, f"depositing {sol_fmt(deposit_amount)} into vault...")
                    result = await client.deposit_to_vault(agent_id, deposit_amount)
                    log(label, f"✓ deposited — sig: {result.signature[:16]}... via_kora: {result.via_kora}")
                else:
                    log(label, "balance too low to deposit, holding")
            else:
                log(label, "below threshold, holding position")
        except PolicyViolationError as e:
            log(label, f"✗ policy blocked: {e}")
        except KlaveApiError as e:
            log(label, f"✗ API error: {e}")
        except Exception as e:
            log(label, f"✗ unexpected: {e}")

        if round_num < MAX_ROUNDS:
            await asyncio.sleep(LOOP_INTERVAL)


async def trader_loop(
    client: KlaveClient,
    agent_id: str,
    label: str,
    peer_pubkeys: list[str],
) -> None:
    """Moderate: transfers small amounts of SOL between peer agents."""
    for round_num in range(1, MAX_ROUNDS + 1):
        log(label, f"round {round_num}/{MAX_ROUNDS}")
        try:
            balance = await client.get_balance(agent_id)
            log(label, f"wallet: {sol_fmt(balance.sol_lamports)} | vault: {sol_fmt(balance.vault_lamports)}")

            # Transfer 0.05 SOL to a peer if we have enough
            transfer_amount = int(0.05 * SOL)
            if balance.sol_lamports > transfer_amount + int(0.1 * SOL):
                peer = peer_pubkeys[round_num % len(peer_pubkeys)]
                log(label, f"transferring {sol_fmt(transfer_amount)} to {peer[:8]}...")
                result = await client.transfer_sol(agent_id, peer, transfer_amount)
                log(label, f"✓ transferred — sig: {result.signature[:16]}... via_kora: {result.via_kora}")
            else:
                log(label, "insufficient balance for transfer, skipping")
        except PolicyViolationError as e:
            log(label, f"✗ policy blocked: {e}")
        except KlaveApiError as e:
            log(label, f"✗ API error: {e}")
        except Exception as e:
            log(label, f"✗ unexpected: {e}")

        if round_num < MAX_ROUNDS:
            await asyncio.sleep(LOOP_INTERVAL)


async def whale_loop(client: KlaveClient, agent_id: str, label: str) -> None:
    """Aggressive: deposits heavily, then withdraws strategically."""
    for round_num in range(1, MAX_ROUNDS + 1):
        log(label, f"round {round_num}/{MAX_ROUNDS}")
        try:
            balance = await client.get_balance(agent_id)
            log(label, f"wallet: {sol_fmt(balance.sol_lamports)} | vault: {sol_fmt(balance.vault_lamports)}")

            if round_num <= MAX_ROUNDS // 2:
                # First half: deposit aggressively
                deposit = int(balance.sol_lamports * 0.4)
                if deposit > 10_000:
                    log(label, f"depositing {sol_fmt(deposit)} into vault (aggressive)...")
                    result = await client.deposit_to_vault(agent_id, deposit)
                    log(label, f"✓ deposited — sig: {result.signature[:16]}... via_kora: {result.via_kora}")
                else:
                    log(label, "wallet nearly empty, skipping deposit")
            else:
                # Second half: withdraw from vault
                if balance.vault_lamports > 10_000:
                    withdraw = balance.vault_lamports // 3
                    if withdraw > 10_000:
                        log(label, f"withdrawing {sol_fmt(withdraw)} from vault...")
                        result = await client.withdraw_from_vault(agent_id, withdraw)
                        log(label, f"✓ withdrew — sig: {result.signature[:16]}... via_kora: {result.via_kora}")
                    else:
                        log(label, "vault amount too small to withdraw")
                else:
                    log(label, "vault empty, nothing to withdraw")
        except PolicyViolationError as e:
            log(label, f"✗ policy blocked: {e}")
        except KlaveApiError as e:
            log(label, f"✗ API error: {e}")
        except Exception as e:
            log(label, f"✗ unexpected: {e}")

        if round_num < MAX_ROUNDS:
            await asyncio.sleep(LOOP_INTERVAL)


# ── Main ──────────────────────────────────────────────────────────

async def main() -> None:
    print()
    print("\033[1mKLAVE Multi-Agent Demo\033[0m")
    print(f"  server:   {KLAVE_URL}")
    print(f"  rounds:   {MAX_ROUNDS}")
    print(f"  interval: {LOOP_INTERVAL}s")
    print()

    if not KLAVE_API_KEY:
        print("\033[31m  ✗ KLAVE_API_KEY not found. Set it in .env or as an environment variable.\033[0m")
        sys.exit(1)

    async with KlaveClient(KLAVE_URL, api_key=KLAVE_API_KEY) as client:
        # ── 1. Create agents ──────────────────────────────────
        print("\033[1m  Phase 1: Creating agents\033[0m")
        agents = []
        for spec in AGENTS:
            try:
                agent = await client.create_agent(spec["label"], spec["policy"])
                log(spec["label"], f"created — id: {agent.id[:8]}... pubkey: {agent.pubkey[:12]}...")
                agents.append({"agent": agent, **spec})
            except KlaveApiError as e:
                print(f"\033[31m  ✗ Failed to create {spec['label']}: {e}\033[0m")
                return
        print()

        # ── 2. Fund agents via airdrop ────────────────────────
        print("\033[1m  Phase 2: Funding agents (devnet airdrop)\033[0m")
        for entry in agents:
            agent = entry["agent"]
            ok = await airdrop(agent.pubkey)
            if ok:
                log(entry["label"], f"airdrop 2 SOL → {agent.pubkey[:12]}... ✓")
            else:
                log(entry["label"], "airdrop failed — fund manually:")
                log(entry["label"], f"  solana airdrop 2 {agent.pubkey} --url devnet")

        # Wait for airdrops to confirm
        print("  waiting for confirmations...")
        await asyncio.sleep(5)
        print()

        # ── 3. Initialize vaults ──────────────────────────────
        print("\033[1m  Phase 3: Initializing vaults\033[0m")
        for entry in agents:
            agent = entry["agent"]
            try:
                result = await client.initialize_vault(agent.id)
                log(entry["label"], f"vault initialized — sig: {result.signature[:16]}... via_kora: {result.via_kora}")
            except KlaveApiError as e:
                log(entry["label"], f"vault init skipped: {e}")
        print()

        # ── 4. Run autonomous decision loops ──────────────────
        print(f"\033[1m  Phase 4: Running {MAX_ROUNDS} decision rounds\033[0m")
        print()

        peer_pubkeys = [e["agent"].pubkey for e in agents]

        tasks = []
        for entry in agents:
            agent = entry["agent"]
            label = entry["label"]
            strategy = entry["strategy"]

            if strategy == "saver":
                tasks.append(saver_loop(client, agent.id, label))
            elif strategy == "trader":
                others = [p for p in peer_pubkeys if p != agent.pubkey]
                tasks.append(trader_loop(client, agent.id, label, others))
            elif strategy == "whale":
                tasks.append(whale_loop(client, agent.id, label))

        await asyncio.gather(*tasks)
        print()

        # ── 5. Final report ───────────────────────────────────
        print("\033[1m  Summary\033[0m")
        for entry in agents:
            agent = entry["agent"]
            try:
                balance = await client.get_balance(agent.id)
                history = await client.get_history(agent.id)
                confirmed = sum(1 for h in history if h.status == "confirmed")
                rejected = sum(1 for h in history if h.status == "rejected")
                log(
                    entry["label"],
                    f"wallet: {sol_fmt(balance.sol_lamports)} | "
                    f"vault: {sol_fmt(balance.vault_lamports)} | "
                    f"txns: {confirmed} confirmed, {rejected} rejected",
                )
            except Exception as e:
                log(entry["label"], f"could not fetch summary: {e}")

        print()
        print("  \033[32m✓ Demo complete.\033[0m Open http://localhost:8888 to see the dashboard.")
        print()


if __name__ == "__main__":
    asyncio.run(main())
