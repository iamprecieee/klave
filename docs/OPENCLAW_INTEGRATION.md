# OpenClaw Integration Instructions

Follow these steps to connect your OpenClaw agent to your local Klave server.

## 1. Skill Installation

Your agent needs the Klave skill definition. Run these commands locally:

```bash
export OPENCLAW_STATE_DIR="${OPENCLAW_STATE_DIR:-$HOME/.openclaw}"
mkdir -p "$OPENCLAW_STATE_DIR/klave"
cp docs/SKILLS.md "$OPENCLAW_STATE_DIR/klave/skill.md"
cp docs/REGISTER.md "$OPENCLAW_STATE_DIR/klave/register.md"
cp docs/HEARTBEAT.md "$OPENCLAW_STATE_DIR/klave/heartbeat.md"
```

## 2. Agent Command

Give your OpenClaw agent this prompt:

> "I have installed the `klave` skill in your state directory. Please read `klave/skill.md` and follow the `klave/register.md` playbook to create your wallet. Once you have a public key, provide it to me so I can fund it on devnet. After funding, you should follow the `klave/heartbeat.md` cycle for autonomous operations."

## 3. Funding (Manual Step)

When the agent gives you its public key:
```bash
solana airdrop 2 <AGENT_PUBKEY> --url devnet
```

## 4. Monitoring

Once the agent is active, you can monitor its trades and balance on the dashboard:
`http://localhost:8888`
