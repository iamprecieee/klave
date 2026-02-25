# solana-keychain — Complete Reference

## What is solana-keychain

`solana-keychain` is a unified Solana transaction signing library for Rust and TypeScript applications. It provides a single `SolanaSigner` trait (Rust) and compatible interface (TypeScript) that works across multiple key management backends — from local keypairs in development all the way to institutional MPC custody in production.

The core value is backend portability: you write your signing logic once against the `SolanaSigner` interface, then swap the underlying key management provider (memory, Vault, AWS KMS, Privy, Turnkey, Fireblocks) by changing configuration, not code.

> **Security Notice**: This library has not been audited. Not recommended for production use with real funds without a thorough security review. Authors are not responsible for loss of funds.

---

## Table of Contents

1. Architecture Overview
2. Supported Backends
3. Rust — Installation and Feature Flags
4. Rust — Core API (SolanaSigner Trait)
5. Rust — Backend Configuration
6. Rust — Unified Signer Enum
7. TypeScript — Installation
8. TypeScript — Backend Configuration
9. TypeScript — Framework Compatibility
10. Adding a Custom Signer Backend
11. AWS KMS — Credential Setup
12. Error Handling
13. Security Best Practices
14. Development Workflow
15. Dependency Reference

---

## 1. Architecture Overview

```
Your Application Code
        │
        ▼
  SolanaSigner trait  ◄──── single interface for all backends
        │
   ┌────┴──────────────────────────────────┐
   │              Signer enum              │
   └──┬────────┬──────────┬───────────────┘
      │        │          │
  Memory    Vault      AWS KMS   Privy   Turnkey   Fireblocks
  (local)  (HSM)    (cloud KMS)
```

**Languages:** Rust + TypeScript  
**Rust compatibility:** `solana-sdk` and `solana-sdk-v3`  
**TypeScript compatibility:** `@solana/kit` and `@solana/signers`

The library is modular by design. Feature flags in Rust and separate npm packages in TypeScript mean you only include the code for the backends you actually use.

---

## 2. Supported Backends

| Backend             | Use Case                             | Rust                 | TypeScript                             |
| ------------------- | ------------------------------------ | -------------------- | -------------------------------------- |
| **Memory**          | Local keypairs, development, testing | `memory` feature     | via `@solana/signers` or `@solana/kit` |
| **HashiCorp Vault** | Self-hosted HSM with Transit engine  | `vault` feature      | `@solana/keychain-vault`               |
| **AWS KMS**         | Cloud-native Ed25519 signing         | `aws_kms` feature    | `@solana/keychain-aws-kms`             |
| **Privy**           | Embedded wallets                     | `privy` feature      | `@solana/keychain-privy`               |
| **Turnkey**         | Non-custodial key management         | `turnkey` feature    | `@solana/keychain-turnkey`             |
| **Fireblocks**      | Institutional MPC custody            | `fireblocks` feature | `@solana/keychain-fireblocks`          |

Choose based on your operational environment:

- **Local development / testing** → Memory
- **Self-hosted infrastructure, want control of HSM** → Vault
- **Already on AWS, want cloud-managed keys** → AWS KMS
- **Building a consumer app with embedded wallets** → Privy
- **Non-custodial product where users retain key ownership** → Turnkey
- **Institutional / exchange / regulated environment** → Fireblocks

---

## 3. Rust — Installation and Feature Flags

Add to `Cargo.toml`:

```toml
# Memory signer only (default — for development)
solana-keychain = "0.2.1"

# Specific backends only (zero-cost — only compile what you use)
solana-keychain = { version = "0.2.1", default-features = false, features = ["vault", "aws_kms"] }

# All backends
solana-keychain = { version = "0.2.1", features = ["all"] }
```

### Feature Flag Reference

| Feature      | Description                    | Pulls In                                     |
| ------------ | ------------------------------ | -------------------------------------------- |
| `memory`     | Local keypair signing          | (default, minimal)                           |
| `vault`      | HashiCorp Vault Transit engine | `reqwest`, `vaultrs`, `base64`               |
| `aws_kms`    | AWS KMS with Ed25519           | AWS SDK crates                               |
| `privy`      | Privy embedded wallets         | `reqwest`, `base64`                          |
| `turnkey`    | Turnkey API                    | `reqwest`, `base64`, `p256`, `hex`, `chrono` |
| `fireblocks` | Fireblocks MPC                 | Fireblocks SDK                               |
| `all`        | Every backend                  | All of the above                             |

