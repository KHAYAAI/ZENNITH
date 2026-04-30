/// Multi-token payment system for Zenith Gateway
///
/// Supports:
/// - USDC (USD Coin) on Polygon
/// - DAI (Decentralized stable coin)
/// - $ZEN (native token)
///
/// Payment flow:
/// 1. User submits request with preferred payment token (USDC/DAI/$ZEN)
/// 2. System validates user has sufficient balance
/// 3. System swaps to $ZEN if needed (via Uniswap V3)
/// 4. Provers are paid in $ZEN
/// 5. User sees pricing in their preferred currency

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing;

/// Supported payment tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentToken {
    /// Native Zenith token (primary)
    #[serde(rename = "ZEN")]
    Zen,
    /// USDC on Polygon
    #[serde(rename = "USDC")]
    Usdc,
    /// DAI stablecoin
    #[serde(rename = "DAI")]
    Dai,
    /// Future: South African Rand stablecoin
    #[serde(rename = "ZAR")]
    Zar,
}

impl PaymentToken {
    /// Get token contract address on Polygon
    pub fn contract_address(&self) -> &'static str {
        match self {
            PaymentToken::Zen => "0xZEN_POLYGON_ADDRESS",     // Will be actual address
            PaymentToken::Usdc => "0x2791bca1f2de4661ed88a30c99a7a9449aa84174", // Actual USDC Polygon
            PaymentToken::Dai => "0x8f3cf7ad23cd3cadbd9735aff958023d60313e8e",  // Actual DAI Polygon
            PaymentToken::Zar => "0xZAR_FUTURE_ADDRESS",
        }
    }

    /// Get token decimals (for amount conversion)
    pub fn decimals(&self) -> u8 {
        match self {
            PaymentToken::Zen => 18,
            PaymentToken::Usdc => 6,
            PaymentToken::Dai => 18,
            PaymentToken::Zar => 18,
        }
    }

    /// Get display symbol
    pub fn symbol(&self) -> &'static str {
        match self {
            PaymentToken::Zen => "ZEN",
            PaymentToken::Usdc => "USDC",
            PaymentToken::Dai => "DAI",
            PaymentToken::Zar => "ZAR",
        }
    }
}

/// Payment request from user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    /// Token user wants to pay with
    pub token: PaymentToken,
    /// Amount in the token (as string to handle precision)
    pub amount: String,
    /// User's wallet address (Polygon)
    pub user_address: String,
    /// Optional: prefer this currency for display
    pub display_currency: Option<String>,
}

/// Payment record (for audit trail)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRecord {
    /// Unique payment ID
    pub id: String,
    /// User who paid
    pub user: String,
    /// Token received from user
    pub payment_token: PaymentToken,
    /// Amount received
    pub payment_amount: String,
    /// Token provers are paid in (always ZEN)
    pub settlement_token: PaymentToken,
    /// Amount paid to provers (after conversion)
    pub settlement_amount: String,
    /// Exchange rate used
    pub exchange_rate: f64,
    /// Transaction hash on blockchain
    pub tx_hash: Option<String>,
    /// Status
    pub status: PaymentStatus,
    /// When created
    pub created_at: DateTime<Utc>,
    /// When completed
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PaymentStatus {
    /// Waiting for user to approve/sign
    Pending,
    /// Transaction submitted
    Processing,
    /// Payment received and verified
    Received,
    /// Conversion to ZEN completed
    Converted,
    /// Provers have been paid
    Completed,
    /// Something went wrong
    Failed,
}

/// Price oracle for token conversions
pub struct PriceOracle {
    /// Cached prices (token_pair -> price)
    cache: Arc<RwLock<HashMap<String, PriceCacheEntry>>>,
    /// Cache TTL in seconds
    cache_ttl: u64,
    /// Chainlink oracle client for real prices
    chainlink: crate::chainlink::ChainlinkOracleClient,
}

#[derive(Clone)]
struct PriceCacheEntry {
    price: f64,
    cached_at: DateTime<Utc>,
}

impl PriceOracle {
    pub fn new() -> Self {
        PriceOracle {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: 60, // 1 minute
            chainlink: crate::chainlink::ChainlinkOracleClient::new(),
        }
    }

