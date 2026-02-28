#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub operator_api_key: String,
    pub solana_rpc_url: String,
    pub kora_rpc_url: String,
    pub kora_api_key: Option<String>,
    pub kora_pubkey: String,
    pub encryption_key: [u8; 32],
    pub jupiter_api_key: Option<String>,
    pub allowed_origins: Vec<String>,
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
            operator_api_key: std::env::var("KLAVE_OPERATOR_API_KEY")
                .expect("KLAVE_OPERATOR_API_KEY must be set"),
            solana_rpc_url: std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
            kora_rpc_url: std::env::var("KORA_RPC_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            kora_api_key: std::env::var("KORA_API_KEY").ok(),
            kora_pubkey: std::env::var("KORA_PUBKEY").expect("KORA_PUBKEY must be set"),
            encryption_key: klave_core::crypto::parse_hex_key(
                &std::env::var("KLAVE_ENCRYPTION_KEY")
                    .expect("KLAVE_ENCRYPTION_KEY must be set (run `klave init`)"),
            )
            .expect("KLAVE_ENCRYPTION_KEY must be 64 hex chars"),
            jupiter_api_key: std::env::var("JUPITER_API_KEY")
                .ok()
                .filter(|k| !k.is_empty()),
            allowed_origins: std::env::var("KLAVE_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "*".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }
}