Use `default-features = false` with explicit features when deploying to production. This keeps compile times shorter and the binary smaller.

---

## 4. Rust — Core API (SolanaSigner Trait)

All backends implement this single trait, defined in `src/traits.rs`:

```rust
#[async_trait]
pub trait SolanaSigner: Send + Sync {
    /// Returns the public key of this signer.
    fn pubkey(&self) -> Pubkey;

    /// Signs a Solana transaction in place.
    /// Returns the produced signature.
    async fn sign_transaction(&self, tx: &mut Transaction) -> Result<Signature, SignerError>;

    /// Signs arbitrary bytes.
    async fn sign_message(&self, message: &[u8]) -> Result<Signature, SignerError>;

    /// Returns true if the signer is reachable and healthy.
    /// For local signers this is always true. For remote signers (Vault, KMS, etc.)
    /// this makes a health-check network call.
    async fn is_available(&self) -> bool;
}
```

### Using Any Backend Generically

Write your application logic against the trait and it works with any backend:

```rust
use solana_keychain::{SolanaSigner, SignerError};
use solana_sdk::transaction::Transaction;

async fn sign_and_send(
    signer: &impl SolanaSigner,
    tx: &mut Transaction,
) -> Result<(), SignerError> {
    let pubkey = signer.pubkey();
    println!("Signing with: {}", pubkey);

    // For remote signers — check availability before signing
    if !signer.is_available().await {
        return Err(SignerError::NotAvailable("Signer offline".into()));
    }

    let signature = signer.sign_transaction(tx).await?;
    println!("Signature: {}", signature);
    Ok(())
}
```

`is_available()` is a no-op for `MemorySigner` (always returns true). For remote backends (Vault, AWS KMS, etc.) it performs a lightweight health-check call. Calling it before every transaction in a hot path may add latency — consider caching the result or only calling it during startup.

---

## 5. Rust — Backend Configuration

All backends are constructed through the unified `Signer` enum's factory methods. You can also construct the underlying signer struct directly if you need the concrete type.

### Memory Signer

For local development and testing. Uses in-process keypairs — no network calls.

```rust
use solana_keychain::Signer;

// From base58-encoded private key string
let signer = Signer::from_memory("base58_private_key")?;

// From a keypair JSON file path (same format as solana-keygen output)
let signer = Signer::from_memory("/path/to/keypair.json")?;

// From a byte array string (same format as Solana CLI output)
let signer = Signer::from_memory("[41,99,180,88,51,57,48,80,...]")?;
```

### HashiCorp Vault

For self-hosted HSM using Vault's Transit engine. Vault never exposes the private key — it signs inside the engine and returns the signature.

```rust
use solana_keychain::Signer;

let signer = Signer::from_vault(
    "https://vault.example.com:8200".to_string(), // Vault server address
    "hvs.xxxxx".to_string(),                       // Vault token
    "my-solana-key".to_string(),                   // Key name in Transit engine
    "base58_public_key".to_string(),               // Expected public key (for validation)
)?;
```

The `public_key` parameter is used for local validation — it confirms the Vault key you specified is the one you expect before making any signing requests.

### AWS KMS

For cloud-native signing with AWS KMS using Ed25519 keys.

```rust
use solana_keychain::Signer;

let signer = Signer::from_aws_kms(
    "alias/my-solana-key".to_string(),   // KMS key ID, alias, or full ARN
    "base58_public_key".to_string(),     // Expected public key (for validation)
    Some("us-east-1".to_string()),       // AWS region (None uses default from env)
).await?;
```

This is an `async` constructor because it verifies the key exists and retrieves key metadata from AWS on creation.

KMS key ID formats accepted:

- Key ID: `12345678-1234-1234-1234-123456789012`
- Key alias: `alias/my-solana-key`
- Full ARN: `arn:aws:kms:us-east-1:123456789012:key/12345678-...`

See Section 11 for AWS credential setup.

### Privy

For embedded wallet signing via Privy's infrastructure.

