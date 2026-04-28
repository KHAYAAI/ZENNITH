# Zenith Gateway API Reference

## Base URL
```
https://api.zenith.io
http://localhost:8000 (local development)
```

## Authentication
All write operations require JWT authentication via Bearer token:
```
Authorization: Bearer <jwt-token>
```

Get operations (GET, health) do not require authentication.

## Endpoints

### Health & Metrics

#### GET `/health`
Check gateway health status.

**Response (200 OK):**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600
}
```

#### GET `/v1/metrics`
Get Prometheus metrics in text format.

**Response (200 OK):**
```
zenith_requests_total 1234
zenith_errors_total 3
zenith_canisters_deployed_total 42
zenith_active_canisters 38
```

### Canisters

#### POST `/v1/canisters`
Deploy a new canister.

**Request:**
```json
{
  "wasm_base64": "AGFzbQEAAAA=",
  "init_args_base64": "",
  "cycles": 1000000
}
```

**Response (201 Created):**
```json
{
  "canister_id": "canister-abc123",
  "tx_hash": "0x1234...",
  "block_number": 42,
  "status": "deployed",
  "prover_system": "Plonk",
  "routing": {
    "workload_type": "NeuralNetwork",
    "confidence": 0.95
  }
}
```

#### GET `/v1/canisters`
List all deployed canisters.

**Response (200 OK):**
```json
[
  {
    "id": "canister-abc123",
    "status": "deployed",
    "cycles": 1000000,
    "created_at": "2024-01-01T12:00:00Z"
  }
]
```

#### GET `/v1/canisters/:canister_id`
Get details of a specific canister.

**Response (200 OK):**
```json
{
  "id": "canister-abc123",
  "status": "deployed",
  "cycles": 1000000,
  "created_at": "2024-01-01T12:00:00Z",
  "calls": [
    {
      "call_id": "call-xyz789",
      "method": "infer",
      "status": "completed",
      "proof_hash": "0x..."
    }
  ]
}
```

#### DELETE `/v1/canisters/:canister_id`
Stop a canister.

**Response (200 OK):**
```json
{
  "id": "canister-abc123",
  "status": "stopped"
}
```

### Canister Calls

#### POST `/v1/canisters/:canister_id/call/:method`
Call a canister method and generate a proof.

**Request:**
```json
{
  "method": "infer",
  "input_base64": "dGVzdA==",
  "gas_limit": 100000
}
```

**Response (202 Accepted):**
```json
{
  "call_id": "call-xyz789",
  "canister_id": "canister-abc123",
  "status": "pending",
  "estimated_latency_ms": 250,
  "prover_system": "Plonk"
}
```

#### GET `/v1/canisters/:canister_id/calls/:call_id`
Get the result of a canister call.

**Response (200 OK):**
```json
{
  "call_id": "call-xyz789",
  "canister_id": "canister-abc123",
  "status": "completed",
  "result_base64": "cmVzdWx0",
  "proof_hash": "0x..."
}
```

### Proofs

#### POST `/v1/proofs`
Submit a proof for verification.

**Request:**
```json
{
  "proof_base64": "AGFzbQEAAAA=",
  "prover_system": "Plonk",
  "canister_id": "canister-abc123"
}
```

**Response (201 Created):**
```json
{
  "proof_hash": "0x1234...",
  "canister_id": "canister-abc123",
  "prover_system": "Plonk",
  "status": "submitted"
}
```

#### GET `/v1/proofs/:proof_hash`
Get proof metadata.

**Response (200 OK):**
```json
{
  "proof_hash": "0x1234...",
  "computation_hash": "0x...",
  "result_hash": "0x...",
  "prover_system": "Plonk",
  "timestamp": 1704110400,
  "canister_id": "canister-abc123",
  "call_id": "call-xyz789"
}
```

#### GET `/v1/proofs/:proof_hash/verify`
Verify a proof.

**Response (200 OK):**
```json
{
  "proof_hash": "0x1234...",
  "valid": true,
  "prover_system": "Plonk",
  "timestamp": 1704110400
}
```

### Provers

#### GET `/v1/provers`
List all registered provers.

**Response (200 OK):**
```json
[
  {
    "prover_id": "prover-abc123",
    "node_address": "prover-1.zenith.io:9001",
    "gpu_count": 2,
    "stake_amount": 5000,
    "status": "registered",
    "registered_at": "2024-01-01T12:00:00Z"
  }
]
```

#### POST `/v1/provers/register`
Register a new prover node.

**Request:**
```json
{
  "node_address": "prover-1.zenith.io:9001",
  "gpu_count": 2,
  "stake_amount": 5000
}
```

**Response (200 OK):**
```json
{
  "prover_id": "prover-abc123",
  "status": "registered",
  "node_address": "prover-1.zenith.io:9001",
  "gpu_count": 2
}
```

## Error Responses

### 400 Bad Request
Missing or invalid request parameters.
```json
{
  "error": "wasm_base64 is required"
}
```

### 401 Unauthorized
Missing or invalid JWT token.
```json
{
  "error": "Invalid or missing Authorization header"
}
```

### 404 Not Found
Resource not found.
```json
{
  "error": "canister not found",
  "id": "canister-unknown"
}
```

### 429 Too Many Requests
Rate limit exceeded. Retry after delay.
```json
{
  "error": "Too many requests"
}
```

### 500 Internal Server Error
Server-side error.
```json
{
  "error": "Failed to deploy canister"
}
```

## Rate Limiting

**Global:** 1,000 requests/second per instance  
**Per-IP:** 100 requests/second  

Responses exceeding limits return `429 Too Many Requests`.

## Authentication Example

```bash
# Generate token (admin only)
TOKEN=$(zenith auth generate --role deployer --exp 3600)

# Deploy canister
curl -X POST https://api.zenith.io/v1/canisters \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"wasm_base64":"...", "init_args_base64":"", "cycles":1000000}'
```

## Webhook Events (Future)

Webhooks for proof completion, canister execution, and prover registration:

```
POST /webhooks/proof-completed
POST /webhooks/canister-called
POST /webhooks/prover-registered
```
