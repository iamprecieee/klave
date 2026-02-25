#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub api_key: String,
    pub solana_rpc_url: String,
    pub kora_rpc_url: String,
    pub kora_api_key: Option<String>,
    pub kora_pubkey: String,
    pub encryption_key: [u8; 32],
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("KLAVE_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:klave.db?mode=rwc".to_string()),
            port: std::env::var("KLAVE_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            api_key: std::env::var("KLAVE_API_KEY").unwrap_or_else(|_| "klave-dev-key".to_string()),
            solana_rpc_url: std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
            kora_rpc_url: std::env::var("KORA_RPC_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            kora_api_key: std::env::var("KORA_API_KEY").ok(),
            kora_pubkey: std::env::var("KORA_PUBKEY")
                .unwrap_or_else(|_| "KoraGateway11111111111111111111111111111111".to_string()),
            encryption_key: klave_core::crypto::parse_hex_key(
                &std::env::var("KLAVE_ENCRYPTION_KEY").unwrap_or_else(|_| "ab".repeat(32)),
            )
            .expect("KLAVE_ENCRYPTION_KEY must be 64 hex chars"),
        }
    }
}