```rust
use solana_keychain::Signer;

let signer = Signer::from_privy(
    "app_id".to_string(),       // Privy application ID
    "app_secret".to_string(),   // Privy application secret
    "wallet_id".to_string(),    // Wallet ID within your Privy app
).await?;
```

`app_id` and `app_secret` come from your Privy dashboard. `wallet_id` is the identifier of the specific wallet you want to sign with.

### Turnkey

For non-custodial signing where users retain key ownership.

```rust
use solana_keychain::Signer;

let signer = Signer::from_turnkey(
    "api_public_key".to_string(),    // Turnkey API public key
    "api_private_key".to_string(),   // Turnkey API private key
    "org_id".to_string(),            // Turnkey organization ID
    "private_key_id".to_string(),    // Private key ID within Turnkey
    "base58_public_key".to_string(), // Expected public key (for validation)
)?;
```

The `api_public_key` and `api_private_key` are your Turnkey API credentials (not the Solana signing key itself). Turnkey uses these to authenticate your API requests.

### Fireblocks

For institutional MPC custody.

```rust
use solana_keychain::{Signer, FireblocksSignerConfig};

let config = FireblocksSignerConfig {
    api_key: "your-fireblocks-api-key".to_string(),
    private_key_pem: "-----BEGIN RSA PRIVATE KEY-----\n...\n-----END RSA PRIVATE KEY-----".to_string(),
    vault_account_id: "0".to_string(),    // Fireblocks vault account ID
    asset_id: "SOL".to_string(),          // "SOL" for mainnet, "SOL_TEST" for devnet
};

let signer = Signer::from_fireblocks(config).await?;
```

`private_key_pem` is the RSA private key used to authenticate your Fireblocks API requests (not the Solana signing key). Store this securely — never commit it to source control.

---

## 6. Rust — Unified Signer Enum

The `Signer` enum wraps all backends behind a single type. Use it when you need to select backends at runtime (e.g., based on a config file or environment variable).

```rust
use solana_keychain::{Signer, SolanaSigner, SignerError};

async fn get_signer(backend: &str, config: &Config) -> Result<Signer, SignerError> {
    match backend {
        "memory" => Signer::from_memory(&config.private_key),
        "vault"  => Signer::from_vault(
            config.vault_addr.clone(),
            config.vault_token.clone(),
            config.vault_key_name.clone(),
            config.public_key.clone(),
        ),
        "kms"    => Signer::from_aws_kms(
            config.kms_key_id.clone(),
            config.public_key.clone(),
            config.aws_region.clone(),
        ).await,
        _        => Err(SignerError::ConfigError("Unknown backend".into())),
    }
}

// Use it — same interface regardless of backend
let signer = get_signer("kms", &config).await?;
let pubkey  = signer.pubkey();
let sig     = signer.sign_transaction(&mut tx).await?;
```

The `Signer` enum itself implements `SolanaSigner`, so you can pass it anywhere a `&impl SolanaSigner` is expected without needing dynamic dispatch (`Box<dyn SolanaSigner>`).

---

## 7. TypeScript — Installation

Each backend is a separate npm package. Install only what you need:

```bash
# Core (required for all backends)
pnpm add @solana/keychain

# Individual backends
pnpm add @solana/keychain-vault
pnpm add @solana/keychain-aws-kms
pnpm add @solana/keychain-privy
pnpm add @solana/keychain-turnkey
pnpm add @solana/keychain-fireblocks
```

---

## 8. TypeScript — Backend Configuration

### HashiCorp Vault

```typescript
import { VaultSigner } from "@solana/keychain-vault";

const signer = new VaultSigner({
  vaultAddr: "https://vault.example.com:8200",
  vaultToken: "hvs.xxxxx",
  keyName: "my-solana-key",
  publicKey: "base58_public_key",
});
```

### AWS KMS

```typescript
import { AwsKmsSigner } from "@solana/keychain-aws-kms";

const signer = new AwsKmsSigner({
  keyId: "alias/my-solana-key",
  publicKey: "base58_public_key",
  region: "us-east-1",
});
```

### Privy

```typescript
import { PrivySigner } from "@solana/keychain-privy";

const signer = new PrivySigner({
  appId: "your-privy-app-id",
  appSecret: "your-privy-app-secret",
  walletId: "your-wallet-id",
});
```

