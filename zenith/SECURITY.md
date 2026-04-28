# Zenith Security Guidelines

## Security Hardening (Phase 1 Complete)

This document outlines security measures implemented and those recommended for production.

## Implemented Hardening

### 1. ✅ Persistent State (sled Database)

**Why:** In-memory state was lost on restart, making the system unreliable.

**Solution:** Integrated `sled` embedded database for gateway state:

```rust
// In gateway/src/db.rs
pub struct Database {
    db: sled::Db,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        Ok(Database { db })
    }
    
    pub fn insert_canister(&self, canister: &DeployRequest) -> Result<(), String>
    pub fn list_canisters(&self) -> Result<Vec<DeployRequest>, String>
    // ... other operations
}
```

**Deployment:**
```bash
cargo run --release -p zenith-gateway -- \
  --data-dir /data/zenith-gateway
```

**Backup:**
```bash
# Daily backup
0 2 * * * cp -r /data/zenith-gateway /backups/zenith-gateway-$(date +\%s)

# Restore
cp -r /backups/zenith-gateway-<timestamp> /data/zenith-gateway
```

### 2. ✅ JWT Authentication

**Why:** Gateway endpoints were open to anyone. Write operations need authorization.

**Solution:** JWT-based authentication with role-based access control:

```rust
// In gateway/src/auth.rs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub role: String,
}

pub fn generate_token(sub: &str, role: &str) -> Result<String, Error>
pub fn verify_token(token: &str) -> Result<TokenData<Claims>, Error>

pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // GET requests: no auth required
    // POST/PUT/DELETE: require valid Bearer token
}
```

**Usage:**
```bash
# Generate token
TOKEN=$(zenith auth generate --role deployer --exp 86400)

# Use in API calls
curl -H "Authorization: Bearer $TOKEN" \
  -X POST https://api.zenith.io/v1/canisters \
  -d '{...}'

# Verify token
curl -H "Authorization: Bearer $TOKEN" https://api.zenith.io/health
```

**Secret Management:**
```rust
// CHANGE THIS FOR PRODUCTION
pub const JWT_SECRET: &[u8] = b"zenith-platform-secret-key-change-in-production";
```

Use environment variables in production:
```bash
export JWT_SECRET=$(openssl rand -base64 32)
# Embed into binary at build time or read from vault
```

### 3. ✅ Prometheus Metrics & Observability

**Why:** No visibility into platform usage, errors, or performance.

**Solution:** Prometheus metrics on critical operations:

```rust
// In gateway/src/metrics.rs
pub struct Metrics {
    pub requests_total: IntCounter,
    pub errors_total: IntCounter,
    pub canisters_deployed: IntCounter,
    pub proofs_verified: IntCounter,
    pub proofs_submitted: IntCounter,
    pub provers_registered: IntCounter,
    pub active_canisters: IntGauge,
    pub pending_proofs: IntGauge,
}
```

**Access metrics:**
```bash
curl http://gateway:8000/v1/metrics

# Example output:
zenith_requests_total 1234
zenith_errors_total 3
zenith_canisters_deployed_total 42
zenith_active_canisters 38
```

**Alert on:**
- Error rate > 1% of requests
- Proof verification failures > 5
- Pending proofs > 1000
- Gateway latency p99 > 1000ms

### 4. ✅ Structured Logging

**Why:** Needed to debug issues and audit operations.

**Solution:** Integrated `tracing` with structured fields:

```rust
// Automatically logs request/response with tracing middleware
info!("Deployed canister: {} with prover {} in block {}",
      response.canister_id, prover.as_str(), response.block_number);

// Configure via environment:
RUST_LOG=zenith_gateway=debug,hyper=info
```

### 5. ✅ Input Validation

**Why:** Malformed input could crash gateway or execute unintended code.

**Solution:** Validate all request fields:

```rust
// Example: deploy_canister
let wasm = payload["wasm_base64"]
    .as_str()
    .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing wasm_base64".to_string()))?
    .to_string();

// Base64 validation happens on decode
let wasm_bytes = BASE64.decode(&wasm)?;
```

## Recommended Hardening (Pre-Production)

### 1. Rate Limiting

**Issue:** Platform can be DDoS'd via API endpoints.

**Solution:** Add `governor` rate limiter:

```rust
// Add to gateway/src/rate_limit.rs
use governor::Quota;

let limiter = governor::RateLimiter::direct(Quota::per_second(100));

// In middleware
if limiter.check().is_err() {
    return Err(StatusCode::TOO_MANY_REQUESTS);
}
```

**Deploy with reverse proxy (nginx):**
```nginx
limit_req_zone $binary_remote_addr zone=api:10m rate=100r/s;
limit_req zone=api burst=200;
```

### 2. TLS/HTTPS

**Issue:** Network traffic is in plaintext, exposing auth tokens.

