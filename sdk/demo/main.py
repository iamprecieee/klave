#!/usr/bin/env python3
"""
KLAVE Agent Demo
========================================

Shows how to empower an LLM with full wallet capabilities:
  1. Check Balances (SOL & Vault)
  2. Swap Tokens (Orca Whirlpools)
  3. Transfer SOL (Standard Transfers)
  4. Secure Deposits (Anchor Vault)

Supports: OpenAI, Google Gemini, and a robust Simulation Mode.
Usage:
  uv venv
  source .venv/bin/activate
  uv pip install -e ".[langchain]"
  uv run --active klave-sdk-demo --api-key <key>
"""

import argparse
import asyncio
import os
import sys
from pathlib import Path

from dotenv import load_dotenv

sdk_dir = Path(__file__).resolve().parents[1]
if str(sdk_dir) not in sys.path:
    sys.path.insert(0, str(sdk_dir))

skills_path = Path(__file__).resolve().parents[2] / "docs" / "SKILLS.md"
heartbeat_path = Path(__file__).resolve().parents[2] / "docs" / "HEARTBEAT.md"

skills_text = (
    skills_path.read_text()
    if skills_path.exists()
    else "Technical documentation missing."
)
heartbeat_text = (
    heartbeat_path.read_text() if heartbeat_path.exists() else "Playbook missing."
)

# Escape curly braces for LangChain prompt templates
skills_text = skills_text.replace("{", "{{").replace("}", "}}")
heartbeat_text = heartbeat_text.replace("{", "{{").replace("}", "}}")

try:
    for path in [Path.cwd(), Path(__file__).resolve().parents[2], Path.cwd().parent]:
        env_file = (path / ".env").resolve()
        if env_file.exists():
            load_dotenv(dotenv_path=env_file, override=True)
            break
except ImportError:
    pass

try:
    from langchain.agents import AgentExecutor, create_openai_tools_agent
    from langchain_core.prompts import ChatPromptTemplate, MessagesPlaceholder
    from langchain_openai import ChatOpenAI
    from langchain_google_genai import ChatGoogleGenerativeAI

    LANGCHAIN_AVAILABLE = True
except ImportError as e:
    if os.getenv("DEBUG"):
        print(f"DEBUG: LangChain import failed: {e}")
    LANGCHAIN_AVAILABLE = False

from klave import KlaveClient, AgentPolicyInput
from klave.tools import build_agent_tools, build_operator_tools

from .utils import (
    flow_start,
    flow_line,
    flow_step,
    flow_done,
    flow_end,
    SOL_MINT,
    USDC_MINT,
    _color,
    _status,
    AMBER,
    BLUE,
    TEAL,
    FlowCallbackHandler,
)
from .simulation import run_simulation


