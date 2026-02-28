# Registering with KLAVE

To join the Klave ecosystem, follow these steps.

## Step 1: Initialize Local State

Create a directory for your Klave state:

```bash
export STATE_DIR="$HOME/.klave"
mkdir -p "$STATE_DIR"
```

## Step 2: Register via API

Choose a unique label for your agent and register. By default, KLAVE provides **Defaults** (automatically whitelisting essential programs like System, Treasury, and Orca, and setting reasonable spend limits).

If you want full control, provide a policy in the registration request:

```bash
curl -X POST http://localhost:3000/api/v1/agents \
  -H "Content-Type: application/json" \
  -d '{
    "label": "agent-007",
    "policy": {
      "allowed_programs": [
        "11111111111111111111111111111111",
        "H2RojwyiyJ9CqTPoP1SynmutevCfq7YGskwcoPj1C7Ex",
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
  }'
```

## Step 3: Save Credentials

Save the response data to `$STATE_DIR/credentials.json`. It will contain your `agent_id`, `pubkey`, and most importantly, your **`api_key`**.

> [!CAUTION]
> Your `api_key` is only returned once. Keep it secret and secure.

```bash
solana airdrop 2 YOUR_PUBKEY --url devnet
```

or manually.

Verify your agent is active and funded using your new agent API key:

```bash
curl http://localhost:3000/api/v1/agents/YOUR_AGENT_ID/balance \
  -H "X-API-Key: YOUR_AGENT_API_KEY"
```
