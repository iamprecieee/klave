use std::sync::Arc;

use tokio::sync::RwLock;

use crate::error::{KlaveError, Result};

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const JUPITER_PRICE_URL: &str = "https://api.jup.ag/price/v3";
const CACHE_TTL_SECS: i64 = 60;

struct PriceCache {
    sol_usd: f64,
    fetched_at: i64,
}

// #[derive(Clone)]
pub struct PriceFeed {
    client: reqwest::Client,
    api_key: Option<String>,
    cache: Arc<RwLock<PriceCache>>,
}

impl PriceFeed {
    pub fn new(jupiter_api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: jupiter_api_key,
            cache: Arc::new(RwLock::new(PriceCache {
                sol_usd: 0.0,
                fetched_at: 0,
            })),
        }
    }

    /// Convert lamports to USD using the cached SOL/USD price.
    /// Returns 0.0 if the price feed is unavailable (effectively disabling USD limits).
    pub async fn lamports_to_usd(&self, lamports: u64) -> f64 {
        let price = self.get_sol_price().await;
        (lamports as f64 / 1e9) * price
    }

    async fn get_sol_price(&self) -> f64 {
        let now = chrono::Utc::now().timestamp();

        {
            let cache = self.cache.read().await;
            if now - cache.fetched_at < CACHE_TTL_SECS && cache.sol_usd > 0.0 {
                return cache.sol_usd;
            }
        }

        match self.fetch_sol_price().await {
            Ok(price) => {
                let mut cache = self.cache.write().await;
                cache.sol_usd = price;
                cache.fetched_at = now;
                price
            }
            Err(e) => {
                tracing::warn!("Failed to fetch SOL/USD price: {}", e);
                self.cache.read().await.sol_usd
            }
        }
    }

    async fn fetch_sol_price(&self) -> Result<f64> {
        let mut req = self
            .client
            .get(JUPITER_PRICE_URL)
            .query(&[("ids", SOL_MINT)]);

        if let Some(ref key) = self.api_key {
            req = req.header("x-api-key", key);
        }

        let resp: serde_json::Value = req
            .send()
            .await
            .map_err(|e| KlaveError::Internal(format!("price fetch: {}", e)))?
            .json()
            .await
            .map_err(|e| KlaveError::Internal(format!("price parse: {}", e)))?;

        resp[SOL_MINT]["usdPrice"]
            .as_f64()
            .ok_or_else(|| KlaveError::Internal("SOL usdPrice not in response".into()))
    }
}
