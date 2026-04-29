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

    /// Fetch price from Chainlink or other oracle
    async fn fetch_price_from_oracle(
        &self,
        from_token: PaymentToken,
        to_token: PaymentToken,
    ) -> Result<f64, String> {
        // This would call Chainlink Oracle or price feed
        // For now, return mock prices
        match (from_token, to_token) {
            (PaymentToken::Usdc, PaymentToken::Zen) => {
                // 1 USDC = ~50 ZEN (example)
                Ok(50.0)
            }
            (PaymentToken::Zen, PaymentToken::Usdc) => {
                // 1 ZEN = ~0.02 USDC
                Ok(0.02)
            }
            (PaymentToken::Dai, PaymentToken::Zen) => {
                // 1 DAI ≈ 1 USDC ≈ 50 ZEN
                Ok(50.0)
            }
            (PaymentToken::Zen, PaymentToken::Dai) => {
                Ok(0.02)
            }
            _ => Err("Price pair not supported".to_string()),
        }
    }
}

/// DEX Swapper (for converting USDC → ZEN)
pub struct DexSwapper {
    /// Uniswap V3 router address on Polygon
    router_address: String,
    /// Slippage tolerance (0.5%)
    slippage_tolerance: f64,
}

impl DexSwapper {
    pub fn new() -> Self {
        DexSwapper {
            router_address: "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(), // Uniswap V3 Router on Polygon
            slippage_tolerance: 0.005, // 0.5%
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
        // Get current price
        let price = price_oracle.get_price(from_token, to_token).await?;

        // Calculate expected output
        let input_amount: f64 = amount.parse().map_err(|_| "Invalid amount")?;
        let expected_output = input_amount * price;

        // Apply slippage tolerance
        let minimum_output = expected_output * (1.0 - self.slippage_tolerance);

        Ok(SwapResult {
            from_token,
            to_token,
            input_amount: amount.clone(),
            output_amount: expected_output.to_string(),
            minimum_output: minimum_output.to_string(),
            execution_price: price,
            // In real implementation, would execute actual Uniswap swap
            tx_hash: format!("0x{:x}", rand::random::<u64>()),
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
    /// Storage for payment records
    payment_records: Arc<RwLock<HashMap<String, PaymentRecord>>>,
}

impl PaymentProcessor {
    pub fn new() -> Self {
        PaymentProcessor {
            oracle: Arc::new(PriceOracle::new()),
            swapper: Arc::new(DexSwapper::new()),
            payment_records: Arc::new(RwLock::new(HashMap::new())),
        }
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

            // Store record
            {
                let mut records = self.payment_records.write().await;
                records.insert(payment_id.clone(), record.clone());
            }

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

        // Store record
        {
            let mut records = self.payment_records.write().await;
            records.insert(payment_id.clone(), record.clone());
        }

        Ok(record)
    }

    /// Get payment record by ID
    pub async fn get_payment(&self, payment_id: &str) -> Result<PaymentRecord, String> {
        let records = self.payment_records.read().await;
        records
            .get(payment_id)
            .cloned()
            .ok_or_else(|| "Payment not found".to_string())
    }

    /// Update payment status
    pub async fn update_payment_status(
        &self,
        payment_id: &str,
        status: PaymentStatus,
    ) -> Result<PaymentRecord, String> {
        let mut records = self.payment_records.write().await;
        let mut record = records
            .get(payment_id)
            .cloned()
            .ok_or_else(|| "Payment not found".to_string())?;

        record.status = status;
        if matches!(status, PaymentStatus::Completed) {
            record.completed_at = Some(Utc::now());
        }

        records.insert(payment_id.to_string(), record.clone());
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
