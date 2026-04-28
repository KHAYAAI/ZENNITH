// Load test for Zenith Gateway
// Run with: k6 run tests/load_test.js
// Or install k6: https://k6.io/docs/getting-started/installation/

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const deploymentLatency = new Trend('deployment_latency');
const callLatency = new Trend('call_latency');
const verifyLatency = new Trend('verify_latency');
const deploymentCount = new Counter('deployments_total');
const callCount = new Counter('calls_total');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8000';
const AUTH_TOKEN = __ENV.AUTH_TOKEN || '';

// Virtual user scenarios
export const options = {
  // Test stages: ramp up -> sustained -> ramp down
  stages: [
    { duration: '30s', target: 10 },     // Ramp up to 10 users
    { duration: '1m30s', target: 50 },   // Ramp up to 50 users
    { duration: '2m', target: 50 },      // Stay at 50 users
    { duration: '30s', target: 0 },      // Ramp down
  ],
  // Thresholds for pass/fail
  thresholds: {
    'deployment_latency': ['p(95)<500', 'p(99)<1000'], // 95th percentile < 500ms
    'call_latency': ['p(95)<200', 'p(99)<500'],
    'errors': ['rate<0.1'], // Error rate < 10%
  },
  ext: {
    loadimpact: {
      projectID: 0, // No cloud sync unless configured
    },
  },
};

// Sample WASM bytecode (minimal valid WASM module)
const WASM_BASE64 = 'AGFzbQEAAAABBIABf2AAAA=='; // Minimal WASM

function generateCanisterId() {
  return `canister-${Math.random().toString(36).substring(7)}`;
}

export default function () {
  // Deploy canister
  group('deploy_canister', function () {
    const deployPayload = JSON.stringify({
      wasm_base64: WASM_BASE64,
      init_args_base64: '',
      cycles: 1000000,
    });

    const deployParams = {
      headers: {
        'Content-Type': 'application/json',
        ...(AUTH_TOKEN && { Authorization: `Bearer ${AUTH_TOKEN}` }),
      },
    };

    const deployResponse = http.post(
      `${BASE_URL}/v1/canisters`,
      deployPayload,
      deployParams
    );

    const deployOk = check(deployResponse, {
      'deployment status is 201': (r) => r.status === 201,
      'canister_id exists': (r) => r.json('canister_id') !== undefined,
      'deployment completes in < 1s': (r) => r.timings.duration < 1000,
    });

    deploymentLatency.add(deployResponse.timings.duration);
    deploymentCount.add(1);
    errorRate.add(!deployOk);
  });

  sleep(1);

  // Call canister
  group('call_canister', function () {
    const canisterId = generateCanisterId();
    const callPayload = JSON.stringify({
      method: 'test',
      input_base64: 'dGVzdA==', // 'test' in base64
      gas_limit: 100000,
    });

    const callParams = {
      headers: {
        'Content-Type': 'application/json',
        ...(AUTH_TOKEN && { Authorization: `Bearer ${AUTH_TOKEN}` }),
      },
    };

    const callResponse = http.post(
      `${BASE_URL}/v1/canisters/${canisterId}/call/test`,
      callPayload,
      callParams
    );

    const callOk = check(callResponse, {
      'call status is 202 (accepted)': (r) => r.status === 202,
      'call_id exists': (r) => r.json('call_id') !== undefined,
      'call completes in < 500ms': (r) => r.timings.duration < 500,
    });

    callLatency.add(callResponse.timings.duration);
    callCount.add(1);
    errorRate.add(!callOk);
  });

  sleep(1);

  // Verify proof
  group('verify_proof', function () {
    const proofHash = `proof-${Math.random().toString(36).substring(7)}`;

    const verifyParams = {
      headers: {
        ...(AUTH_TOKEN && { Authorization: `Bearer ${AUTH_TOKEN}` }),
      },
    };

    const verifyResponse = http.get(
      `${BASE_URL}/v1/proofs/${proofHash}/verify`,
      verifyParams
    );

    // 404 is expected for non-existent proofs
    const verifyOk = check(verifyResponse, {
      'verify returns 200 or 404': (r) => r.status === 200 || r.status === 404,
      'verify completes in < 200ms': (r) => r.timings.duration < 200,
    });

    verifyLatency.add(verifyResponse.timings.duration);
    errorRate.add(!verifyOk);
  });

  sleep(1);

  // Health check
  group('health_check', function () {
    const healthResponse = http.get(`${BASE_URL}/health`);

    check(healthResponse, {
      'health status is 200': (r) => r.status === 200,
      'health status is "healthy"': (r) => r.json('status') === 'healthy',
    });
  });

  sleep(1);

  // List canisters
  group('list_canisters', function () {
    const listParams = {
      headers: {
        ...(AUTH_TOKEN && { Authorization: `Bearer ${AUTH_TOKEN}` }),
      },
    };

    const listResponse = http.get(`${BASE_URL}/v1/canisters`, listParams);

    check(listResponse, {
      'list status is 200': (r) => r.status === 200,
      'list returns array': (r) => Array.isArray(r.json()),
    });
  });

  sleep(2);
}