### Turnkey

```typescript
import { TurnkeySigner } from "@solana/keychain-turnkey";

const signer = new TurnkeySigner({
  apiPublicKey: "your-turnkey-api-public-key",
  apiPrivateKey: "your-turnkey-api-private-key",
  organizationId: "your-org-id",
  privateKeyId: "your-private-key-id",
  publicKey: "base58_public_key",
});
```

### Fireblocks

```typescript
import { FireblocksSigner } from "@solana/keychain-fireblocks";

const signer = new FireblocksSigner({
  apiKey: "your-fireblocks-api-key",
  privateKeyPath: "/path/to/fireblocks_secret.key",
  vaultAccountId: "0",
  assetId: "SOL", // or "SOL_TEST" for devnet
});
```

---

## 9. TypeScript — Framework Compatibility

All TypeScript keychain signers are compatible with both `@solana/kit` (the new Solana web3 SDK) and `@solana/signers`. They implement the standard `TransactionSigner` interface, so they plug directly into the `@solana/kit` transaction builder pipeline.

### With @solana/kit

```typescript
import { VaultSigner } from "@solana/keychain-vault";
import { signTransactionMessageWithSigners } from "@solana/signers";
import {
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  appendTransactionMessageInstruction,
  pipe,
} from "@solana/kit";

async function buildAndSign() {
  const signer = new VaultSigner({
    vaultAddr: "https://vault.example.com:8200",
    vaultToken: "hvs.xxxxx",
    keyName: "my-solana-key",
    publicKey: "base58_public_key",
  });

  // Build the transaction message using @solana/kit's pipe pattern
  const transactionMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(signer, tx),
    (tx) => appendTransactionMessageInstruction(myInstruction, tx),
  );

  // Sign using the standard signers API
  const signedTx = await signTransactionMessageWithSigners(transactionMessage);
  return signedTx;
}
```

Since all keychain signers implement the standard `@solana/signers` interface, any function that accepts a `TransactionSigner` will work with any keychain backend. You can swap `VaultSigner` for `AwsKmsSigner` or any other backend without changing the transaction building code.

---

## 10. Adding a Custom Signer Backend

This section documents the complete process for implementing a new signing backend. Follow this when integrating a key management provider not currently supported by the library.

### Architecture of a Backend

Each backend lives in its own directory under `src/`:

```
src/
├── your_service/
│   ├── mod.rs      # Signer struct + SolanaSigner impl
│   └── types.rs    # API request/response types
```

### Step 1 — Define the Signer Struct

In `src/your_service/mod.rs`:

```rust
//! YourService signer integration
use crate::{error::SignerError, traits::SolanaSigner};
use solana_sdk::{pubkey::Pubkey, signature::Signature, transaction::Transaction};
use std::str::FromStr;

/// YourService-based signer
#[derive(Clone)]
pub struct YourServiceSigner {
    api_key:      String,
    api_secret:   String,
    wallet_id:    String,
    api_base_url: String,
    client:       reqwest::Client,
    public_key:   Pubkey,
}

// Debug impl must NOT expose sensitive fields
impl std::fmt::Debug for YourServiceSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("YourServiceSigner")
            .field("public_key", &self.public_key)
            .finish_non_exhaustive()  // hides api_key, api_secret
    }
}
```

### Step 2 — Implement Constructor and Internal Sign Method