async def entry():
    parser = argparse.ArgumentParser()
    parser.add_argument("--api-key", required=False)
    parser.add_argument("--operator-key", required=False)
    parser.add_argument("--base-url", default="http://localhost:3000")
    args = parser.parse_args()

    api_key = args.api_key or os.getenv("KLAVE_API_KEY")
    operator_key = args.operator_key or os.getenv("KLAVE_OPERATOR_API_KEY")

    flow_start("initialization")

    llm = None
    if not LANGCHAIN_AVAILABLE:
        flow_line("LangChain dependencies not found.")
        flow_line("Install with: pip install 'klave[langchain]'")
        _status("Continuing with internal simulation logic...", "warn")
    elif os.getenv("GOOGLE_API_KEY"):
        _status(f"Using Google Gemini ({_color(TEAL, 'Free Tier')})")
        llm = ChatGoogleGenerativeAI(model="gemini-2.5-flash")
    elif os.getenv("OPENAI_API_KEY"):
        _status(f"Using OpenAI ({_color(TEAL, 'Paid Tier')})")
        llm = ChatOpenAI(model="gpt-4o", temperature=0)
    else:
        _status("No LLM key found. Falling back to simulation.", "warn")

    async with KlaveClient(
        args.base_url, api_key=api_key, operator_api_key=operator_key or ""
    ) as client:
        if LANGCHAIN_AVAILABLE and llm:
            flow_step("LLM Agent Initializing...")
            tools = build_agent_tools(client)

            # Check if we have an agent already
            existing_agent = None
            if api_key or operator_key:
                try:
                    agents = await client.list_agents()
                    existing_agent = next(
                        (a for a in agents if a.label == "demo-agent-v1"), None
                    )
                except Exception:
                    pass

            agent_context = f"Your ID is {existing_agent.id if existing_agent else 'UNKNOWN'}.\n"
            if existing_agent:
                agent_context += f"Your Public Key is {existing_agent.pubkey}.\n"

            prompt = ChatPromptTemplate.from_messages(
                [
                    (
                        "system",
                        "You are a fully autonomous KLAVE wallet agent.\n\n"
                        f"{agent_context}\n"
                        "### INITIAL SETUP INSTRUCTIONS:\n"
                        "1. If your ID is UNKNOWN, you MUST call `create_agent` with label 'demo-agent-v1' to register yourself first.\n"
                        "2. After registration, if your SOL balance is 0, you MUST call `wait_for_manual_funding` to ask the user for funds.\n"
                        "3. Once funded, proceed to your goal.\n\n"
                        "### GOAL: Maintain your financial health by following the HEARTBEAT playbook.\n\n"
                        "### DECISION LOGIC (HEARTBEAT Playbook):\n"
                        f"{heartbeat_text}\n\n"
                        "### TECHNICAL REFERENCE (SKILLS API):\n"
                        f"{skills_text}\n\n"
                        "INSTRUCTIONS:\n"
                        "1. Analyze your current state (health, balance, tokens).\n"
                        "2. Compare against the decision flowchart in the HEARTBEAT playbook.\n"
                        "3. Execute ONLY the necessary actions to remain in optimal sync (deposit/withdraw/rebalance).\n"
                        "4. ALWAYS conclude your cycle by calling `save_heartbeat` with the final state metrics.\n"
                        "5. Conclude ONLY after you have reached a stable state according to the playbook.",
                    ),
                    ("human", "{input}"),
                    MessagesPlaceholder(variable_name="agent_scratchpad"),
                ]
            )

            try:
                executor = AgentExecutor(
                    agent=create_openai_tools_agent(llm, tools, prompt),
                    tools=tools,
                    verbose=False,
                    handle_parsing_errors=True,
                    max_iterations=12, 
                )

                result = await executor.ainvoke(
                    {
                        "input": "Initialize yourself if needed, then perform your heartbeat cycle."
                    },
                    {"callbacks": [FlowCallbackHandler()]},
                )

                flow_line("")
                flow_done(f"Final Result: {result['output']}")

            except Exception as e:
                _status(f"LLM Error: {str(e)}", "err")
                # Attempt simulation fallback if we have an agent
                if existing_agent:
                    await run_simulation(client, existing_agent.id)
        else:
            # Traditional simulation fallback
            flow_step("Configuring Agent (Simulation)...")
            agent = None
            if api_key or operator_key:
                try:
                    agents = await client.list_agents()
                    agent = next((a for a in agents if a.label == "demo-agent-v1"), None)
                except Exception:
                    pass

            policy = AgentPolicyInput(
                allowed_programs=["11111111111111111111111111111111", "GCU8h2yUZKPKemrxGu4tZoiiiUdhWeSonaWCgYbZaRBx", "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"],
                token_allowlist=[SOL_MINT, USDC_MINT],
                max_lamports_per_tx=1_000_000_000,
                daily_spend_limit_usd=100.0,
                daily_swap_volume_usd=500.0,
                slippage_bps=50,
            )

            if not agent:
                agent = await client.create_agent("demo-agent-v1", policy)
                flow_line(f"Registered new agent: {_color(TEAL, '[NEW]')}")
            
            balance = await client.get_balance(agent.id)
            if balance.sol_lamports < 50000000:
                flow_line(f"Fund: {agent.pubkey}")
                input("Press Enter once funded...")
                await asyncio.sleep(5)

            await run_simulation(client, agent.id)

    flow_step("Cleanup...")
    flow_done("Agent session wrapped.")
    flow_end("Session complete.")


def run_demo():
    """Synchronous entry point for the klave-sdk-demo command."""
    try:
        asyncio.run(entry())
    except KeyboardInterrupt:
        sys.exit(0)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    run_demo()
