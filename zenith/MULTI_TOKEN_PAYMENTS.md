# Zenith Multi-Token Payment System

**Status:** Phase 1 Complete - USDC, DAI, $ZEN Support  
**Date:** April 29, 2026  
**Version:** 1.0.0

---

## Overview

Zenith now supports **multiple payment tokens** alongside the native $ZEN token. This enables:

- ✅ **Enterprise adoption** - Pay in stable USD (USDC/DAI)
- ✅ **Community incentives** - Earn and pay in $ZEN
- ✅ **Flexibility** - Choose payment token per transaction
- ✅ **Transparent pricing** - See costs in your preferred currency
- ✅ **Future-ready** - Easy to add ZAR stablecoins and other tokens

---

## Supported Tokens

### Phase 1 (Now)

| Token | Network | Address | Use Case |
|-------|---------|---------|----------|
| **$ZEN** | Polygon | TBD | Native, governance, staking |
| **USDC** | Polygon | 0x2791bca1f2de... | Enterprise payments, stable pricing |
| **DAI** | Polygon | 0x8f3cf7ad23cd... | Decentralized stable payments |

### Phase 2 (3-6 months)

- South African Rand stablecoin (ZAR)
- USDT (Tether)
- USDC on Ethereum mainnet
- Other emerging market stablecoins

---

## Architecture

### Payment Flow Diagram

```
User submits request:
    "I want to call a canister, pay in USDC"
              ↓
┌───────────────────────────────────┐
│  1. PRICE ESTIMATION              │
│  - Get USDC/USD price             │
│  - Display cost: "50 USDC = $50"  │
└───────────────────────────────────┘
              ↓
┌───────────────────────────────────┐
│  2. USER APPROVAL                 │
│  - User signs transaction          │
│  - Sends USDC from wallet          │
└───────────────────────────────────┘
              ↓
┌───────────────────────────────────┐
│  3. PAYMENT RECEIVED              │
│  - Verify USDC transaction         │
│  - Create payment record           │
└───────────────────────────────────┘
              ↓
┌───────────────────────────────────┐
│  4. AUTOMATED SWAP                │
│  - Query price oracle (Chainlink)  │
│  - Swap 50 USDC → ~2500 ZEN       │
│  - Via Uniswap V3                  │
└───────────────────────────────────┘
              ↓
┌───────────────────────────────────┐
│  5. SETTLEMENT                    │
│  - Pay provers in $ZEN             │
│  - Update ledger                   │
│  - Emit event                      │
└───────────────────────────────────┘
              ↓
User gets: Computation result + proof
System paid: Provers in $ZEN, treasury in $ZEN
```

### System Components

**1. Payment Module** (`gateway/src/payment.rs`)
- `PaymentToken` enum (ZEN, USDC, DAI, ZAR)
- `PaymentRequest` struct (user input)
- `PaymentRecord` struct (audit trail)
- `PriceOracle` (real-time price feeds)
- `DexSwapper` (Uniswap V3 integration)
- `PaymentProcessor` (orchestration)

**2. Payment API** (`gateway/src/payment_api.rs`)
- `POST /v1/payment/estimate` - Get price in different currencies
- `POST /v1/payment/process` - Submit and process payment
- `GET /v1/payment/:id` - Check payment status

**3. Integration Points**
- Database: Store payment records with audit trail
- Metrics: Track payment volumes and swap rates
- Audit Logger: Log all payments for compliance
- Circuit Breaker: Handle DEX failures gracefully

---

## API Endpoints

### 1. Estimate Price (No Auth)

**Request:**
```bash
POST /v1/payment/estimate
Content-Type: application/json

{
  "amount_usd": 50.0,
  "token": "USDC",
  "include_other_tokens": true
}
```

**Response:**
```json
{
  "amount_usd": 50.0,
  "amount_in_token": "50.00",
  "exchange_rate": 1.0,
  "token_symbol": "USDC",
  "price_breakdown": {
    "zen": "2500",
    "usdc": "50.00",
    "dai": "50.00"
  },
  "display_message": "Cost: $50.00 USD = 50.00 USDC = 2500 ZEN"
}
```

### 2. Process Payment (Requires Auth)

**Request:**
```bash
POST /v1/payment/process
Authorization: Bearer <jwt-token>
Content-Type: application/json

{
  "payment_token": "USDC",
  "amount": "50.00",
  "user_address": "0x123...",
  "computation_id": "call-456",
  "display_currency": "USD"
}
```