```rust
impl YourServiceSigner {
    /// Create a new signer.
    ///
    /// # Arguments
    /// * `api_key`    - YourService API key
    /// * `api_secret` - YourService API secret
    /// * `wallet_id`  - Wallet ID
    /// * `public_key` - Base58 Solana public key (for local validation)
    pub fn new(
        api_key:    String,
        api_secret: String,
        wallet_id:  String,
        public_key: String,
    ) -> Result<Self, SignerError> {
        let pubkey = Pubkey::from_str(&public_key)
            .map_err(|e| SignerError::InvalidPublicKey(format!("Invalid public key: {e}")))?;

        Ok(Self {
            api_key,
            api_secret,
            wallet_id,
            api_base_url: "https://api.yourservice.com/v1".to_string(),
            client: reqwest::Client::new(),
            public_key: pubkey,
        })
    }

    /// Internal: call the signing API and return a Solana Signature.
    async fn sign(&self, message: &[u8]) -> Result<Signature, SignerError> {
        // 1. Encode message for transmission (base64 is common)
        let encoded_message = base64::engine::general_purpose::STANDARD.encode(message);

        // 2. Call the API
        let url      = format!("{}/sign", self.api_base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "wallet_id": self.wallet_id,
                "message":   encoded_message,
            }))
            .send()
            .await?;

        // 3. Handle HTTP errors
        if !response.status().is_success() {
            let status     = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error body".to_string());
            return Err(SignerError::RemoteApiError(format!("API error {status}: {error_text}")));
        }

        // 4. Parse signature from response
        let response_data: SignResponse = response.json().await?;
        let sig_bytes = base64::engine::general_purpose::STANDARD
            .decode(&response_data.signature)
            .map_err(|e| SignerError::SerializationError(format!("Decode failed: {e}")))?;

        // 5. Convert to Solana Signature — must be exactly 64 bytes
        let sig_array: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| SignerError::SigningFailed("Invalid signature length".to_string()))?;

        Ok(Signature::from(sig_array))
    }
}
```

### Step 3 — Implement SolanaSigner

```rust
#[async_trait::async_trait]
impl SolanaSigner for YourServiceSigner {
    fn pubkey(&self) -> Pubkey {
        self.public_key
    }

    async fn sign_transaction(&self, tx: &mut Transaction) -> Result<Signature, SignerError> {
        // Serialize the full transaction for signing
        let serialized = bincode::serialize(tx).map_err(|e| {
            SignerError::SerializationError(format!("Failed to serialize transaction: {e}"))
        })?;
        self.sign(&serialized).await
    }

    async fn sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        self.sign(message).await
    }

    async fn is_available(&self) -> bool {
        // Lightweight health check — should not require auth if possible
        let url = format!("{}/health", self.api_base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}
```

### Step 4 — Define API Types

In `src/your_service/types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct SignRequest {
    pub wallet_id: String,
    pub message:   String,
}

#[derive(Deserialize)]
pub struct SignResponse {
    pub signature: String,
}
```

### Step 5 — Add Feature Flag in Cargo.toml

```toml
[features]
default      = ["memory"]
memory       = []
vault        = ["dep:reqwest", "dep:vaultrs", "dep:base64"]
your_service = ["dep:reqwest", "dep:base64"]   # add this
all          = ["memory", "vault", "your_service"]  # add to all

[dependencies]
reqwest = { version = "...", optional = true }
base64  = { version = "...", optional = true }
```

### Step 6 — Register in src/lib.rs

```rust
// Feature-gated module
#[cfg(feature = "your_service")]
pub mod your_service;

// Re-export the signer type
#[cfg(feature = "your_service")]
pub use your_service::YourServiceSigner;

// Add to Signer enum
#[derive(Debug)]
pub enum Signer {
    #[cfg(feature = "memory")]
    Memory(MemorySigner),
    // ... existing variants
    #[cfg(feature = "your_service")]
    YourService(YourServiceSigner),
}

// Add factory method on Signer
impl Signer {
    #[cfg(feature = "your_service")]
    pub fn from_your_service(
        api_key:    String,
        api_secret: String,
        wallet_id:  String,
        public_key: String,
    ) -> Result<Self, SignerError> {
        Ok(Self::YourService(YourServiceSigner::new(
            api_key, api_secret, wallet_id, public_key,
        )?))
    }
}

// Delegate all trait methods in SolanaSigner impl for Signer enum
#[async_trait::async_trait]
impl SolanaSigner for Signer {
    fn pubkey(&self) -> Pubkey {
        match self {
            // ... existing arms
            #[cfg(feature = "your_service")]
            Signer::YourService(s) => s.pubkey(),
        }
    }

    async fn sign_transaction(&self, tx: &mut Transaction) -> Result<Signature, SignerError> {
        match self {
            // ... existing arms
            #[cfg(feature = "your_service")]
            Signer::YourService(s) => s.sign_transaction(tx).await,
        }
    }

    async fn sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        match self {
            // ... existing arms
            #[cfg(feature = "your_service")]
            Signer::YourService(s) => s.sign_message(message).await,
        }
    }

    async fn is_available(&self) -> bool {
        match self {
            // ... existing arms
            #[cfg(feature = "your_service")]
            Signer::YourService(s) => s.is_available().await,
        }
    }
}
```

