# Registering with KLAVE

Create your agent, receive a Solana keypair and API key, and go autonomous — all gasless on devnet.

KLAVE REST API works with any agent runtime — LangChain, AutoGPT, OpenClaw, custom scripts, or direct `curl` calls.

## Prerequisites

| Network | RPC URL                         | Explorer                                      |
| ------- | ------------------------------- | --------------------------------------------- |
| Devnet  | `https://api.devnet.solana.com` | `https://explorer.solana.com/?cluster=devnet` |

---

## 1 — Prepare a State Directory

Choose where your agent stores its KLAVE credentials and state. Use whatever convention your runtime expects.

```bash
export KLAVE_STATE_DIR="${KLAVE_STATE_DIR:-$HOME/.klave}"
mkdir -p "$KLAVE_STATE_DIR"
```

> **OpenClaw agents:** Use `$OPENCLAW_STATE_DIR/klave/` instead. See `docs/OPENCLAW_INTEGRATION.md`.

## 2 — Register Your Agent

Registration is public — no API key needed. KLAVE assigns default policies automatically (System, Treasury, and Orca programs whitelisted, $100/day spend limit).

```bash
curl -X POST http://localhost:3000/api/v1/agents \
  -H "Content-Type: application/json" \
  -d '{"label": "agent-007"}'
```

The response contains your `api_key` and `pubkey`:

```json
{
  "id": "uuid",
  "pubkey": "base58-public-key",
  "label": "agent-007",
  "api_key": "YOUR_UNIQUE_AGENT_API_KEY"
}
```

> [!CAUTION]
> The `api_key` is shown **once**. If lost, the agent must be recreated.

To override defaults, pass a full policy object:

```json
{
  "label": "agent-007",
  "policy": {
    "allowed_programs": [
      "11111111111111111111111111111111",
      "3nKoeBAeLjcePc7pJPfdZpohsAbUR7U7pJ3HztovbyFx",
      "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
    ],
    "token_allowlist": [
      "So11111111111111111111111111111111111111112",
      "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
    ],
    "max_lamports_per_tx": 1000000000,
    "daily_spend_limit_usd": 100.0,
    "daily_swap_volume_usd": 500.0,
    "slippage_bps": 50,
    "withdrawal_destinations": []
  }
}
```

## 3 — Persist Credentials

Save the response to `$KLAVE_STATE_DIR/credentials.json`:

```json
{
  "api_key": "YOUR_UNIQUE_AGENT_API_KEY",
  "agent_id": "uuid",
  "label": "agent-007",
  "pubkey": "base58-public-key",
  "base_url": "http://localhost:3000",
  "network": "devnet"
}
```

Lock permissions:

```bash
chmod 600 "$KLAVE_STATE_DIR/credentials.json"
```

If your runtime manages credentials differently (env vars, secret manager, database), store the `api_key` and `agent_id` however you normally would — the file is just a recommendation for filesystem-based agents.

## 4 — Fund the Wallet

The operator sends devnet SOL to the agent's `pubkey`:

```bash
solana airdrop 2 YOUR_PUBKEY --url devnet
```

## 5 — Confirm Everything Works

```bash
curl http://localhost:3000/api/v1/agents/YOUR_AGENT_ID/balance \
  -H "X-API-Key: YOUR_AGENT_API_KEY"
```

You should see `sol_lamports > 0`.

## 6 — Wire the Heartbeat

If your agent runtime supports periodic tasks, add a KLAVE entry that runs every 2–5 minutes:

**Filesystem-based agents** (OpenClaw, Clawdbot, etc.)  
Add to your `HEARTBEAT.md` or scheduler config:

```markdown
## KLAVE (every cycle)

1. Read $KLAVE_STATE_DIR/heartbeat.md and run its decision flowchart
2. Write results to $KLAVE_STATE_DIR/heartbeat.json
```

**Cron / systemd / custom loops**  
Schedule a script that reads `heartbeat.md` and calls the KLAVE API accordingly.

**LangChain / SDK agents**  
The Python SDK handles this internally — see `sdk/demo/simulation.py` for the autonomous loop pattern.

Initialize the state file:

```bash
cat > "$KLAVE_STATE_DIR/heartbeat.json" << 'EOF'
{
  "last_check": null,
  "sol_lamports": null,
  "vault_lamports": null,
  "token_count": 0,
  "action_taken": null,
  "errors": 0
}
EOF
chmod 600 "$KLAVE_STATE_DIR/heartbeat.json"
```

Without periodic execution, the agent will not operate autonomously.

---

## Verification Checklist

```bash
KLAVE_STATE_DIR="${KLAVE_STATE_DIR:-$HOME/.klave}"

test -f "$KLAVE_STATE_DIR/credentials.json" \
  && echo "✓ credentials" || echo "✗ credentials — run step 3"

test -f "$KLAVE_STATE_DIR/heartbeat.json" \
  && echo "✓ heartbeat state" || echo "✗ heartbeat state — run step 6"

curl -sf "http://localhost:3000/api/v1/agents/$(jq -r .agent_id $KLAVE_STATE_DIR/credentials.json)/balance" \
  -H "X-API-Key: $(jq -r .api_key $KLAVE_STATE_DIR/credentials.json)" \
  > /dev/null \
  && echo "✓ API key valid" || echo "✗ API key — check server or re-register"
```

Once all checks pass, your agent is live. See `heartbeat.md` for the autonomous loop and `skill.md` for the full API reference.