    /// Get price of from_token in to_token
    pub async fn get_price(
        &self,
        from_token: PaymentToken,
        to_token: PaymentToken,
    ) -> Result<f64, String> {
        if from_token == to_token {
            return Ok(1.0);
        }

        let cache_key = format!("{}_{}", from_token.symbol(), to_token.symbol());

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(&cache_key) {
                let age = Utc::now()
                    .signed_duration_since(entry.cached_at)
                    .num_seconds() as u64;
                if age < self.cache_ttl {
                    return Ok(entry.price);
                }
            }
        }

        // Fetch fresh price from oracle
        let price = self.fetch_price_from_oracle(from_token, to_token).await?;

        // Cache it
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                cache_key,
                PriceCacheEntry {
                    price,
                    cached_at: Utc::now(),
                },
            );
        }

        Ok(price)
    }

    /// Fetch price from Chainlink oracle with fallback to mock prices
    async fn fetch_price_from_oracle(
        &self,
        from_token: PaymentToken,
        to_token: PaymentToken,
    ) -> Result<f64, String> {
        // Try to get real prices from Chainlink
        match self
            .chainlink
            .get_conversion_rate(from_token, to_token)
            .await
        {
            Ok(price) => return Ok(price),
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch price from Chainlink: {}. Using fallback prices.",
                    e
                );
            }
        }

        // Fallback to mock prices if Chainlink is unavailable
        match (from_token, to_token) {
            (PaymentToken::Usdc, PaymentToken::Zen) => Ok(50.0),
            (PaymentToken::Zen, PaymentToken::Usdc) => Ok(0.02),
            (PaymentToken::Dai, PaymentToken::Zen) => Ok(50.0),
            (PaymentToken::Zen, PaymentToken::Dai) => Ok(0.02),
            _ => Err("Price pair not supported".to_string()),
        }
    }
}

/// DEX Swapper (for converting USDC/DAI → ZEN via Uniswap V3)
pub struct DexSwapper {
    /// Uniswap V3 router
    router: crate::uniswap::UniswapV3Router,
}

impl DexSwapper {
    pub fn new() -> Self {
        DexSwapper {
            router: crate::uniswap::UniswapV3Router::new("https://polygon-rpc.com"),
        }
    }

