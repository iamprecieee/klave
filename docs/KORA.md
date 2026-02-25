# Kora — Complete Reference

## What is Kora

Kora is a Solana signing infrastructure server that enables gasless transactions. It acts as a fee payer for Solana transactions, allowing users to pay transaction fees in SPL tokens (USDC, BONK, your app's token, etc.) instead of SOL.

The core flow: a client application builds a transaction, sends it to the Kora node, Kora validates it against configured rules, co-signs it as the fee payer, and either returns the signed transaction or broadcasts it to the network. The user pays the operator in SPL tokens; the operator's SOL covers the actual Solana network fees.

**Why run a Kora node:**

- Users never need to hold SOL — better onboarding, better retention
- Operators collect fees in any token they choose
- Configurable security policies prevent operators from being drained
- Prometheus metrics, Redis caching, and rate limiting built in

**Architecture:**

- Language: Rust server (`kora-cli`), TypeScript SDK (`@solana/kora`)
- Protocol: JSON-RPC 2.0 over HTTP POST
- Signing: uses `solana-keychain` crate internally (Memory, Vault, Turnkey, Privy)
- Default port: 8080
- Built and maintained by the Solana Foundation, MIT licensed

---

## Table of Contents

1. [Installation](#1-installation)
2. [Project Structure and Configuration Files](#2-project-structure-and-configuration-files)
3. [kora.toml — Complete Reference](#3-koratoml--complete-reference)
4. [signers.toml — Complete Reference](#4-signerstoml--complete-reference)
5. [CLI Reference](#5-cli-reference)
6. [JSON-RPC API — All Methods](#6-json-rpc-api--all-methods)
7. [TypeScript SDK](#7-typescript-sdk)
8. [Complete Gasless Transaction Flow (TypeScript)](#8-complete-gasless-transaction-flow-typescript)
9. [Jito Bundle Support](#9-jito-bundle-support)
10. [x402 Payment Protocol Integration](#10-x402-payment-protocol-integration)
11. [Authentication](#11-authentication)
12. [Fee Calculation](#12-fee-calculation)
13. [Monitoring and Metrics](#13-monitoring-and-metrics)
14. [Deployment (Docker, Railway)](#14-deployment-docker-railway)
15. [Adding a Custom Signer to Kora](#15-adding-a-custom-signer-to-kora)
16. [Security Best Practices](#16-security-best-practices)
17. [Troubleshooting](#17-troubleshooting)

---

## 1. Installation

### System Requirements

| Component     | Requirement                            |
| ------------- | -------------------------------------- |
| Rust (CLI)    | 1.86 or higher                         |
| Node.js (SDK) | LTS or higher                          |
| TypeScript    | latest                                 |
| Docker        | optional, for containerized deployment |
| Solana CLI    | optional, useful for key generation    |

### Kora CLI

**From Cargo (recommended for development):**

```bash
cargo install kora-cli
```

**From source:**

```bash
git clone https://github.com/solana-foundation/kora.git
cd kora
just install
```

**Docker:**

```bash
docker pull ghcr.io/solana-foundation/kora:latest

docker run \
  -v $(pwd)/kora.toml:/app/kora.toml \
  -v $(pwd)/signers.toml:/app/signers.toml \
  -p 8080:8080 \
  ghcr.io/solana-foundation/kora:latest \
  rpc start --signers-config /app/signers.toml
```

Verify:

```bash
kora --version
```

If `kora: command not found`, add Cargo's bin directory to PATH:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### TypeScript SDK

```bash
pnpm add @solana/kora

# Required peer dependencies
pnpm add @solana/kit @solana-program/token
```

**Version requirements:**

- `KoraClient` (standalone): `@solana/kit` v5.0+
- `koraPlugin()` (composable): `@solana/kit` v5.4+

Verify SDK connection:

```typescript
import { KoraClient } from "@solana/kora";

const client = new KoraClient("http://localhost:8080");
try {
  const config = await client.getConfig();
  console.log("Connected to Kora");
} catch (error) {
  console.error("Connection failed:", error.message);
}
```

---

## 2. Project Structure and Configuration Files

A minimal Kora deployment needs two configuration files:

```
my-kora-node/
├── kora.toml       # server behavior, validation rules, fee policies
├── signers.toml    # which signing keys/services to use
└── .env            # environment variables for private keys and API secrets
```

Every `kora rpc start` command requires both files:

```bash
kora --config kora.toml rpc start --signers-config signers.toml
```

The RPC endpoint and other flags can be set via CLI or environment variables:

| Environment Variable | CLI Flag           | Description                                            |
| -------------------- | ------------------ | ------------------------------------------------------ |
| `RPC_URL`            | `--rpc-url`        | Solana RPC endpoint (default: `http://127.0.0.1:8899`) |
| `RUST_LOG`           | `--logging-format` | Log level / format                                     |

---

## 3. kora.toml — Complete Reference

`kora.toml` is the control center for your Kora node. Every aspect of validation, security, pricing, caching, and monitoring is configured here.

### `[kora]` — Core Server Settings

```toml
[kora]
rate_limit = 100
payment_address = "YourPaymentAddressPubkey"   # optional
```

| Option            | Required | Description                                                                                                                         |
| ----------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `rate_limit`      | Yes      | Global rate limit in requests per second                                                                                            |
| `payment_address` | No       | Where token payments are sent. Defaults to the signer's address if not specified. Useful when you want a separate treasury address. |

### `[kora.auth]` — Authentication

Authentication is optional but strongly recommended for production. Without it, anyone who discovers your endpoint can submit transactions and consume your SOL.

```toml
[kora.auth]
api_key = "kora_live_sk_1234567890abcdef"
hmac_secret = "kora_hmac_your-strong-32-char-secret"
max_timestamp_age = 300
```

| Option              | Description                                                                   |
| ------------------- | ----------------------------------------------------------------------------- |
| `api_key`           | Shared secret sent in the `x-api-key` header                                  |
| `hmac_secret`       | HMAC-SHA256 secret (minimum 32 characters) for signature-based authentication |
| `max_timestamp_age` | Maximum age of an HMAC timestamp in seconds (default: 300)                    |

Both can be set simultaneously for maximum security. When both are configured, clients must send all three headers: `x-api-key`, `x-timestamp`, and `x-hmac-signature`.

Both values can also be set via environment variables (`KORA_API_KEY`, `KORA_HMAC_SECRET`), which take priority over the TOML file.

Generate a key:

```bash
openssl rand -hex 32
```

### `[kora.cache]` — Redis Caching (Optional)

Reduces redundant Solana RPC calls. Gracefully falls back to direct RPC if Redis is unavailable.

```toml
[kora.cache]
enabled = true
url = "redis://localhost:6379"
default_ttl = 300    # 5 minutes
account_ttl = 60     # 1 minute
```

### `[kora.usage_limit]` — Per-Wallet Transaction Limiting (Optional)

Prevents abuse and enables reward programs. Currently, limits are permanent once reached — they do not reset automatically.

```toml
[kora.usage_limit]
enabled = true
cache_url = "redis://localhost:6379"
max_transactions = 100      # 0 = unlimited
fallback_if_unavailable = true
```

When `fallback_if_unavailable = true`, transactions proceed even if Redis is down. Set to `false` if you need strict enforcement.

### `[kora.enabled_methods]` — RPC Method Access Control

If this section is present in your TOML, all methods must be explicitly set. If the section is absent, all methods are enabled.

```toml
[kora.enabled_methods]
liveness                  = true
estimate_transaction_fee  = true
get_supported_tokens      = true
sign_transaction          = true
sign_and_send_transaction = true
transfer_transaction      = true
get_blockhash             = true
get_config                = true
get_payer_signer          = true
# Bundle methods (Kora 2.2+)
sign_bundle               = true
sign_and_send_bundle      = true
estimate_bundle_fee       = true
```

Disabling methods you do not use reduces attack surface.

### `[validation]` — Transaction Validation Rules

This is the most critical section for security. Transactions that do not pass these rules are rejected before signing.

```toml
[validation]
max_allowed_lamports    = 1000000     # 0.001 SOL max per transaction
max_signatures          = 10
price_source            = "Jupiter"   # "Jupiter" or "Mock"
allow_durable_transactions = false    # default false — see security note below

allowed_programs = [
    "11111111111111111111111111111111",               # System Program
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",    # SPL Token
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",    # Token-2022
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",   # Associated Token
    "AddressLookupTab1e1111111111111111111111111",    # Address Lookup Table
    "ComputeBudget11111111111111111111111111111111",  # Compute Budget
    "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr",   # Memo
]

allowed_tokens = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",   # USDC mainnet
]

allowed_spl_paid_tokens = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",   # USDC mainnet
]

disallowed_accounts = [
    # "KnownBadActor111111111111111111111111111111",
]
```

**Option details:**

| Option                       | Description                                                                                                                                                    |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `max_allowed_lamports`       | Maximum SOL fee the operator will absorb per transaction. Limits exposure.                                                                                     |
| `max_signatures`             | Solana fees scale with signature count. Cap this to prevent high-fee transactions.                                                                             |
| `price_source`               | `"Jupiter"` for production (requires `JUPITER_API_KEY` env var). `"Mock"` for local testing.                                                                   |
| `allow_durable_transactions` | Default `false`. Durable nonce transactions can be held and submitted later, creating economic attacks. Only enable if your use case specifically requires it. |
| `allowed_programs`           | Whitelist of program IDs transactions can interact with. Must include at minimum the programs your users actually call.                                        |
| `allowed_tokens`             | Token mints that can appear in transactions.                                                                                                                   |
| `allowed_spl_paid_tokens`    | Subset of `allowed_tokens` — tokens actually accepted as fee payment.                                                                                          |
| `disallowed_accounts`        | Explicit blocklist of accounts.                                                                                                                                |

At least one program, one token, and one paid token must be configured or no transactions can be processed.

To use Jupiter pricing:

```bash
JUPITER_API_KEY=your_api_key kora rpc start --signers-config signers.toml
```

### `[validation.token2022]` — Token Extensions Blocking

All Token-2022 extensions are allowed by default. Block risky ones explicitly.

```toml
[validation.token2022]
blocked_mint_extensions = [
    "transfer_hook",       # custom transfer logic — unpredictable behavior
    "pausable",            # operator can freeze all transfers
    "permanent_delegate",  # delegate can seize tokens at any time
]

blocked_account_extensions = [
    "cpi_guard",           # restricts composability
    "memo_transfer",       # requires memo on every incoming transfer
]
```

**Available mint extensions:**

| Name                         | Description                                              |
| ---------------------------- | -------------------------------------------------------- |
| `confidential_transfer_mint` | Encrypted transfer configuration                         |
| `confidential_mint_burn`     | Confidential mint/burn                                   |
| `transfer_fee_config`        | Per-transfer fee                                         |
| `mint_close_authority`       | Authority to close the mint                              |
| `interest_bearing_config`    | Accruing interest                                        |
| `non_transferable`           | Soulbound tokens                                         |
| `permanent_delegate`         | Irrevocable authority over all token transfers and burns |
| `transfer_hook`              | Custom program called on every transfer                  |
| `pausable`                   | Can freeze all transfers                                 |

**Available account extensions:**

| Name                            | Description                        |
| ------------------------------- | ---------------------------------- |
| `confidential_transfer_account` | Confidential transfer state        |
| `non_transferable_account`      | Non-transferable state             |
| `transfer_hook_account`         | Transfer hook state                |
| `pausable_account`              | Pausable state                     |
| `memo_transfer`                 | Requires memo on inbound transfers |
| `cpi_guard`                     | Prevents certain CPI calls         |
| `immutable_owner`               | Owner cannot be changed            |
| `default_account_state`         | Default state for new accounts     |

**`permanent_delegate` is particularly dangerous for payment tokens.** A token with this extension allows the delegate to seize or burn tokens at any time. Funds paid to your Kora node could be taken back after the fact. Block it or avoid using payment tokens that have it.

### `[validation.fee_payer_policy]` — Fee Payer Role Restrictions

This prevents users from crafting transactions that use your Kora node's signer key for operations beyond fee paying. All values default to `false`. Start with everything disabled and enable only what your specific use case requires.

```toml
[validation.fee_payer_policy.system]
allow_transfer         = false    # block SOL transfers from fee payer
allow_assign           = false
allow_create_account   = false
allow_allocate         = false

[validation.fee_payer_policy.system.nonce]
allow_initialize       = false
allow_advance          = false
allow_authorize        = false
allow_withdraw         = false

[validation.fee_payer_policy.spl_token]
allow_transfer         = false    # CRITICAL: block SPL transfers from fee payer
allow_burn             = false
allow_close_account    = false
allow_approve          = false
allow_revoke           = false
allow_set_authority    = false
allow_mint_to          = false
allow_initialize_mint  = false
allow_initialize_account = false
allow_initialize_multisig = false
allow_freeze_account   = false
allow_thaw_account     = false

[validation.fee_payer_policy.token_2022]
# identical option set as spl_token
allow_transfer         = false
# ... same fields
```

**What each permission unlocks — and why it is dangerous:**

| Permission                      | Risk if enabled                                         |
| ------------------------------- | ------------------------------------------------------- |
| `system.allow_transfer`         | Users can drain SOL from the fee payer wallet           |
| `spl_token.allow_transfer`      | Users can drain token balances from the fee payer       |
| `spl_token.allow_mint_to`       | Users can mint tokens if fee payer holds mint authority |
| `spl_token.allow_set_authority` | Users can take over accounts controlled by fee payer    |
| `spl_token.allow_close_account` | Users receive rent from accounts owned by fee payer     |
| `spl_token.allow_burn`          | Users can burn tokens held by fee payer                 |

**Exception:** When using Jito bundles and Kora pays the Jito tip, you must enable:

```toml
[validation.fee_payer_policy.system]
allow_transfer = true  # required for Kora to send the tip transfer
```

### `[validation.price]` — Fee Pricing Model

Three models are available:

**Margin pricing (default):**

```toml
[validation.price]
type = "margin"
margin = 0.15    # 0.15 = 15% markup on actual network fees
```

**Fixed pricing:**

```toml
[validation.price]
type = "fixed"
amount = 1000000    # in token's smallest unit (e.g. 1 USDC = 1,000,000)
token = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
```

**Free (fully sponsored):**

```toml
[validation.price]
type = "free"
```

**Critical security note for `fixed` and `free`:** These models do NOT include fee payer outflow in the charged amount. If `allow_transfer` is enabled alongside fixed/free pricing, users can make the fee payer send arbitrary SOL or tokens at a fixed cost to them, draining the wallet. Always combine fixed/free pricing with fully restrictive fee payer policies and authentication.

The config validator warns you about dangerous combinations:

```bash
kora --config kora.toml config validate
```

### `[metrics]` — Prometheus Monitoring (Optional)

```toml
[metrics]
enabled         = true
endpoint        = "/metrics"
port            = 8080
scrape_interval = 60

[metrics.fee_payer_balance]
enabled         = true
expiry_seconds  = 30
```

Metrics are served at `http://localhost:{port}/{endpoint}`.

### `[kora.bundle]` — Jito Bundle Support (v2.2+)

```toml
[kora.bundle]
enabled = true

[kora.bundle.jito]
block_engine_url = "https://mainnet.block-engine.jito.wtf"
```

### Complete Production-Ready Example

```toml
[kora]
rate_limit = 100

[kora.auth]
hmac_secret = "kora_hmac_minimum_32_character_secret_here"
max_timestamp_age = 300

[kora.cache]
enabled = true
url = "redis://localhost:6379"
default_ttl = 300
account_ttl = 60

[kora.usage_limit]
enabled = true
cache_url = "redis://localhost:6379"
max_transactions = 100
fallback_if_unavailable = true

[kora.enabled_methods]
liveness                  = true
estimate_transaction_fee  = true
get_supported_tokens      = true
sign_transaction          = true
sign_and_send_transaction = true
transfer_transaction      = true
get_blockhash             = true
get_config                = true
get_payer_signer          = true

[validation]
price_source            = "Jupiter"
max_allowed_lamports    = 1000000
max_signatures          = 10
allow_durable_transactions = false

allowed_programs = [
    "11111111111111111111111111111111",
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
    "AddressLookupTab1e1111111111111111111111111",
    "ComputeBudget11111111111111111111111111111111",
]

allowed_tokens = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  # USDC
]

allowed_spl_paid_tokens = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  # USDC
]

disallowed_accounts = []

[validation.fee_payer_policy.system]
allow_transfer         = false
allow_assign           = false
allow_create_account   = false
allow_allocate         = false

[validation.fee_payer_policy.system.nonce]
allow_initialize       = false
allow_advance          = false
allow_authorize        = false
allow_withdraw         = false

[validation.fee_payer_policy.spl_token]
allow_transfer         = false
allow_burn             = false
allow_close_account    = false
allow_approve          = false
allow_revoke           = false
allow_set_authority    = false
allow_mint_to          = false
allow_initialize_mint  = false
allow_initialize_account = false
allow_initialize_multisig = false
allow_freeze_account   = false
allow_thaw_account     = false

[validation.fee_payer_policy.token_2022]
allow_transfer         = false
allow_burn             = false
allow_close_account    = false
allow_approve          = false
allow_revoke           = false
allow_set_authority    = false
allow_mint_to          = false
allow_initialize_mint  = false
allow_initialize_account = false
allow_initialize_multisig = false
allow_freeze_account   = false
allow_thaw_account     = false

[validation.token2022]
blocked_mint_extensions = [
    "transfer_hook",
    "pausable",
    "permanent_delegate",
]
blocked_account_extensions = [
    "cpi_guard",
    "memo_transfer",
]

[validation.price]
type = "margin"
margin = 0.15

[metrics]
enabled         = true
endpoint        = "/metrics"
port            = 8080
scrape_interval = 60

[metrics.fee_payer_balance]
enabled        = true
expiry_seconds = 30
```

---

## 4. signers.toml — Complete Reference

The `signers.toml` file configures which signing keys Kora uses and how it selects among them.

### `[signer_pool]`

```toml
[signer_pool]
strategy = "round_robin"    # round_robin | random | weighted
```

For production, configure multiple signers for improved reliability and to distribute account lock pressure across transactions.

### `[[signers]]` — Per-Signer Configuration

At least one signer is required. Each signer has a `name` (unique within the pool), a `type`, and type-specific configuration pointing to environment variables.

#### Private Key (Memory) Signer

The simplest approach. Accepts three key formats:

```toml
[[signers]]
name = "main_signer"
type = "memory"
private_key_env = "KORA_PRIVATE_KEY"
```

Supported key formats in the environment variable:

| Format          | Example value                 |
| --------------- | ----------------------------- |
| Base58 string   | `5KKsLVU6TcbVDK4BS6K1DG...`   |
| U8 array string | `[174,47,154,16,202,193,...]` |
| JSON file path  | `/path/to/keypair.json`       |

Generate a keypair:

```bash
solana-keygen new --outfile ~/.config/solana/kora-keypair.json
solana-keygen pubkey ~/.config/solana/kora-keypair.json
```

#### Turnkey Signer

Enterprise HSM with policy controls:

```toml
[[signers]]
name = "turnkey_signer"
type = "turnkey"
api_public_key_env  = "TURNKEY_API_PUBLIC_KEY"
api_private_key_env = "TURNKEY_API_PRIVATE_KEY"
organization_id_env = "TURNKEY_ORG_ID"
private_key_id_env  = "TURNKEY_PRIVATE_KEY_ID"
public_key_env      = "TURNKEY_PUBLIC_KEY"
```

Required env vars:

```bash
TURNKEY_API_PUBLIC_KEY="your_turnkey_api_public_key"
TURNKEY_API_PRIVATE_KEY="your_turnkey_api_private_key"
TURNKEY_ORG_ID="your_organization_id"
TURNKEY_PRIVATE_KEY_ID="your_private_key_id"   # from Turnkey wallet details
TURNKEY_PUBLIC_KEY="your_solana_address"        # the wallet's Solana address
```

Setup (Turnkey dashboard):

1. Copy your organization ID from the user menu
2. Create an API key under Account Settings → API Keys → Generate in-browser
3. Create a private key under Wallets → Create Private Key → ED25519 → Solana asset type
4. Copy the Private Key ID and Address from wallet details

#### Privy Signer

Embedded wallet infrastructure:

```toml
[[signers]]
name = "privy_signer"
type = "privy"
app_id_env     = "PRIVY_APP_ID"
app_secret_env = "PRIVY_APP_SECRET"
wallet_id_env  = "PRIVY_WALLET_ID"
```

Required env vars:

```bash
PRIVY_APP_ID="your_privy_app_id"
PRIVY_APP_SECRET="your_privy_app_secret"
PRIVY_WALLET_ID="your_wallet_id"
```

Setup (Privy dashboard):

1. Select your application → Retrieve API Keys → New Secret → copy App ID and Secret
2. Wallets → Wallet Infrastructure → New Wallet → Solana → copy Wallet ID
3. Fund the wallet address with SOL

#### Multi-Signer Round-Robin Example

```toml
[signer_pool]
strategy = "round_robin"

[[signers]]
name = "signer_1"
type = "memory"
private_key_env = "SIGNER_1_PRIVATE_KEY"

[[signers]]
name = "signer_2"
type = "memory"
private_key_env = "SIGNER_2_PRIVATE_KEY"

[[signers]]
name = "signer_3_turnkey"
type = "turnkey"
api_public_key_env  = "TURNKEY_API_PUBLIC_KEY"
api_private_key_env = "TURNKEY_API_PRIVATE_KEY"
organization_id_env = "TURNKEY_ORG_ID"
private_key_id_env  = "TURNKEY_PRIVATE_KEY_ID"
public_key_env      = "TURNKEY_PUBLIC_KEY"
```

#### Weighted Strategy

```toml
[signer_pool]
strategy = "weighted"

[[signers]]
name = "primary"
type = "memory"
private_key_env = "PRIMARY_KEY"
weight = 3        # selected 3x as often as backup

[[signers]]
name = "backup"
type = "memory"
private_key_env = "BACKUP_KEY"
weight = 1
```

### Signer Consistency Across Calls

When making multiple related calls (estimate → sign), use the same signer for consistency. Fetch the signer address first and pass it to each call:

```typescript
const { signer_address } = await client.getPayerSigner();

const estimate = await client.estimateTransactionFee({
  transaction: tx,
  signer_key: signer_address,
});

const signed = await client.signTransaction({
  transaction: tx,
  signer_key: signer_address, // must match
});
```

---

## 5. CLI Reference

### Global Flags (apply to all commands)

```bash
kora [GLOBAL FLAGS] <COMMAND>
```

| Flag        | Default                 | Description         |
| ----------- | ----------------------- | ------------------- |
| `--config`  | `kora.toml`             | Path to kora.toml   |
| `--rpc-url` | `http://127.0.0.1:8899` | Solana RPC endpoint |
| `--version` | —                       | Print version       |
| `--help`    | —                       | Print help          |

### `kora rpc start` — Start the Server

```bash
kora --config path/to/kora.toml rpc start --signers-config path/to/signers.toml
```

| Flag               | Default  | Description                                    |
| ------------------ | -------- | ---------------------------------------------- |
| `--signers-config` | Required | Path to signers.toml                           |
| `--no-load-signer` | false    | Start without signers (read-only methods only) |
| `-p` / `--port`    | 8080     | HTTP port                                      |
| `--logging-format` | standard | `standard` or `json`                           |

With Jupiter pricing:

```bash
JUPITER_API_KEY=your_key kora --config kora.toml rpc start --signers-config signers.toml
```

### `kora config validate` — Validate Configuration

```bash
# Quick validation (offline, no RPC calls)
kora --config kora.toml config validate

# Thorough validation including on-chain account checks
kora --config kora.toml --rpc-url https://api.mainnet-beta.solana.com config validate-with-rpc
```

`validate-with-rpc` checks:

- All allowed programs exist and are executable on-chain
- All allowed token mints exist as valid accounts
- Payment address has ATAs initialized for all allowed tokens
- Account types match expectations (program vs mint)

### `kora rpc initialize-atas` — Initialize Payment ATAs

Run this before starting your node to ensure the payment address has Associated Token Accounts for each token in `allowed_spl_paid_tokens`. Kora needs these to receive token payments.

```bash
kora rpc initialize-atas --signers-config signers.toml

# With options
kora rpc initialize-atas \
    --signers-config signers.toml \
    --fee-payer-key "7xKXtg2..." \
    --compute-unit-price 1000 \
    --chunk-size 10
```

| Flag                   | Description                                                    |
| ---------------------- | -------------------------------------------------------------- |
| `--fee-payer-key`      | Specific signer to pay ATA creation (defaults to first signer) |
| `--compute-unit-price` | Priority fee in micro-lamports                                 |
| `--compute-unit-limit` | Compute unit limit                                             |
| `--chunk-size`         | ATAs to create per transaction                                 |

### Common Usage Patterns

```bash
# Local development (no auth, mock pricing)
kora --config kora.toml rpc start --signers-config signers.toml

# Production devnet
kora --config kora.toml \
     --rpc-url https://api.devnet.solana.com \
     rpc start \
     --signers-config signers.toml \
     --logging-format json

# Test config only (no signer needed)
kora --config kora.toml rpc start --no-load-signer

# Custom port
kora --config kora.toml rpc start --signers-config signers.toml --port 3000
```

---

## 6. JSON-RPC API — All Methods

All requests follow JSON-RPC 2.0:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "methodName",
  "params": {}
}
```

Successful response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {}
}
```

Error response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32600,
    "message": "Invalid request"
  }
}
```

---

### `getConfig`

Returns the node's current configuration. Call this to discover what the node supports before building transactions.

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getConfig","params":[]}'
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "enabled_methods": {},
    "fee_payers": ["3Z1Ef7YaxK8oUMoi6exf7wYZjZKWJJsrzJXSt1c3qrDE"],
    "validation_config": {
      "max_allowed_lamports": 1000000,
      "allowed_programs": ["..."],
      "allowed_spl_paid_tokens": ["..."]
    }
  }
}
```

TypeScript:

```typescript
const config = await client.getConfig();
console.log("Fee payer:", config.fee_payer);
console.log("Paid tokens:", config.validation_config.allowed_spl_paid_tokens);
```

---

### `getPayerSigner`

Returns the active signer's address and where payment tokens should be sent. Always call this first and use the returned `signer_address` in subsequent calls to ensure consistency within a transaction flow.

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getPayerSigner","params":[]}'
```

Response:

```json
{
  "result": {
    "payment_address": "3Z1Ef7YaxK8oUMoi6exf7wYZjZKWJJsrzJXSt1c3qrDE",
    "signer_address": "3Z1Ef7YaxK8oUMoi6exf7wYZjZKWJJsrzJXSt1c3qrDE"
  }
}
```

`payment_address` and `signer_address` differ when `payment_address` is configured separately in `kora.toml`.

---

### `getSupportedTokens`

Returns the list of tokens accepted as fee payment.

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getSupportedTokens","params":[]}'
```

Response:

```json
{
  "result": {
    "tokens": ["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"]
  }
}
```

TypeScript:

```typescript
const { tokens } = await client.getSupportedTokens();
```

---

### `getBlockhash`

Returns the latest blockhash from the Solana RPC the node is connected to. Use this when building estimate transactions to ensure a valid blockhash for simulation.

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getBlockhash","params":[]}'
```

Response:

```json
{
  "result": {
    "blockhash": "C8W8d5w2H4jKXyFg5CEBoiaPvEpJ1am7xLxZ3fym4a2g"
  }
}
```

---

### `estimateTransactionFee`

Estimates the fee for a transaction in both lamports and the specified payment token. Build a transaction with your intended instructions, encode it as base64, and pass it here before adding the payment instruction.

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0","id":1,
    "method":"estimateTransactionFee",
    "params":{
        "transaction": "base64EncodedTransaction",
        "fee_token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    }
  }'
```

Response:

```json
{
  "result": {
    "fee_in_lamports": 5000,
    "fee_in_token": 1000000,
    "payment_address": "3Z1Ef7Y...",
    "signer_pubkey": "3Z1Ef7Y..."
  }
}
```

TypeScript:

```typescript
const fees = await client.estimateTransactionFee({
  transaction: "base64EncodedTransaction",
  fee_token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
});
console.log("Fee in USDC base units:", fees.fee_in_token);
```

---

### `getPaymentInstruction`

**Client-side only.** Does not make a network call to the Kora server. The TypeScript SDK calculates the payment instruction locally using fee estimation data.

Call this after getting an estimate. Returns an instruction you append to your final transaction before signing.

```typescript
const paymentInfo = await client.getPaymentInstruction({
  transaction: "base64EncodedTransaction",
  fee_token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  source_wallet: "sourceWalletPublicKey",
});

// Append paymentInfo.payment_instruction to your transaction
```

Response structure:

```json
{
  "original_transaction": "base64EncodedTransaction",
  "payment_address": "3Z1Ef7Y...",
  "payment_amount": 1000000,
  "payment_instruction": {},
  "payment_token": "EPjFWdd...",
  "signer_address": "3Z1Ef7Y..."
}
```

---

### `transferTransaction`

Creates a token or SOL transfer transaction with Kora as the fee payer. Returns a transaction ready for the user to sign and submit. Use the System Program address (`11111111111111111111111111111111`) as the `token` field for SOL transfers.

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0","id":1,
    "method":"transferTransaction",
    "params":{
        "amount": 1000000,
        "token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "source": "sourcePublicKey",
        "destination": "destinationPublicKey"
    }
  }'
```

Response:

```json
{
  "result": {
    "blockhash": "...",
    "instructions": "...",
    "message": "...",
    "signer_pubkey": "3Z1Ef7Y...",
    "transaction": "base64EncodedTransaction"
  }
}
```

TypeScript:

```typescript
const transfer = await client.transferTransaction({
  amount: 1000000, // 1 USDC (6 decimals)
  token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  source: "sourceWalletPublicKey",
  destination: "destinationWalletPublicKey",
});
```

---

### `signTransaction`

Validates and signs a transaction without broadcasting it. Kora checks that the transaction includes a valid payment instruction before signing. Returns the signed transaction for you to broadcast yourself.

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0","id":1,
    "method":"signTransaction",
    "params":{"transaction":"base64EncodedTransaction"}
  }'
```

Response:

```json
{
  "result": {
    "signature": "base58Signature",
    "signed_transaction": "base64EncodedTransaction",
    "signer_pubkey": "3Z1Ef7Y..."
  }
}
```

TypeScript:

```typescript
const result = await client.signTransaction({
  transaction: "base64EncodedTransaction",
  signer_key: signer_address, // optional — pin to specific signer
});
```

---

### `signAndSendTransaction`

Validates, signs, and broadcasts the transaction in one call. Returns the transaction signature.

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0","id":1,
    "method":"signAndSendTransaction",
    "params":{"transaction":"base64EncodedTransaction"}
  }'
```

Response:

```json
{
  "result": {
    "signature": "base58Signature",
    "signed_transaction": "base64EncodedTransaction",
    "signer_pubkey": "3Z1Ef7Y..."
  }
}
```

TypeScript:

```typescript
const result = await client.signAndSendTransaction({
  transaction: "base64EncodedTransaction",
});
console.log("Signature:", result.signature);
```

---

## 7. TypeScript SDK

Two client options are available.

### KoraClient (Standalone)

Works with `@solana/kit` v5.0+.

```typescript
import { KoraClient } from "@solana/kora";

const kora = new KoraClient("https://your-kora-server.com");
const kora = new KoraClient({
  rpcUrl: "https://your-kora-server.com",
  apiKey: process.env.KORA_API_KEY, // if API key auth is enabled
  hmacSecret: process.env.KORA_HMAC_SECRET, // if HMAC auth is enabled
});
```

### koraPlugin (Kit Composable)

Requires `@solana/kit` v5.4+. Composes Kora methods into an existing Kit client.

```typescript
import { createEmptyClient } from "@solana/kit";
import { koraPlugin } from "@solana/kora";

const client = createEmptyClient().use(
  koraPlugin({ endpoint: "https://your-kora-server.com" }),
);

const config = await client.kora.getConfig();
```

The plugin returns Kit-typed responses (`Address`, `Blockhash`, `Base64EncodedWireTransaction`). The `KoraApi` type is exported for composition with other plugins.

---

## 8. Complete Gasless Transaction Flow (TypeScript)

This is the standard pattern for building a full gasless transaction where the user pays fees in a token and Kora handles gas.

### Flow Overview

```
1. Initialize clients and fetch signer address
2. Build estimate transaction (user instructions + noop signer)
3. Call getPaymentInstruction → receive payment instruction
4. Build final transaction (user instructions + payment instruction)
5. User signs with their keypair
6. Submit to Kora: signTransaction or signAndSendTransaction
7. Kora validates payment and co-signs
8. Broadcast to Solana (if using signTransaction)
```

### Complete Implementation

```typescript
import { KoraClient } from "@solana/kora";
import {
  createKeyPairSignerFromBytes,
  getBase58Encoder,
  createNoopSigner,
  address,
  getBase64EncodedWireTransaction,
  partiallySignTransactionMessageWithSigners,
  partiallySignTransaction,
  Blockhash,
  Base64EncodedWireTransaction,
  Instruction,
  KeyPairSigner,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  MicroLamports,
} from "@solana/kit";
import {
  updateOrAppendSetComputeUnitLimitInstruction,
  updateOrAppendSetComputeUnitPriceInstruction,
} from "@solana-program/compute-budget";
import { createRecentSignatureConfirmationPromiseFactory } from "@solana/transaction-confirmation";

const CONFIG = {
  computeUnitLimit: 200_000,
  computeUnitPrice: 1_000_000n as MicroLamports,
  solanaRpcUrl: "http://127.0.0.1:8899",
  solanaWsUrl: "ws://127.0.0.1:8900",
  koraRpcUrl: "http://localhost:8080/",
};

// Step 1: Clients
const client = new KoraClient({ rpcUrl: CONFIG.koraRpcUrl });
const rpc = createSolanaRpc(CONFIG.solanaRpcUrl);
const rpcSub = createSolanaRpcSubscriptions(CONFIG.solanaWsUrl);
const confirmTransaction = createRecentSignatureConfirmationPromiseFactory({
  rpc,
  rpcSubscriptions: rpcSub,
});

// Step 2: Fetch signer — use this address consistently throughout
const { signer_address } = await client.getPayerSigner();
const noopSigner = createNoopSigner(address(signer_address));

// Step 3: Get blockhash for estimate
const { blockhash } = await client.getBlockhash();

// Build estimate transaction with your instructions and a noop signer
// The noop signer stands in for Kora's address before Kora actually signs
const estimateTransaction = pipe(
  createTransactionMessage({ version: 0 }),
  (tx) => setTransactionMessageFeePayerSigner(noopSigner, tx),
  (tx) =>
    setTransactionMessageLifetimeUsingBlockhash(
      {
        blockhash: blockhash as Blockhash,
        lastValidBlockHeight: 0n,
      },
      tx,
    ),
  (tx) => appendTransactionMessageInstructions(userInstructions, tx),
  (tx) =>
    updateOrAppendSetComputeUnitPriceInstruction(CONFIG.computeUnitPrice, tx),
  (tx) =>
    updateOrAppendSetComputeUnitLimitInstruction(CONFIG.computeUnitLimit, tx),
);

const signedEstimate =
  await partiallySignTransactionMessageWithSigners(estimateTransaction);
const base64Estimate = getBase64EncodedWireTransaction(signedEstimate);

// Step 4: Get payment instruction from Kora
const paymentInfo = await client.getPaymentInstruction({
  transaction: base64Estimate,
  fee_token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  source_wallet: senderKeypair.address,
});

// Step 5: Build final transaction with payment instruction
const { blockhash: freshBlockhash } = await client.getBlockhash();

const finalTransaction = pipe(
  createTransactionMessage({ version: 0 }),
  (tx) => setTransactionMessageFeePayerSigner(noopSigner, tx),
  (tx) =>
    setTransactionMessageLifetimeUsingBlockhash(
      {
        blockhash: freshBlockhash as Blockhash,
        lastValidBlockHeight: 0n,
      },
      tx,
    ),
  (tx) =>
    appendTransactionMessageInstructions(
      [...userInstructions, paymentInfo.payment_instruction],
      tx,
    ),
  (tx) =>
    updateOrAppendSetComputeUnitPriceInstruction(CONFIG.computeUnitPrice, tx),
  (tx) =>
    updateOrAppendSetComputeUnitLimitInstruction(CONFIG.computeUnitLimit, tx),
);

// Step 6: User signs
const partialSigned =
  await partiallySignTransactionMessageWithSigners(finalTransaction);
const userSigned = await partiallySignTransaction(
  [senderKeypair.keyPair],
  partialSigned,
);
const base64Final = getBase64EncodedWireTransaction(userSigned);

// Step 7: Kora validates payment and co-signs
const { signed_transaction } = await client.signTransaction({
  transaction: base64Final,
  signer_key: signer_address,
});

// Step 8: Broadcast and confirm
const signature = await rpc
  .sendTransaction(signed_transaction as Base64EncodedWireTransaction, {
    encoding: "base64",
  })
  .send();

await confirmTransaction({
  commitment: "confirmed",
  signature,
  abortSignal: new AbortController().signal,
});

console.log("Transaction confirmed:", signature);
```

### Key Concepts

**Noop Signer:** A placeholder signer that holds Kora's address in the transaction before Kora actually provides its signature. Required by `@solana/kit`'s transaction builder, which needs a fee payer set before you can serialize.

**Fresh Blockhash for Final Transaction:** Always fetch a new blockhash for the final transaction. The estimate transaction's blockhash may have aged by the time you assemble the final transaction.

**Signer Consistency:** Use the same `signer_address` for the noop signer, `getPaymentInstruction`, and `signTransaction`. Mixing addresses causes validation failures.

**Two-Transaction Pattern:** The estimate transaction and the final transaction are separate objects. The estimate is used to calculate fees — it is never broadcast. The final transaction adds the payment instruction and is what gets signed and sent.

---

## 9. Jito Bundle Support (v2.2+)

Jito bundles allow up to 5 transactions to execute atomically in sequence. Kora can sign and submit entire bundles. Install the release candidate:

```bash
cargo install kora-cli@2.2.0-beta.2
```

### kora.toml bundle configuration

```toml
[kora.enabled_methods]
sign_bundle               = true
sign_and_send_bundle      = true
estimate_bundle_fee       = true

[validation.fee_payer_policy.system]
allow_transfer = true   # required: Kora must send the Jito tip

[validation.price]
type = "free"           # Kora absorbs all fees including tip

[kora.bundle]
enabled = true

[kora.bundle.jito]
block_engine_url = "https://mainnet.block-engine.jito.wtf"
```

### TypeScript Bundle Flow

```typescript
// Jito tip accounts — select one at random
const JITO_TIP_ACCOUNTS = [
  "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
  "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
  "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
  // ... 5 more
];

const MINIMUM_JITO_TIP = 1_000n; // lamports

// Build each transaction with a noop signer for Kora's address
// Add the Jito tip transfer to the LAST transaction only
// Use the same blockhash across all transactions in the bundle

// Submit the bundle
const { bundle_uuid } = await client.signAndSendBundle({
  transactions: [base64Tx1, base64Tx2, base64Tx3, base64Tx4],
  signer_key: signer_address,
});

console.log("Bundle UUID:", bundle_uuid);
```

**How the tip works:** The tip transfer uses the noop signer as `source` and a random Jito tip account as `destination`. Since Kora's address is the source (via noopSigner), Kora's actual signature authorizes this transfer when it co-signs the bundle. The `allow_transfer = true` policy is what permits this.

**Important:** Tips on mainnet are non-refundable. Small tips (1,000 lamports) may not land in competitive conditions.

---

## 10. x402 Payment Protocol Integration

x402 is an HTTP payment standard where APIs return HTTP 402 when payment is required. Kora can act as the facilitator in an x402 flow, handling transaction verification and settlement on behalf of API servers.

### Component Roles

| Component         | Port | Role                                                               |
| ----------------- | ---- | ------------------------------------------------------------------ |
| Kora RPC          | 8080 | Gasless signing, fee payment                                       |
| Facilitator proxy | 3000 | Bridges x402 ↔ Kora, implements `/verify`, `/settle`, `/supported` |
| Protected API     | 4021 | Your monetized API using `x402-express` middleware                 |
| Client            | —    | Uses x402 fetch wrapper to auto-pay on 402 responses               |

### Facilitator kora.toml for x402

```toml
[kora.auth]
api_key = "kora_facilitator_api_key_example"

[validation]
price_source = "Mock"    # or Jupiter for production
allowed_programs = [
    "11111111111111111111111111111111",
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
    "ComputeBudget111111111111111111111111111111",
]

# All fee payer permissions set to false (payment-only use case)
[validation.fee_payer_policy.system]
allow_transfer = false
# ...

[validation.price]
type = "free"    # Kora absorbs gas; x402 payment handles revenue
```

### Payment Flow

```
Client GET /protected
    → API returns 402 with payment requirements

x402 fetch wrapper detects 402
    → builds Solana transaction with payment instruction
    → POSTs to Facilitator /verify

Facilitator /verify
    → calls Kora signTransaction (validates without broadcasting)
    → returns { isValid: true }

Facilitator /settle
    → calls Kora signAndSendTransaction (signs + broadcasts)
    → returns { transaction: signature, success: true }

Client retries request with X-PAYMENT-RESPONSE header
    → API validates receipt
    → returns protected content
```

### Client Usage

```typescript
import { wrapFetchWithPayment } from "x402";

const payer = await createSigner(network, privateKey);
const fetchWithPayment = wrapFetchWithPayment(fetch, payer);

// This automatically handles 402 responses, creates payments,
// and retries with the payment proof
const response = await fetchWithPayment("http://localhost:4021/protected");
const data = await response.json();
```

---

## 11. Authentication

### API Key Authentication

Server configuration (kora.toml or env var `KORA_API_KEY`):

```toml
[kora.auth]
api_key = "kora_live_sk_1234567890abcdef"
```

Client — include in every request:

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -H "x-api-key: kora_live_sk_1234567890abcdef" \
  -d '{"jsonrpc":"2.0","method":"getConfig","id":1}'
```

TypeScript SDK:

```typescript
const kora = new KoraClient({
  rpcUrl: "http://localhost:8080",
  apiKey: process.env.KORA_API_KEY,
});
```

### HMAC Authentication

More secure. Each request signature expires after 5 minutes. Cannot be replayed.

Server configuration (kora.toml or env var `KORA_HMAC_SECRET`):

```toml
[kora.auth]
hmac_secret = "kora_hmac_minimum_32_character_secret_here"
max_timestamp_age = 300
```

TypeScript SDK (handles HMAC automatically):

```typescript
const kora = new KoraClient({
  rpcUrl: "http://localhost:8080",
  hmacSecret: process.env.KORA_HMAC_SECRET,
});
```

Manual implementation (other clients):

```javascript
const timestamp = Math.floor(Date.now() / 1000).toString();
const body = JSON.stringify({
  jsonrpc: "2.0",
  method: "getConfig",
  params: [],
  id: 1,
});

const signature = crypto
  .createHmac("sha256", process.env.KORA_HMAC_SECRET)
  .update(timestamp + body) // message = {timestamp}{body}
  .digest("hex");

// Headers: x-timestamp, x-hmac-signature
```

### Exempt Endpoints

The `/liveness` health check endpoint always bypasses authentication:

```bash
curl http://localhost:8080/liveness   # works regardless of auth config
```

### Security Comparison

| Method  | Security | Use Case                           |
| ------- | -------- | ---------------------------------- |
| None    | None     | Development only                   |
| API Key | Basic    | Internal services, trusted clients |
| HMAC    | High     | Public APIs, any untrusted network |
| Both    | Maximum  | High-security production           |

---

## 12. Fee Calculation

Kora's fee estimation (`estimateTransactionFee`, `signTransaction`) uses this formula for margin pricing:

```
Total Fee = Base Fee
          + Account Creation Fee
          + Kora Signature Fee
          + Fee Payer Outflow
          + Payment Instruction Fee
          + Transfer Fee Amount (Token-2022 only)
          + Margin Adjustment
```

| Component               | Calculation                                                               | Applied When                                      |
| ----------------------- | ------------------------------------------------------------------------- | ------------------------------------------------- |
| Base Fee                | `RpcClient.get_fee_for_message()` — actual Solana network fee             | Always                                            |
| Account Creation Fee    | Rent-exempt minimum × number of new ATAs (165–355 bytes)                  | When new ATAs are created                         |
| Kora Signature Fee      | Fixed 5,000 lamports                                                      | When Kora is not already a transaction signer     |
| Fee Payer Outflow       | Sum of SOL transfers, account creations, nonce withdrawals from fee payer | When fee payer performs System Program operations |
| Payment Instruction Fee | Fixed 50 lamports estimate                                                | When payment is required but not yet included     |
| Transfer Fee            | Token-2022 configured fee on the mint                                     | Token-2022 transfers to Kora payment address      |
| Margin Adjustment       | Configured margin × total fee                                             | When `[validation.price]` uses margin model       |

### Pricing Model Comparison

| Model    | Formula                               | Includes Outflow | Best For                                       |
| -------- | ------------------------------------- | ---------------- | ---------------------------------------------- |
| `margin` | (base + outflow + ...) × (1 + margin) | Yes              | Production — reflects real costs               |
| `fixed`  | Fixed token amount                    | No               | Predictable pricing — requires strict policies |
| `free`   | 0                                     | No               | Sponsored use cases — requires strict policies |

---

## 13. Monitoring and Metrics

### Configuration

```toml
[metrics]
enabled         = true
endpoint        = "/metrics"
port            = 8080
scrape_interval = 60

[metrics.fee_payer_balance]
enabled        = true
expiry_seconds = 30
```

### Available Metrics

```
# Request counts by method and HTTP status
kora_http_requests_total{method="signTransaction",status="200"} 42
kora_http_requests_total{method="signTransaction",status="400"} 3

# Request duration in seconds
kora_http_request_duration_seconds{method="signTransaction"} 0.045

# Signer SOL balances (multi-signer: one entry per signer)
signer_balance_lamports{signer_name="primary",signer_pubkey="4gBe..."} 500000000
signer_balance_lamports{signer_name="backup",signer_pubkey="7XyZ..."} 300000000
```

### Access Metrics

```bash
curl http://localhost:8080/metrics

# Filter to specific method
curl http://localhost:8080/metrics | grep signTransaction
```

### Useful Prometheus Queries

```
# Request rate by method
rate(kora_http_requests_total[1m])

# 95th percentile response time
histogram_quantile(0.95, kora_http_request_duration_seconds_bucket)

# Error rate
rate(kora_http_requests_total{status!="200"}[5m])

# Low balance alert (< 0.05 SOL on any signer)
min(signer_balance_lamports) < 50000000

# Total SOL across all signers
sum(signer_balance_lamports) / 1000000000

# Minimum signer balance (SOL)
min(signer_balance_lamports) / 1000000000
```

### Prometheus + Grafana Stack

From the Kora repository root:

```bash
just run-metrics
```

- Prometheus: http://localhost:9090
- Grafana dashboard: http://localhost:3000 (default: admin/admin)

---

## 14. Deployment

### Docker

```bash
docker pull ghcr.io/solana-foundation/kora:latest

docker run \
  -v $(pwd)/kora.toml:/app/kora.toml \
  -v $(pwd)/signers.toml:/app/signers.toml \
  -e RPC_URL=https://api.mainnet-beta.solana.com \
  -e KORA_PRIVATE_KEY=your-base58-key \
  -p 8080:8080 \
  ghcr.io/solana-foundation/kora:latest \
  rpc start --signers-config /app/signers.toml
```

Docker Compose with Redis (for caching and usage limits):

```yaml
version: "3"
services:
  kora:
    image: ghcr.io/solana-foundation/kora:latest
    ports:
      - "8080:8080"
    volumes:
      - ./kora.toml:/app/kora.toml
      - ./signers.toml:/app/signers.toml
    environment:
      - RPC_URL=https://api.mainnet-beta.solana.com
      - KORA_PRIVATE_KEY=${KORA_PRIVATE_KEY}
    command: rpc start --signers-config /app/signers.toml
    depends_on:
      - redis
  redis:
    image: redis:alpine
    ports:
      - "6379:6379"
```

### Railway

Step-by-step:

1. Create directory with `kora.toml`, `signers.toml`, `Dockerfile`
2. `railway login` → `railway init` → `railway up`
3. In Railway dashboard: Settings → Variables → add:
   - `RPC_URL` — your Solana RPC endpoint
   - `KORA_PRIVATE_KEY` — base58 private key
   - `RUST_LOG` — `info`
4. Settings → Generate domain → use port 8080
5. Redeploy

Test your deployment:

```bash
curl -X POST https://my-kora-node.railway.app \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getConfig","params":[]}'
```

---

## 15. Adding a Custom Signer to Kora

Kora uses the `solana-keychain` crate for all signing. Adding a new signer backend is a two-stage process.

### Stage 1: Add to solana-keychain

Before modifying Kora, the signer must be implemented in `solana-keychain`. Follow the solana-keychain ADDING_SIGNERS guide and submit a PR there first. Wait for PR approval and crate publication.

### Stage 2: Add Kora Configuration Support

Once your signer is in `solana-keychain`:

#### Step 1 — Update Cargo.toml

```toml
[dependencies]
solana-keychain = { version = "X.Y.Z", default-features = false, features = ["all", "sdk-v3"] }
```

#### Step 2 — Define Configuration Struct

In `crates/lib/src/signer/config.rs`:

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct YourServiceSignerConfig {
    pub api_key_env:    String,
    pub api_secret_env: String,
    pub wallet_id_env:  String,
}
```

#### Step 3 — Add to SignerTypeConfig Enum

```rust
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignerTypeConfig {
    Memory { #[serde(flatten)] config: MemorySignerConfig },
    // ... existing variants ...
    YourService {
        #[serde(flatten)]
        config: YourServiceSignerConfig,
    },
}
```

#### Step 4 — Add Build Logic

```rust
impl SignerConfig {
    pub async fn build_signer_from_config(config: &SignerConfig) -> Result<Signer, KoraError> {
        match &config.config {
            // ... existing cases
            SignerTypeConfig::YourService { config: c } => {
                Self::build_your_service_signer(c, &config.name).await
            }
        }
    }

    async fn build_your_service_signer(
        config: &YourServiceSignerConfig,
        signer_name: &str,
    ) -> Result<Signer, KoraError> {
        let api_key    = get_env_var_for_signer(&config.api_key_env, signer_name)?;
        let api_secret = get_env_var_for_signer(&config.api_secret_env, signer_name)?;
        let wallet_id  = get_env_var_for_signer(&config.wallet_id_env, signer_name)?;

        Signer::from_your_service(api_key, api_secret, wallet_id)
            .await
            .map_err(|e| KoraError::SigningError(format!(
                "Failed to create YourService signer '{signer_name}': {}",
                sanitize_error!(e)
            )))
    }
}
```

#### Step 5 — Add Validation Logic

```rust
impl SignerConfig {
    pub fn validate_individual_signer_config(&self, index: usize) -> Result<(), KoraError> {
        match &self.config {
            SignerTypeConfig::YourService { config } => {
                Self::validate_your_service_config(config, &self.name)
            }
            // ... other cases
        }
    }

    fn validate_your_service_config(
        config: &YourServiceSignerConfig,
        signer_name: &str,
    ) -> Result<(), KoraError> {
        let env_vars = [
            ("api_key_env",    &config.api_key_env),
            ("api_secret_env", &config.api_secret_env),
            ("wallet_id_env",  &config.wallet_id_env),
        ];
        for (field, val) in env_vars {
            if val.is_empty() {
                return Err(KoraError::ValidationError(format!(
                    "YourService signer '{signer_name}' must specify non-empty {field}"
                )));
            }
        }
        Ok(())
    }
}
```

#### Step 6 — Export in mod.rs

```rust
pub use config::{
    MemorySignerConfig,
    TurnkeySignerConfig,
    PrivySignerConfig,
    VaultSignerConfig,
    YourServiceSignerConfig,   // add this
};
```

#### Step 7 — Add Test Mock Builder

In `crates/lib/src/tests/config_mock.rs`:

```rust
impl SignerPoolConfigBuilder {
    pub fn with_your_service_signer(
        mut self,
        name: String,
        api_key_env: String,
        api_secret_env: String,
        wallet_id_env: String,
        weight: Option<u32>,
    ) -> Self {
        self.config.signers.push(SignerConfig {
            name,
            weight,
            config: SignerTypeConfig::YourService {
                config: YourServiceSignerConfig { api_key_env, api_secret_env, wallet_id_env },
            },
        });
        self
    }
}
```

#### Step 8 — Add Integration Test Fixture

`tests/src/common/fixtures/signers-your-service.toml`:

```toml
[signer_pool]
strategy = "round_robin"

[[signers]]
name = "yourservice_main"
type = "your_service"
api_key_env    = "YOUR_SERVICE_API_KEY"
api_secret_env = "YOUR_SERVICE_API_SECRET"
wallet_id_env  = "YOUR_SERVICE_WALLET_ID"
```

Run tests:

```bash
just test-integration
cargo run -p tests --bin test_runner -- --phases your_service
```

### PR Checklist

```
feat(signer): add YourService signer configuration support

- [ ] Signer implemented in solana-keychain crate and published
- [ ] Cargo.toml updated to latest solana-keychain version
- [ ] Configuration struct added
- [ ] SignerTypeConfig variant added
- [ ] build_signer_from_config arm added
- [ ] validate_individual_signer_config arm added
- [ ] Exported in mod.rs
- [ ] Test mock builder added (optional)
- [ ] Integration test fixture added
- [ ] Documentation added to SIGNERS.md
- [ ] Example .env vars added to .env.example
- [ ] Code compiles without warnings (just build)
- [ ] All tests pass (just test, just test-integration)
- [ ] No hardcoded values or secrets
- [ ] Linting passes (just fmt)
```

---

## 16. Security Best Practices

### Operator Fundamentals

**Use a dedicated keypair.** Never reuse your personal Solana wallet as the Kora signing key. Create a new keypair specifically for Kora and fund it with only what you are willing to spend on fees.

**Set conservative limits first.** Start with small `max_allowed_lamports`, a short whitelist in `allowed_programs`, and fully disabled `fee_payer_policy`. Expand only as needed.

**Always enable authentication in production.** Without it, anyone who discovers your endpoint can submit transactions and consume your SOL balance.

**Block dangerous Token-2022 extensions.** At minimum, block `permanent_delegate` for any token that could end up in your payment address. A malicious token with this extension can seize your payment funds.

**Disable durable transactions.** Leave `allow_durable_transactions = false` unless you have a specific requirement. Durable nonces allow signed transactions to be held and submitted at a strategically advantageous time.

### Fee Payer Policy Guidelines

| Scenario                       | Minimum Required Permissions                |
| ------------------------------ | ------------------------------------------- |
| Standard gasless transactions  | All disabled (default)                      |
| Jito bundle with Kora-paid tip | `system.allow_transfer = true`              |
| Kora creates ATAs for users    | `spl_token.allow_initialize_account = true` |
| Any fixed or free pricing      | All disabled + authentication required      |

### Key Management

- Store private keys in environment variables or secrets managers (Railway secrets, AWS Secrets Manager)
- Never commit private keys, API secrets, or PEM files to source control
- Use Turnkey, Privy, or Vault for production when possible — they never expose the raw private key
- Maintain minimal SOL balance with automated monitoring and top-up

### Monitoring and Alerting

Set up balance alerts before your node runs out of SOL:

```
# Prometheus alert: any signer below 0.05 SOL
min(signer_balance_lamports) < 50000000
```

Watch for unusual patterns:

- Spikes in `signTransaction` error rates
- Repeated requests from the same wallet near rate limits
- Unexpected high-outflow transactions

### Regular Maintenance

- Validate configuration after every change: `kora config validate`
- Review your `allowed_programs` list as your application evolves — remove programs you no longer use
- Update `disallowed_accounts` as you identify bad actors
- Rotate authentication keys periodically

---

## 17. Troubleshooting

### CLI

| Error                                        | Fix                                                                 |
| -------------------------------------------- | ------------------------------------------------------------------- |
| `kora: command not found`                    | Add `~/.cargo/bin` to PATH                                          |
| Build fails                                  | `rustup update stable`                                              |
| `--rpc-url required`                         | Set `RPC_URL` env var or pass `--rpc-url` flag                      |
| `error: a value is required for '--rpc-url'` | Seen in Railway — add `RPC_URL` to environment variables            |
| `JUPITER_API_KEY not set`                    | Set `JUPITER_API_KEY` env var when using `price_source = "Jupiter"` |

### Signers

| Error                                    | Fix                                                              |
| ---------------------------------------- | ---------------------------------------------------------------- |
| `At least one signer must be configured` | Add a `[[signers]]` entry or use `--no-load-signer`              |
| `Invalid base58 string`                  | Check key format, no extra whitespace                            |
| `Invalid private key length`             | Use a full 64-byte Solana keypair                                |
| `Turnkey {key} required`                 | Set all `TURNKEY_*` environment variables                        |
| `Privy {key} required`                   | Set all `PRIVY_*` environment variables                          |
| `Duplicate signer name`                  | Each signer in `[[signers]]` must have a unique `name`           |
| `Signer with pubkey ... not found`       | `signer_key` in client call does not match any configured signer |

### Transactions

| Error                           | Fix                                                                                    |
| ------------------------------- | -------------------------------------------------------------------------------------- |
| Transaction validation fails    | Verify all programs and tokens are in the allowlist                                    |
| `max_allowed_lamports` exceeded | Increase limit or reduce transaction complexity                                        |
| Payment instruction rejected    | Check ATA initialization, token is in `allowed_spl_paid_tokens`                        |
| Signature verification fails    | Ensure user signed before sending to Kora, transaction not modified after user signing |
| Stale blockhash                 | Always fetch a fresh blockhash for the final transaction                               |

### SDK

| Error                    | Fix                                                                 |
| ------------------------ | ------------------------------------------------------------------- |
| Peer dependency warnings | `pnpm add @solana/kit @solana-program/token`                        |
| TypeScript errors        | Ensure TypeScript 4.5+, add `@types/node`                           |
| Connection refused       | Kora server not running or wrong URL                                |
| 401 Unauthorized         | Check API key or HMAC headers match server config                   |
| 401 HMAC timestamp       | Timestamp must be within `max_timestamp_age` seconds of server time |

### Enable Debug Logging

```bash
RUST_LOG=debug kora --config kora.toml rpc start --signers-config signers.toml
```

---

## Quick Reference

### Minimum Working Setup

```toml
# kora.toml
[kora]
rate_limit = 10

[validation]
price_source            = "Mock"
max_allowed_lamports    = 1000000
max_signatures          = 10
allowed_programs        = ["11111111111111111111111111111111", "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL", "ComputeBudget11111111111111111111111111111111"]
allowed_tokens          = ["YOUR_TOKEN_MINT"]
allowed_spl_paid_tokens = ["YOUR_TOKEN_MINT"]
disallowed_accounts     = []

[validation.price]
type = "margin"
margin = 0.1
```

```toml
# signers.toml
[signer_pool]
strategy = "round_robin"

[[signers]]
name = "main"
type = "memory"
private_key_env = "KORA_PRIVATE_KEY"
```

```bash
# .env
KORA_PRIVATE_KEY="your-base58-private-key"
RPC_URL="http://127.0.0.1:8899"
```

```bash
# Start
kora --config kora.toml rpc start --signers-config signers.toml
```

### Startup Checklist

```
[ ] kora.toml: allowed_programs includes every program transactions will call
[ ] kora.toml: allowed_spl_paid_tokens includes your payment token
[ ] kora.toml: price_source="Jupiter" has JUPITER_API_KEY set
[ ] kora.toml: fee_payer_policy is all-false unless specifically needed
[ ] kora.toml: auth configured for production
[ ] signers.toml: signer env vars are set in .env
[ ] Signer wallet funded with SOL
[ ] ATAs initialized: kora rpc initialize-atas --signers-config signers.toml
[ ] Config validated: kora config validate-with-rpc
```