**Solution:** Use Let's Encrypt certificates:

```bash
certbot certonly --standalone -d api.zenith.io

# In nginx
ssl_certificate /etc/letsencrypt/live/api.zenith.io/fullchain.pem;
ssl_certificate_key /etc/letsencrypt/live/api.zenith.io/privkey.pem;
```

### 3. Circuit Breaker for Node RPC

**Issue:** If Substrate node goes down, gateway hangs requests.

**Solution:** Add timeout + circuit breaker:

```rust
// gateway/src/rpc_client.rs
let client = reqwest::ClientBuilder::new()
    .timeout(Duration::from_secs(10))
    .build()?;

// Retry with exponential backoff
async fn call_with_retry(&self, ...) -> Result<...> {
    let mut backoff = 100ms;
    for attempt in 0..3 {
        match self.call(...).await {
            Ok(result) => return Ok(result),
            Err(e) if attempt < 2 => {
                sleep(backoff).await;
                backoff *= 2;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 4. Audit Logging

**Issue:** No record of who deployed what canister or verified which proofs.

**Solution:** Log all write operations:

```rust
// Log to file/syslog
info!(
    user = claims.sub,
    action = "deploy_canister",
    canister_id = response.canister_id,
    block_number = response.block_number,
    "Canister deployment"
);
```

### 5. Database Encryption

**Issue:** Sensitive data (proof hashes, pending calls) stored in plaintext.

**Solution:** Encrypt sled database:

```rust
// gateway/src/db.rs
use sled_encryption::EncryptedDb;

let config = sled::Config::new()
    .encryption(EncryptionConfig::new(encryption_key));

let db = config.open()?;
```

### 6. Token Rotation & Expiry

**Issue:** Long-lived tokens could be compromised.

**Solution:** Force token rotation:

```rust
// In Claims
pub struct Claims {
    pub sub: String,
    pub exp: i64,  // Unix timestamp
    pub iat: i64,  // Issued at
    pub role: String,
}

// Verify expiry on every request
if claims.exp < Utc::now().timestamp() {
    return Err(StatusCode::UNAUTHORIZED);
}
```

**Policy:**
- Default expiry: 24 hours
- Max expiry: 30 days
- Require rotation every 90 days

## Security Checklist Before Launch

- [ ] **Network**
  - [ ] TLS/HTTPS enabled on gateway
  - [ ] Reverse proxy (nginx/Caddy) in front
  - [ ] Rate limiting configured (100 req/s per IP)
  - [ ] Firewall blocks non-whitelisted IPs from RPC port

- [ ] **Authentication**
  - [ ] JWT secret changed from default
  - [ ] Secret stored in vault (not source code)
  - [ ] Token rotation policy documented
  - [ ] API keys/tokens never logged

- [ ] **Database**
  - [ ] Encryption at rest enabled
  - [ ] Daily backups tested (restore from backup)
  - [ ] Backup retention: 30 days
  - [ ] Backup location: offline/remote

- [ ] **Monitoring**
  - [ ] Prometheus metrics scraping configured
  - [ ] Alerting rules for error rate > 1%
  - [ ] Centralized logging (ELK, Datadog, etc.)
  - [ ] Real-time dashboard for gateway health

- [ ] **Operations**
  - [ ] Run-book for incident response
  - [ ] On-call rotation for critical alerts
  - [ ] Deployment process documented
  - [ ] Rollback procedure tested

- [ ] **Compliance**
  - [ ] Audit logs enabled
  - [ ] Data retention policy documented
  - [ ] Privacy policy matches storage practices
  - [ ] GDPR/SOC 2 controls implemented

## Security Incident Response

### If Gateway is Compromised

1. **Immediate:**
   - Rotate JWT secret
   - Revoke all active tokens
   - Block compromised IP addresses

2. **Short-term (hours):**
   - Audit database for unauthorized deployments
   - Restore database from clean backup if needed
   - Analyze logs for attack patterns

3. **Long-term (days):**
   - Implement 2FA for critical operations
   - Upgrade to hardware security keys
   - Conduct security audit

### If Database is Corrupted

1. Restore from latest backup: `cp -r /backups/zenith-gateway-<timestamp> /data/zenith-gateway`
2. Restart gateway: `systemctl restart zenith-gateway`
3. Verify data integrity: `curl http://localhost:8000/health`

### If RPC Node Goes Down

- Gateway operates in demo mode (mocked responses)
- Alerting triggers after 5 minutes of disconnection
- Manual failover: Point `--node-rpc` to backup node
- Automatic recovery: RPC client retries with exponential backoff

## References

- [OWASP Top 10](https://owasp.org/Top10/)
- [Substrate Security Guidelines](https://docs.substrate.io/deploy/security/)
- [JWT Best Practices](https://tools.ietf.org/html/rfc8725)
- [Let's Encrypt](https://letsencrypt.org)
