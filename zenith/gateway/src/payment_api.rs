/// REST API endpoints for multi-token payments
///
/// Endpoints:
/// - POST /v1/payment/estimate - Get price estimate in different currencies
/// - POST /v1/payment/process - Submit payment
/// - GET /v1/payment/:id - Check payment status
/// - GET /v1/payment/:id/confirm - Confirm payment completed

use axum::{
    extract::Path,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::payment::{PaymentRequest, PaymentToken};

/// Request to estimate cost in different currencies
#[derive(Debug, Serialize, Deserialize)]
pub struct PriceEstimateRequest {
    /// Base amount in USD
    pub amount_usd: f64,
    /// Token to estimate price in
    pub token: PaymentToken,
    /// Optional: get price in multiple currencies
    pub include_other_tokens: Option<bool>,
}

/// Response with price in requested currency
#[derive(Debug, Serialize, Deserialize)]
pub struct PriceEstimateResponse {
    /// Original amount in USD
    pub amount_usd: f64,
    /// Amount in requested token
    pub amount_in_token: String,
    /// Current exchange rate
    pub exchange_rate: f64,
    /// Token symbol
    pub token_symbol: String,
    /// Breakdown in other tokens (if requested)
    pub price_breakdown: Option<PriceBreakdown>,
    /// Display message
    pub display_message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PriceBreakdown {
    pub zen: String,
    pub usdc: String,
    pub dai: String,
}

/// Request to process a payment
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessPaymentRequest {
    /// What token user wants to pay with
    pub payment_token: PaymentToken,
    /// Amount in that token
    pub amount: String,
    /// User's wallet address
    pub user_address: String,
    /// What they're paying for (canister call ID)
    pub computation_id: String,
    /// Optional: preferred display currency
    pub display_currency: Option<String>,
}

/// Response after payment is processed
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessPaymentResponse {
    /// Payment ID (for tracking)
    pub payment_id: String,
    /// Status
    pub status: String,
    /// What we received
    pub payment_received: PaymentInfo,
    /// What provers will be paid in
    pub settlement: SettlementInfo,
    /// Where to send confirmation (tx hash, etc.)
    pub next_step: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentInfo {
    pub token: String,
    pub amount: String,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettlementInfo {
    pub token: String,
    pub amount: String,
    pub exchange_rate: f64,
    pub payment_to_provers: String,
}

/// Get payment status
#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentStatusResponse {
    pub payment_id: String,
    pub status: String,
    pub payment_token: String,
    pub amount_paid: String,
    pub settlement_token: String,
    pub amount_settled: String,
    pub exchange_rate: f64,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Estimate price in requested currency
pub async fn estimate_price(
    Json(request): Json<PriceEstimateRequest>,
) -> Result<Json<PriceEstimateResponse>, (StatusCode, String)> {
    // Get oracle from processor (would need to expose it)
    // For now, using hardcoded exchange rates

    let amount_in_token = match request.token {
        PaymentToken::Zen => {
            // 1 USD = ~50 ZEN
            (request.amount_usd * 50.0).to_string()
        }
        PaymentToken::Usdc => {
            // 1 USD = 1 USDC
            request.amount_usd.to_string()
        }
        PaymentToken::Dai => {
            // 1 USD ≈ 1 DAI
            request.amount_usd.to_string()
        }
        PaymentToken::Zar => {
            // 1 USD ≈ 18 ZAR (example rate)
            (request.amount_usd * 18.0).to_string()
        }
    };

    let rate = match request.token {
        PaymentToken::Zen => 50.0,
        PaymentToken::Usdc => 1.0,
        PaymentToken::Dai => 1.0,
        PaymentToken::Zar => 18.0,
    };

    let breakdown = if request.include_other_tokens.unwrap_or(false) {
        Some(PriceBreakdown {
            zen: (request.amount_usd * 50.0).to_string(),
            usdc: request.amount_usd.to_string(),
            dai: request.amount_usd.to_string(),
        })
    } else {
        None
    };

    let display_message = format!(
        "Cost: ${:.2} USD = {} {} = {} ZEN",
        request.amount_usd,
        amount_in_token,
        request.token.symbol(),
        (request.amount_usd * 50.0) as u64
    );

    Ok(Json(PriceEstimateResponse {
        amount_usd: request.amount_usd,
        amount_in_token,
        exchange_rate: rate,
        token_symbol: request.token.symbol().to_string(),
        price_breakdown: breakdown,
        display_message,
    }))
}

/// Process a payment
pub async fn process_payment(
    Json(request): Json<ProcessPaymentRequest>,
) -> Result<(StatusCode, Json<ProcessPaymentResponse>), (StatusCode, String)> {
    // Create a temporary processor for this request
    let processor = Arc::new(crate::payment::PaymentProcessor::new());

    let payment_request = PaymentRequest {
        token: request.payment_token,
        amount: request.amount.clone(),
        user_address: request.user_address.clone(),
        display_currency: request.display_currency,
    };

    let payment_record = processor
        .process_payment(payment_request)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let payment_id_str = payment_record.id.clone();
    let settlement_amount_str = payment_record.settlement_amount.clone();

    let response = ProcessPaymentResponse {
        payment_id: payment_id_str.clone(),
        status: "pending".to_string(),
        payment_received: PaymentInfo {
            token: request.payment_token.symbol().to_string(),
            amount: request.amount,
            tx_hash: payment_record.tx_hash.clone(),
        },
        settlement: SettlementInfo {
            token: "ZEN".to_string(),
            amount: settlement_amount_str.clone(),
            exchange_rate: payment_record.exchange_rate,
            payment_to_provers: format!(
                "{} ZEN will be distributed to provers",
                settlement_amount_str
            ),
        },
        next_step: format!(
            "Send transaction confirmation to POST /v1/payment/{}/confirm",
            payment_id_str
        ),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Get payment status
pub async fn get_payment_status(
    Path(payment_id): Path<String>,
) -> Result<Json<PaymentStatusResponse>, (StatusCode, String)> {
    // Create a temporary processor for this request
    // In production, would retrieve from database instead
    let processor = Arc::new(crate::payment::PaymentProcessor::new());

    let payment = processor
        .get_payment(&payment_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e))?;

    Ok(Json(PaymentStatusResponse {
        payment_id: payment.id,
        status: format!("{:?}", payment.status),
        payment_token: payment.payment_token.symbol().to_string(),
        amount_paid: payment.payment_amount,
        settlement_token: "ZEN".to_string(),
        amount_settled: payment.settlement_amount,
        exchange_rate: payment.exchange_rate,
        created_at: payment.created_at.to_rfc3339(),
        completed_at: payment.completed_at.map(|t| t.to_rfc3339()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_response() {
        let response = PriceEstimateResponse {
            amount_usd: 50.0,
            amount_in_token: "50".to_string(),
            exchange_rate: 1.0,
            token_symbol: "USDC".to_string(),
            price_breakdown: None,
            display_message: "Cost: $50.00 USD".to_string(),
        };

        assert_eq!(response.amount_usd, 50.0);
        assert_eq!(response.exchange_rate, 1.0);
    }
}