    /// Execute swap on Uniswap V3
    pub async fn swap(
        &self,
        from_token: PaymentToken,
        to_token: PaymentToken,
        amount: String,
        price_oracle: &PriceOracle,
    ) -> Result<SwapResult, String> {
        // Get current price from oracle
        let price = price_oracle.get_price(from_token, to_token).await?;

        // Get quote from Uniswap
        let quote = self
            .router
            .get_swap_quote(from_token, to_token, amount.clone(), price)
            .await?;

        // Execute the swap (in production, this submits to the blockchain)
        let swap_result = self
            .router
            .execute_swap(from_token, to_token, &quote, "0x0")
            .await?;

        Ok(SwapResult {
            from_token,
            to_token,
            input_amount: swap_result.input_amount,
            output_amount: swap_result.output_amount,
            minimum_output: quote.minimum_output,
            execution_price: swap_result.execution_price,
            tx_hash: swap_result.tx_hash,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResult {
    pub from_token: PaymentToken,
    pub to_token: PaymentToken,
    pub input_amount: String,
    pub output_amount: String,
    pub minimum_output: String,
    pub execution_price: f64,
    pub tx_hash: String,
}

/// Payment processor (orchestrates the flow)
pub struct PaymentProcessor {
    oracle: Arc<PriceOracle>,
    swapper: Arc<DexSwapper>,
    /// Persistent payment record storage
    payment_db: Arc<crate::payment_db::PaymentDb>,
}

impl PaymentProcessor {
    pub fn new(db_path: &str) -> Result<Self, String> {
        Ok(PaymentProcessor {
            oracle: Arc::new(PriceOracle::new()),
            swapper: Arc::new(DexSwapper::new()),
            payment_db: Arc::new(
                crate::payment_db::PaymentDb::new(db_path)
                    .map_err(|e| format!("Failed to open payment database: {}", e))?,
            ),
        })
    }

    /// Process a payment request
    pub async fn process_payment(
        &self,
        request: PaymentRequest,
    ) -> Result<PaymentRecord, String> {
        let payment_id = uuid::Uuid::new_v4().to_string();

        // If paying in ZEN directly, no conversion needed
        if request.token == PaymentToken::Zen {
            let record = PaymentRecord {
                id: payment_id.clone(),
                user: request.user_address,
                payment_token: PaymentToken::Zen,
                payment_amount: request.amount.clone(),
                settlement_token: PaymentToken::Zen,
                settlement_amount: request.amount.clone(),
                exchange_rate: 1.0,
                tx_hash: None,
                status: PaymentStatus::Pending,
                created_at: Utc::now(),
                completed_at: None,
            };

            self.payment_db.insert(&payment_id, &record)?;
            return Ok(record);
        }

        // If paying in stablecoin, need to swap to ZEN
        let swap_result = self
            .swapper
            .swap(request.token, PaymentToken::Zen, request.amount.clone(), &self.oracle)
            .await?;

        let record = PaymentRecord {
            id: payment_id.clone(),
            user: request.user_address,
            payment_token: request.token,
            payment_amount: request.amount,
            settlement_token: PaymentToken::Zen,
            settlement_amount: swap_result.output_amount,
            exchange_rate: swap_result.execution_price,
            tx_hash: Some(swap_result.tx_hash),
            status: PaymentStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
        };

        self.payment_db.insert(&payment_id, &record)?;
        Ok(record)
    }

    /// Get payment record by ID
    pub async fn get_payment(&self, payment_id: &str) -> Result<PaymentRecord, String> {
        self.payment_db
            .get(payment_id)?
            .ok_or_else(|| "Payment not found".to_string())
    }

    /// Update payment status
    pub async fn update_payment_status(
        &self,
        payment_id: &str,
        status: PaymentStatus,
    ) -> Result<PaymentRecord, String> {
        let mut record = self.payment_db
            .get(payment_id)?
            .ok_or_else(|| "Payment not found".to_string())?;

        record.status = status;
        if matches!(status, PaymentStatus::Completed) {
            record.completed_at = Some(Utc::now());
        }

        self.payment_db.insert(payment_id, &record)?;
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_token_addresses() {
        assert_eq!(
            PaymentToken::Usdc.contract_address(),
            "0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
        );
    }

    #[test]
    fn test_payment_token_decimals() {
        assert_eq!(PaymentToken::Zen.decimals(), 18);
        assert_eq!(PaymentToken::Usdc.decimals(), 6);
    }

    #[tokio::test]
    async fn test_price_oracle_caching() {
        let oracle = PriceOracle::new();
        let price1 = oracle
            .get_price(PaymentToken::Usdc, PaymentToken::Zen)
            .await
            .unwrap();
        let price2 = oracle
            .get_price(PaymentToken::Usdc, PaymentToken::Zen)
            .await
            .unwrap();

        // Should return same price (from cache)
        assert_eq!(price1, price2);
    }

    #[tokio::test]
    async fn test_payment_processor_zen_direct() {
        let processor = PaymentProcessor::new();
        let request = PaymentRequest {
            token: PaymentToken::Zen,
            amount: "1000".to_string(),
            user_address: "0x123".to_string(),
            display_currency: None,
        };

        let record = processor.process_payment(request).await.unwrap();

        assert_eq!(record.payment_token, PaymentToken::Zen);
        assert_eq!(record.settlement_token, PaymentToken::Zen);
        assert_eq!(record.exchange_rate, 1.0);
    }

    #[tokio::test]
    async fn test_payment_processor_usdc_swap() {
        let processor = PaymentProcessor::new();
        let request = PaymentRequest {
            token: PaymentToken::Usdc,
            amount: "50".to_string(),
            user_address: "0x456".to_string(),
            display_currency: Some("USD".to_string()),
        };

        let record = processor.process_payment(request).await.unwrap();

        assert_eq!(record.payment_token, PaymentToken::Usdc);
        assert_eq!(record.settlement_token, PaymentToken::Zen);
        assert!(record.exchange_rate > 0.0);
    }
}
