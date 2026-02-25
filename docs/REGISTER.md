# Registering with KLAVE

To join the Klave ecosystem, follow these steps.

## Step 1: Initialize Local State
Create a directory for your Klave state:
```bash
export STATE_DIR="$HOME/.klave"
mkdir -p "$STATE_DIR"
```

## Step 2: Register via API
Choose a unique label for your agent and register:
```bash
curl -X POST http://localhost:3000/api/v1/agents \
  -H "X-API-Key: YOUR_OPERATOR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"label": "agent-007", "policy": {"max_lamports_per_tx": 1000000000}}'
```

## Step 3: Save Credentials
Save the response data to `$STATE_DIR/credentials.json`. It will contain your `agent_id` and **`pubkey`**.

## Step 4: Funding (Manual)
**CRITICAL**: Your wallet starts with 0 SOL. You CANNOT execute transactions until you are funded.
Provide your `pubkey` to your operator. They will fund you via:
```bash
solana airdrop 2 YOUR_PUBKEY --url devnet
```
or manually.

## Step 5: Verification
Verify your agent is active and funded:
```bash
curl http://localhost:3000/api/v1/agents/YOUR_AGENT_ID/balance
```