**Response:**
```json
{
  "payment_id": "pay-xyz789",
  "status": "pending",
  "payment_received": {
    "token": "USDC",
    "amount": "50.00",
    "tx_hash": "0xabc123..."
  },
  "settlement": {
    "token": "ZEN",
    "amount": "2500",
    "exchange_rate": 50.0,
    "payment_to_provers": "Provers will earn ~2500 ZEN for this computation"
  },
  "next_step": "Send transaction confirmation to POST /v1/payment/pay-xyz789/confirm"
}
```

### 3. Check Payment Status

**Request:**
```bash
GET /v1/payment/pay-xyz789
Authorization: Bearer <jwt-token>
```

**Response:**
```json
{
  "payment_id": "pay-xyz789",
  "status": "completed",
  "payment_token": "USDC",
  "amount_paid": "50.00",
  "settlement_token": "ZEN",
  "amount_settled": "2500",
  "exchange_rate": 50.0,
  "created_at": "2026-04-29T12:00:00Z",
  "completed_at": "2026-04-29T12:00:15Z"
}
```

---

## Integration Examples

### Example 1: User Wants to Pay in USDC

```bash
#!/bin/bash

# Step 1: Get price estimate
curl -X POST http://localhost:8000/v1/payment/estimate \
  -H "Content-Type: application/json" \
  -d '{
    "amount_usd": 50.0,
    "token": "USDC"
  }'

# Response: "Cost is 50 USDC"

# Step 2: User approves and sends USDC from their wallet
# (Off-chain, in Metamask or similar)

# Step 3: Submit payment request
TOKEN=$(zenith auth generate --role user --exp 3600)
curl -X POST http://localhost:8000/v1/payment/process \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "payment_token": "USDC",
    "amount": "50.00",
    "user_address": "0xUser...",
    "computation_id": "call-123",
    "display_currency": "USD"
  }'

# Response: payment_id = "pay-xyz789"

# Step 4: Check status
curl http://localhost:8000/v1/payment/pay-xyz789

# Response: status = "completed"
# Now user can call their canister!
```

### Example 2: User Wants to Pay in $ZEN

```bash
#!/bin/bash

# Same flow, but with ZEN
curl -X POST http://localhost:8000/v1/payment/process \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "payment_token": "ZEN",
    "amount": "2500",
    "user_address": "0xUser...",
    "computation_id": "call-456"
  }'

# No swap needed! 
# Provers get paid 2500 ZEN directly
# No conversion costs
```

### Example 3: CLI Integration (Proposed)

```bash
# User deploys a canister, system suggests payment method
$ zenith deploy --wasm model.wasm --payment-token USDC

# System shows:
# "Deployment will cost ~$50 USD (50 USDC or 2500 ZEN)"
# 
# Preferred method? [usdc/zen/dai] (default: usdc)

# User selects USDC
$ zenith deploy --wasm model.wasm --payment-token USDC
# 
# Please approve 50 USDC from your wallet...
# Waiting for payment... ✓
# Deploying...
# ✓ Canister deployed: canister-abc123
```

---

## Technical Implementation

### Database Schema

```sql
-- Payment records table
CREATE TABLE payments (
  id TEXT PRIMARY KEY,
  user_address TEXT NOT NULL,
  
  -- Payment received
  payment_token TEXT NOT NULL,  -- USDC, DAI, ZEN, etc.
  payment_amount DECIMAL NOT NULL,
  payment_tx_hash TEXT,
  
  -- Settlement (what provers get)
  settlement_token TEXT NOT NULL,  -- Always ZEN
  settlement_amount DECIMAL NOT NULL,
  
  -- Conversion tracking
  exchange_rate DECIMAL NOT NULL,
  
  -- Status
  status TEXT NOT NULL,  -- pending, processing, received, converted, completed, failed
  created_at TIMESTAMP NOT NULL,
  completed_at TIMESTAMP,
  
  -- Audit
  computation_id TEXT,
  notes TEXT
);

-- Price cache (for oracle)
CREATE TABLE price_cache (
  token_pair TEXT PRIMARY KEY,  -- "USDC_ZEN"
  price DECIMAL NOT NULL,
  cached_at TIMESTAMP NOT NULL
);
```

