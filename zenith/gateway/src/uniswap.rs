/// Uniswap V3 Integration for token swaps on Polygon
/// Executes swaps via the Uniswap V3 SwapRouter contract
use serde::{Deserialize, Serialize};
use crate::payment::PaymentToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    /// Input amount in token's native units
    pub input_amount: String,
    /// Expected output amount
    pub output_amount: String,
    /// Minimum output with slippage tolerance applied
    pub minimum_output: String,
    /// Execution price
    pub execution_price: f64,
    /// Slippage tolerance percentage
    pub slippage_tolerance: f64,
}

pub struct UniswapV3Router {
    /// Polygon Uniswap V3 SwapRouter address
    router_address: String,
    /// RPC endpoint for Polygon
    rpc_endpoint: String,
    /// Slippage tolerance (default 0.5%)
    slippage_tolerance: f64,
}

impl UniswapV3Router {
    pub fn new(rpc_endpoint: &str) -> Self {
        UniswapV3Router {
            // Uniswap V3 SwapRouter on Polygon
            router_address: "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(),
            rpc_endpoint: rpc_endpoint.to_string(),
            slippage_tolerance: 0.005, // 0.5%
        }
    }

    /// Get a quote for a swap
    pub async fn get_swap_quote(
        &self,
        from_token: PaymentToken,
        to_token: PaymentToken,
        amount: String,
        execution_price: f64,
    ) -> Result<SwapQuote, String> {
        // Parse the input amount
        let input_amount_f64: f64 = amount.parse()
            .map_err(|_| "Invalid input amount".to_string())?;

        // Calculate expected output based on execution price
        let output_amount = input_amount_f64 * execution_price;

        // Apply slippage tolerance
        let minimum_output = output_amount * (1.0 - self.slippage_tolerance);

        Ok(SwapQuote {
            input_amount: amount,
            output_amount: output_amount.to_string(),
            minimum_output: minimum_output.to_string(),
            execution_price,
            slippage_tolerance: self.slippage_tolerance,
        })
    }

    /// Execute a swap on Uniswap V3
    /// In production, this would call the actual Uniswap router contract
    pub async fn execute_swap(
        &self,
        from_token: PaymentToken,
        to_token: PaymentToken,
        quote: &SwapQuote,
        user_address: &str,
    ) -> Result<SwapResult, String> {
        // Validate inputs
        if quote.input_amount.parse::<f64>().unwrap_or(0.0) <= 0.0 {
            return Err("Invalid input amount".to_string());
        }

        // In production, would construct and send the actual transaction:
        // 1. Create ExactInputSingle params
        // 2. Encode function call
        // 3. Sign transaction with user wallet
        // 4. Send to SwapRouter contract
        // 5. Wait for confirmation
        // 6. Return transaction hash

        // For now, simulate successful swap
        let tx_hash = format!("0x{:x}", rand::random::<u64>());

        Ok(SwapResult {
            from_token,
            to_token,
            input_amount: quote.input_amount.clone(),
            output_amount: quote.output_amount.clone(),
            execution_price: quote.execution_price,
            tx_hash,
            user_address: user_address.to_string(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResult {
    pub from_token: PaymentToken,
    pub to_token: PaymentToken,
    pub input_amount: String,
    pub output_amount: String,
    pub execution_price: f64,
    pub tx_hash: String,
    pub user_address: String,
}

/// Supported token pairs for swaps
pub fn get_pool_fee(from_token: PaymentToken, to_token: PaymentToken) -> Option<u32> {
    // Common Uniswap V3 pool fees (in basis points, 1 bp = 0.01%)
    match (from_token, to_token) {
        // Stablecoin pairs use 1 bp (0.01%) fee
        (PaymentToken::Usdc, PaymentToken::Dai) => Some(100),
        (PaymentToken::Dai, PaymentToken::Usdc) => Some(100),
        // ZEN pairs use 30 bp (0.3%) fee (typical for lower liquidity)
        (PaymentToken::Usdc, PaymentToken::Zen) => Some(3000),
        (PaymentToken::Zen, PaymentToken::Usdc) => Some(3000),
        (PaymentToken::Dai, PaymentToken::Zen) => Some(3000),
        (PaymentToken::Zen, PaymentToken::Dai) => Some(3000),
        // ZAR not yet supported on Uniswap V3
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_swap_quote() {
        let router = UniswapV3Router::new("https://polygon-rpc.com");
        let quote = router
            .get_swap_quote(
                PaymentToken::Usdc,
                PaymentToken::Zen,
                "100".to_string(),
                50.0,
            )
            .await
            .unwrap();

        assert_eq!(quote.input_amount, "100");
        assert!(quote.output_amount.parse::<f64>().unwrap() > 0.0);
        assert_eq!(quote.execution_price, 50.0);
    }

    #[tokio::test]
    async fn test_execute_swap() {
        let router = UniswapV3Router::new("https://polygon-rpc.com");
        let quote = SwapQuote {
            input_amount: "100".to_string(),
            output_amount: "5000".to_string(),
            minimum_output: "4975".to_string(),
            execution_price: 50.0,
            slippage_tolerance: 0.005,
        };

        let result = router
            .execute_swap(
                PaymentToken::Usdc,
                PaymentToken::Zen,
                &quote,
                "0x123",
            )
            .await
            .unwrap();

        assert_eq!(result.input_amount, "100");
        assert_eq!(result.output_amount, "5000");
        assert!(!result.tx_hash.is_empty());
    }

    #[test]
    fn test_pool_fees() {
        assert_eq!(get_pool_fee(PaymentToken::Usdc, PaymentToken::Dai), Some(100));
        assert_eq!(get_pool_fee(PaymentToken::Usdc, PaymentToken::Zen), Some(3000));
        assert_eq!(get_pool_fee(PaymentToken::Dai, PaymentToken::Zar), None);
    }
}
