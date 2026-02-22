#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub api_key: String,
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
        }
    }
}