### Price Oracle Integration

```rust
// Chainlink integration (pseudocode)
impl PriceOracle {
    async fn fetch_price_from_chainlink(
        &self,
        from_token: PaymentToken,
        to_token: PaymentToken,
    ) -> Result<f64> {
        // Call Chainlink Oracle contract on Polygon
        let contract = ChainlinkContract::new(POLYGON_RPC);
        
        match (from_token, to_token) {
            (USDC, ZEN) => {
                // Fetch: USDC/USD * ZEN/USD^-1
                let usdc_usd = contract.get_price("USDC/USD").await?;
                let zen_usd = contract.get_price("ZEN/USD").await?;
                Ok(usdc_usd / zen_usd)
            }
            _ => { /* similar for other pairs */ }
        }
    }
}
```

### DEX Swap (Uniswap V3)

```rust
// Uniswap V3 integration (pseudocode)
impl DexSwapper {
    async fn swap(
        &self,
        from_token: PaymentToken,
        to_token: PaymentToken,
        amount: u128,
    ) -> Result<SwapReceipt> {
        // Build Uniswap V3 swap transaction
        let swap_data = SwapParameters {
            router: UNISWAP_V3_ROUTER,
            path: self.get_path(from_token, to_token),
            amount_in: amount,
            amount_out_min: self.calculate_min_output(amount),
            deadline: Utc::now() + Duration::minutes(10),
            recipient: ZENITH_TREASURY,
        };
        
        // Execute swap
        let tx = execute_uniswap_swap(swap_data).await?;
        Ok(SwapReceipt {
            tx_hash: tx.hash,
            input: amount,
            output: tx.output_amount,
        })
    }
}
```

---

## South African Rand (ZAR) Integration

### Phase 2 Timeline

```
Week 1-2: Identify ZAR stablecoin partner
Week 3-4: Negotiate integration
Week 5-6: Implement ZAR support
Week 7: Test and launch

Roadmap:
  1. Contact existing ZAR stablecoin projects
  2. Or: Create $ZENR (ZAR-pegged Zenith stablecoin)
  3. Add to payment system (easy, same as USDC)
  4. Market to South African enterprises
```

### ZAR Implementation (Identical to USDC)

```rust
// Just add to PaymentToken enum
pub enum PaymentToken {
    Zen,
    Usdc,
    Dai,
    Zar,  // ← New!
}

// That's it! Rest is automatic:
// - Price oracle: Get ZAR/USD rate
// - DEX swap: USDC → ZAR or vice versa  
// - Same API endpoints work
// - Users see "R500" instead of "$50"
```

---

## Security Considerations

### 1. Price Oracle Attacks

**Risk:** Attacker manipulates price to overpay or underpay

**Mitigation:**
- Use Chainlink oracle (decentralized, tamper-resistant)
- Cache prices for 1 minute (prevent rapid manipulation)
- Use TWAP (Time-Weighted Average Price) instead of spot price
- Circuit breaker on unusual price movements (>5% deviation)

### 2. DEX Slippage

**Risk:** Swap gets much less ZEN than expected due to market impact