### Step 7 — Write Tests

Use `wiremock` to mock the HTTP API. Tests must not make real network calls.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{signature::Keypair, signer::Signer as SdkSigner};
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_new_valid_pubkey() {
        let keypair = Keypair::new();
        let result  = YourServiceSigner::new(
            "test-key".to_string(),
            "test-secret".to_string(),
            "test-wallet".to_string(),
            keypair.pubkey().to_string(),
        );
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_new_invalid_pubkey() {
        let result = YourServiceSigner::new(
            "k".to_string(), "s".to_string(), "w".to_string(),
            "not-a-valid-pubkey".to_string(),
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sign_message_success() {
        let mock_server = MockServer::start().await;
        let keypair     = Keypair::new();
        let message     = b"test message";
        let signature   = keypair.sign_message(message);

        Mock::given(method("POST"))
            .and(path("/sign"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "signature": base64::engine::general_purpose::STANDARD.encode(signature.as_ref())
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let mut signer = YourServiceSigner::new(
            "test-key".to_string(),
            "test-secret".to_string(),
            "test-wallet".to_string(),
            keypair.pubkey().to_string(),
        ).unwrap();
        signer.api_base_url = mock_server.uri();  // point at mock server

        let result = signer.sign_message(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sign_message_unauthorized() {
        let mock_server = MockServer::start().await;
        let keypair     = Keypair::new();

        Mock::given(method("POST"))
            .and(path("/sign"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&mock_server)
            .await;

        let mut signer = YourServiceSigner::new(
            "bad-key".to_string(),
            "bad-secret".to_string(),
            "wallet".to_string(),
            keypair.pubkey().to_string(),
        ).unwrap();
        signer.api_base_url = mock_server.uri();

        let result = signer.sign_message(b"test").await;
        assert!(result.is_err());
        // Optionally check the error variant
        matches!(result.unwrap_err(), SignerError::RemoteApiError(_));
    }

    #[tokio::test]
    async fn test_is_available_healthy() {
        let mock_server = MockServer::start().await;
        let keypair     = Keypair::new();

        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let mut signer = YourServiceSigner::new(
            "k".to_string(), "s".to_string(), "w".to_string(),
            keypair.pubkey().to_string(),
        ).unwrap();
        signer.api_base_url = mock_server.uri();

        assert!(signer.is_available().await);
    }
}
```

### Step 8 — Update README.md

Add a row to the supported backends table and a usage example in the documentation.

### PR Checklist

```
feat(signer): add YourService signer integration

- [ ] Code compiles without warnings (just build)
- [ ] Code is formatted and linting passes (just fmt)
- [ ] All tests pass (just test)
- [ ] No hardcoded values, credentials, or secrets in code
- [ ] Debug impl hides sensitive fields
- [ ] Error messages are descriptive and use existing SignerError variants
- [ ] Feature flag added to Cargo.toml and `all` feature updated
- [ ] Signer variant added to enum with all trait method arms
- [ ] Added to README.md supported backends table
- [ ] wiremock tests for success, auth failure, and health check
```

### Reference Implementations

Study these in order of increasing complexity:

| File                 | Pattern                                     |
| -------------------- | ------------------------------------------- |
| `src/memory/mod.rs`  | Simple, synchronous, no external calls      |
| `src/privy/mod.rs`   | Async with initialization                   |
| `src/vault/mod.rs`   | Delegates to external client library        |
| `src/turnkey/mod.rs` | Complex signature handling and request auth |

---

## 11. AWS KMS — Credential Setup

The AWS KMS signer uses the **AWS default credential provider chain**. No credentials are passed to the constructor. They are loaded automatically from the environment.

### Credential Resolution Order

1. Environment variables: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`
2. Shared credentials file: `~/.aws/credentials`
3. IAM role (automatic on EC2, ECS, Lambda — no credentials file needed)
4. Web identity token for EKS/Kubernetes with IRSA

### Recommended Setup by Environment

| Environment                      | Method                                        |
| -------------------------------- | --------------------------------------------- |
| Production on AWS EC2/ECS/Lambda | IAM role (nothing to configure)               |
| Local development                | `~/.aws/credentials` or environment variables |
| CI/CD pipelines                  | Environment variables or OIDC federation      |
| Kubernetes (EKS)                 | IRSA (IAM Roles for Service Accounts)         |

### Creating the KMS Key

The key must use the `ECC_NIST_EDWARDS25519` spec (Ed25519). This is the algorithm Solana uses.

```bash
aws kms create-key \
  --key-spec ECC_NIST_EDWARDS25519 \
  --key-usage SIGN_VERIFY \
  --description "Solana transaction signing key"
```

Create an alias for easier reference:

```bash
aws kms create-alias \
  --alias-name alias/my-solana-key \
  --target-key-id <key-id-from-above>
```

### Required IAM Permissions

The IAM role or user your application runs as needs at minimum:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": ["kms:Sign", "kms:DescribeKey"],
      "Resource": "arn:aws:kms:us-east-1:123456789012:key/YOUR-KEY-ID"
    }
  ]
}
```

`kms:Sign` — required to sign transactions.  
`kms:DescribeKey` — required on construction to validate the key exists and retrieve metadata.

Scope the `Resource` to the specific key ARN in production. Wildcard (`*`) is only acceptable in development.

---

## 12. Error Handling

All signing operations return `Result<T, SignerError>`. The `SignerError` enum covers the standard failure categories across all backends.

### SignerError Variants

| Variant                      | When It Occurs                                                    |
| ---------------------------- | ----------------------------------------------------------------- |
| `NotAvailable(String)`       | `is_available()` returned false, or signer could not be reached   |
| `RemoteApiError(String)`     | HTTP error from Vault, KMS, Privy, Turnkey, Fireblocks API        |
| `SigningFailed(String)`      | The signing operation itself failed (e.g. wrong signature length) |
| `SerializationError(String)` | Failed to encode/decode message or signature                      |
| `InvalidPublicKey(String)`   | Provided base58 public key could not be parsed                    |
| `ConfigError(String)`        | Invalid configuration (unknown backend, missing required fields)  |

### Handling Errors

```rust
use solana_keychain::{SolanaSigner, SignerError};

match signer.sign_transaction(&mut tx).await {
    Ok(sig) => {
        println!("Signed: {}", sig);
    }
    Err(SignerError::NotAvailable(msg)) => {
        eprintln!("Signer offline: {}", msg);
        // Retry or circuit break
    }
    Err(SignerError::RemoteApiError(msg)) => {
        eprintln!("API error: {}", msg);
        // Log for alerting, do not retry immediately
    }
    Err(e) => {
        eprintln!("Signing failed: {}", e);
    }
}
```

### Implementation Guidance

When implementing a custom backend, always use existing `SignerError` variants rather than creating new ones. If your use case genuinely requires a new variant, propose it in the PR.

```rust
// Good — maps to existing variants
.map_err(|e| SignerError::RemoteApiError(format!("API returned {}: {}", status, e)))?

// Good — wraps standard errors
.map_err(|e| SignerError::SerializationError(format!("Base64 decode failed: {e}")))?

// Bad — creating new error types not in the library
Err(MyCustomError::NetworkTimeout)  // don't do this
```

---

## 13. Security Best Practices

### Never Log Sensitive Data

```rust
// Bad — leaks the API key
println!("Creating signer with key: {}", self.api_key);

// Good — hide sensitive fields in Debug
impl std::fmt::Debug for MySigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MySigner")
            .field("public_key", &self.public_key)
            .finish_non_exhaustive()  // omits api_key, api_secret, etc.
    }
}
```

### Validate Inputs Early

Validate the public key in the constructor, not at signing time. This catches misconfiguration at startup rather than during a live transaction.

```rust
let pubkey = Pubkey::from_str(&public_key)
    .map_err(|e| SignerError::InvalidPublicKey(format!("Invalid key: {e}")))?;
```

### Always Use HTTPS

Remote API calls must use HTTPS endpoints. Never send signing requests over plain HTTP, even in development.

### Store Secrets Securely

- Do not hardcode API keys, tokens, or PEM files in source code
- Use environment variables or a secrets manager (Vault, AWS Secrets Manager)
- Private key PEM files (Fireblocks) should never be committed to version control

### Signature Validation

For backends where you control the key (Memory), you can verify the signature locally. For remote backends, trust the service but validate the signature length:

```rust
// Ensure returned signature is exactly 64 bytes before constructing
let sig_array: [u8; 64] = sig_bytes
    .try_into()
    .map_err(|_| SignerError::SigningFailed("Expected 64-byte signature".to_string()))?;
```

### is_available() in Hot Paths

`is_available()` makes a network call for remote backends. Calling it before every transaction in a high-throughput path adds latency. Consider:

- Calling it once at startup to verify connectivity
- Using a background health-check loop with a shared atomic flag
- Only calling it when a signing operation fails (circuit breaker pattern)

---

## 14. Development Workflow

The project uses [Just](https://github.com/casey/just) as a task runner. Install it before running any project commands.

```bash
# Install just (Mac)
brew install just

# Install just (Linux)
cargo install just
```

### Common Commands

```bash
just build   # compile all features, checks for warnings
just test    # run all tests across all features
just fmt     # format code with rustfmt and run clippy
```

### Running Tests for a Specific Feature

```bash
# Only your backend
cargo test --features your_service

# All backends
cargo test --all-features

# Specific test by name
cargo test --features vault test_vault_sign_message
```

### Testing with Mocks

Tests must use `wiremock` to mock HTTP endpoints. Tests that make real network calls to external services are not acceptable in the main test suite (they require credentials, are slow, and are flaky).

```bash
# Add wiremock to dev-dependencies
cargo add wiremock --dev
```

---

## 15. Dependency Reference

### Rust Cargo.toml

```toml
# Minimal — memory signer only
[dependencies]
solana-keychain = "0.2.1"

# Specific backends
[dependencies]
solana-keychain = { version = "0.2.1", default-features = false, features = ["aws_kms", "vault"] }

# Everything
[dependencies]
solana-keychain = { version = "0.2.1", features = ["all"] }

# For implementing a custom backend
[dependencies]
async-trait = "0.1"
reqwest     = { version = "0.11", features = ["json"] }
serde       = { version = "1.0", features = ["derive"] }
serde_json  = "1.0"
base64      = "0.21"
bincode     = "1.3"
tokio       = { version = "1.0", features = ["full"] }

[dev-dependencies]
wiremock = "0.5"
tokio    = { version = "1.0", features = ["full", "test-util"] }
```

### TypeScript package.json

```json
{
  "dependencies": {
    "@solana/keychain": "latest",
    "@solana/keychain-vault": "latest",
    "@solana/keychain-aws-kms": "latest",
    "@solana/keychain-privy": "latest",
    "@solana/keychain-turnkey": "latest",
    "@solana/keychain-fireblocks": "latest",
    "@solana/kit": "latest",
    "@solana/signers": "latest"
  }
}
```

---

## Quick Reference

### Rust — Pick Your Backend

```rust
// Development
let signer = Signer::from_memory("base58_key")?;

// Self-hosted HSM
let signer = Signer::from_vault(addr, token, key_name, pubkey)?;

// AWS cloud
let signer = Signer::from_aws_kms(key_id, pubkey, region).await?;

// Embedded wallets
let signer = Signer::from_privy(app_id, app_secret, wallet_id).await?;

// Non-custodial
let signer = Signer::from_turnkey(api_pub, api_priv, org_id, key_id, pubkey)?;

// Institutional
let signer = Signer::from_fireblocks(config).await?;
```

### Rust — Use Any Signer Uniformly

```rust
let pubkey    = signer.pubkey();
let available = signer.is_available().await;
let sig       = signer.sign_transaction(&mut tx).await?;
let msg_sig   = signer.sign_message(b"hello").await?;
```

### Custom Backend Checklist

1. `src/your_service/mod.rs` — struct, constructor, `sign()`, `SolanaSigner` impl
2. `src/your_service/types.rs` — API request/response types
3. `Cargo.toml` — feature flag + optional dependencies
4. `src/lib.rs` — module declaration, re-export, enum variant, factory method, trait delegation
5. Tests — constructor, sign success, sign failure (4xx), health check
6. README.md — backends table row + usage example
