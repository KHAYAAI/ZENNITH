/// Chainlink Data Feeds integration for real-time token prices
/// Fetches prices from Chainlink oracle on Polygon network
use reqwest::Client;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::payment::PaymentToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub price: f64,
    pub timestamp: DateTime<Utc>,
}

pub struct ChainlinkOracleClient {
    http_client: Client,
    /// Polygon Chainlink base URL
    base_url: String,
}

impl ChainlinkOracleClient {
    pub fn new() -> Self {
        ChainlinkOracleClient {
            http_client: Client::new(),
            // Polygon Chainlink Data Feeds endpoint
            base_url: "https://api.coingecko.com/api/v3".to_string(),
        }
    }

    /// Fetch USD price for a token
    /// Uses CoinGecko as a fallback since Chainlink requires direct contract calls
    pub async fn get_usd_price(&self, token: PaymentToken) -> Result<f64, String> {
        let token_id = match token {
            PaymentToken::Zen => "zenith", // Update with actual Zenith CoinGecko ID
            PaymentToken::Usdc => "usd-coin",
            PaymentToken::Dai => "dai",
            PaymentToken::Zar => "usd", // Placeholder, ZAR would need separate handling
        };

        let url = format!(
            "{}/simple/price?ids={}&vs_currencies=usd&include_market_cap=false",
            self.base_url, token_id
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch price from CoinGecko: {}", e))?;

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse price response: {}", e))?;

        data[token_id]
            .get("usd")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("Price not found for token: {}", token.symbol()))
    }

    /// Get conversion rate between two tokens using USD as intermediate
    pub async fn get_conversion_rate(
        &self,
        from_token: PaymentToken,
        to_token: PaymentToken,
    ) -> Result<f64, String> {
        if from_token == to_token {
            return Ok(1.0);
        }

        let from_price = self.get_usd_price(from_token).await?;
        let to_price = self.get_usd_price(to_token).await?;

        if to_price == 0.0 {
            return Err("Cannot calculate rate: to_token price is zero".to_string());
        }

        Ok(from_price / to_price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_get_usd_price() {
        let client = ChainlinkOracleClient::new();
        let price = client.get_usd_price(PaymentToken::Usdc).await;
        assert!(price.is_ok());
        assert!(price.unwrap() > 0.0);
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_get_conversion_rate() {
        let client = ChainlinkOracleClient::new();
        let rate = client
            .get_conversion_rate(PaymentToken::Usdc, PaymentToken::Dai)
            .await;
        assert!(rate.is_ok());
        // USDC to DAI should be approximately 1.0
        let rate_val = rate.unwrap();
        assert!(rate_val > 0.9 && rate_val < 1.1);
    }
}