**Mitigation:**
- Set 0.5% slippage tolerance (user sees this upfront)
- Use Uniswap V3 (concentrated liquidity, better rates)
- Batch swaps (pool with other users' swaps for better rate)
- Fall back to manual swap if DEX unavailable (circuit breaker)

### 3. Front-Running

**Risk:** Attacker sees payment, front-runs to steal value

**Mitigation:**
- Use private mempool (Flashbots Protect)
- Low amounts (payments are per-computation, usually <$100)
- Accept some slippage (0.5% is acceptable)

### 4. Wallet Approval Risks

**Risk:** User approves token spend that's too high

**Mitigation:**
- Only request approval for exact amount
- Show breakdown: "You're authorizing USDC transfer"
- Link to Etherscan for transparency

---

## Monitoring & Alerts

### Key Metrics to Track

```
1. Payment Volume
   - Daily: Total USD value processed
   - By token: % USDC vs DAI vs ZEN
   - Growth: MoM increase

2. Swap Performance
   - Average slippage: Should stay <0.5%
   - Swap success rate: Should be >99%
   - DEX fee: Track Uniswap fees

3. Exchange Rates
   - Current: USDC/ZEN, DAI/ZEN
   - Historical: Track rate volatility
   - Alert if deviation >2% from oracle

4. Payment Status
   - Pending → completed: Should be <60s
   - Failed payments: Monitor for issues
   - User retry rate: Indicates problems
```

### Alerting Rules

```
CRITICAL (PagerDuty):
  - Swap failure rate > 5%
  - DEX outage > 5 minutes
  - Price oracle stale > 10 minutes
  
MEDIUM (Slack):
  - Slippage > 1.0%
  - Payment processing > 120s
  - Daily volume > 50% below average
  
INFO (Metrics):
  - Daily volume summary
  - Token distribution
  - Average exchange rate
```

---

## Future Enhancements

### 1. Batch Swaps (Cost Optimization)

```
Instead of swapping each payment immediately:
  - Collect 10 USDC payments in a bucket
  - Swap all 10 at once: 500 USDC → 25,000 ZEN
  - Pro: Better Uniswap rates (less slippage)
  - Con: 1-2 minute delay for user
  
Trade-off: Save 0.2% on fees (small)
```

### 2. Liquidity Pools

```
Partner with DEX to create native $ZEN pool:
  - USDC/ZEN pool on Uniswap V3
  - Zenith provides liquidity
  - Pro: Better rates for our swaps
  - Con: Capital allocation required
  
Expected savings: 0.3% on swap costs
```

### 3. Direct Stablecoin Withdrawals

```
For provers:
  - Earn ZEN
  - Option to withdraw as USDC/DAI/ZAR
  - Automatic swap at settlement
  
Pro: Risk-averse provers stay engaged
```

### 4. Multi-Sig Treasury

```
For security:
  - Swapped USDC held in 2-of-3 multisig
  - Keys split between admins
  - Prevents rug pull / fund theft
```

---

## Testing

### Unit Tests

```rust
#[test]
fn test_usdc_payment_flow() {
    let processor = PaymentProcessor::new();
    
    // User submits USDC payment
    let request = PaymentRequest {
        token: PaymentToken::Usdc,
        amount: "50".to_string(),
        user_address: "0x123".to_string(),
        display_currency: Some("USD".to_string()),
    };
    
    let record = processor.process_payment(request).await?;
    
    // Verify conversion
    assert_eq!(record.payment_token, PaymentToken::Usdc);
    assert_eq!(record.settlement_token, PaymentToken::Zen);
    assert_eq!(record.exchange_rate, 50.0);  // 1 USDC = 50 ZEN
    assert_eq!(record.settlement_amount, "2500");  // 50 * 50
}
```

### Integration Tests

```bash
# Test with actual Polygon testnet
cargo test --test integration_tests -- --nocapture --test-threads=1

# Specific test:
# 1. Deploy to Polygon Mumbai
# 2. Send actual USDC (testnet)
# 3. Verify swap happens
# 4. Confirm ZEN received by prover
```

---

## Rollout Plan

### Phase 1 (Now - May 2026)
- ✅ Implement USDC/DAI support
- ✅ Deploy on Polygon testnet
- ✅ Test with users
- ✅ Document API

### Phase 2 (June 2026)
- Launch on Polygon mainnet
- Partner with ZAR stablecoin project OR create $ZENR
- Add South African marketing

### Phase 3 (July+ 2026)
- Add other tokens (USDT, cUSD, etc.)
- Cross-chain support (Ethereum, other L2s)
- Advanced features (batch swaps, liquidity pools)

---

## Support & Troubleshooting

### Common Issues

**Q: Why is my payment showing "pending" for 2 minutes?**
A: System is waiting for blockchain confirmation and DEX swap. This is normal. Check `/v1/payment/:id` for status.

**Q: My swap got less ZEN than expected**
A: This is slippage (0.5% tolerance). If large, market may have moved. Try again in 1 minute.

**Q: Can I use other stablecoins?**
A: Not yet. Phase 2 will add ZAR, USDT, and others. Contact support for your preferred token.

**Q: Is my payment secure?**
A: Yes. Payments use Polygon (lower fees, faster confirmation) and Chainlink oracle (decentralized, secure).

---

## Contact & Questions

- Technical: support@zenith.io
- Partnerships: partnerships@zenith.io
- ZAR Integration: zar@zenith.io

---

**Document Version:** 1.0  
**Last Updated:** April 29, 2026  
**Maintained By:** Zenith Engineering Team
