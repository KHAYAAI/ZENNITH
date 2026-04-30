# Zenith Operations Training Manual

**Target Audience:** DevOps Engineers, SREs, System Administrators  
**Duration:** 4-8 hours of self-study + hands-on practice  
**Certification:** Upon completion of all exercises + final quiz  

---

## Module 1: Platform Architecture (30 min)

### 1.1 What is Zenith?

Zenith is a decentralized cloud for verifiable private computation. It allows users to:

1. **Upload WASM canister** (compiled algorithm)
2. **Network executes it** and generates cryptographic proof
3. **Result is verifiable** on-chain via Substrate blockchain
4. **User pays** in $ZEN tokens

### 1.2 Five-Layer Architecture

```
Layer 1: User APIs (REST Gateway)
         ↓
Layer 2: Execution Engine (Wasmtime JIT)
         ↓
Layer 3: ZK Provers (Plonk, Cairo, RISC Zero)
         ↓
Layer 4: Blockchain (Substrate)
         ↓
Layer 5: Network & Storage (P2P + sled DB)
```

### 1.3 Key Components to Operate

| Component | Purpose | Status | Tech |
|-----------|---------|--------|------|
| **Gateway** | HTTP API for users | Critical | Axum + Tokio |
| **Validators** | Produce blocks | Critical | Substrate BABE |
| **Provers** | Generate ZK proofs | Critical | Plonk/Cairo/RISC Zero |
| **Database** | Persistent state | Critical | sled |
| **Nginx** | TLS/reverse proxy | Important | TLS 1.3 |

---

## Module 2: Running a Validator Node (1 hour)

### 2.1 Prerequisites

```bash
# System Requirements
- OS: Linux (Ubuntu 20.04+ recommended)
- CPU: 4+ cores
- RAM: 8GB minimum
- Storage: 50GB SSD
- Network: 1Mbps minimum

# Install dependencies
sudo apt-get update
sudo apt-get install -y \
  clang curl libssl-dev llvm libudev-dev protobuf-compiler git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2.2 Build Node Binary

```bash
cd /home/user/ZENNITH/zenith

# Build release binary (takes ~10 minutes)
cargo build --release -p zenith-node

# Verify build
file target/release/zenith-node
# Expected: ELF 64-bit executable

# Binary location
/home/user/ZENNITH/zenith/target/release/zenith-node
```

### 2.3 Run Validator Node

**Development mode (single validator):**
```bash
/home/user/ZENNITH/zenith/target/release/zenith-node \
  --dev \
  --validator \
  --rpc-external \
  --ws-external \
  --log=info,zenith=debug
```

**Testnet mode (alice, bob, charlie):**
```bash
/home/user/ZENNITH/zenith/target/release/zenith-node \
  --alice \
  --validator \
  --chain=custom-spec.json \
  --data-dir=./data/alice \
  --rpc-port=9944 \
  --p2p-port=30333 \
  --http-port=8000 \
  --unsafe-rpc-external \
  --ws-external
```

**Mainnet mode:**
```bash
/home/user/ZENNITH/zenith/target/release/zenith-node \
  --validator \
  --chain=mainnet \
  --data-dir=/var/lib/zenith/data \
  --rpc-port=9944 \
  --p2p-port=30333 \
  --pruning=archive \
  --unsafe-rpc-external \
  --log=info,zenith=warn
```

### 2.4 Monitor Validator Health

**Check if running:**
```bash
ps aux | grep zenith-node | grep -v grep
```

**Check sync progress:**
```bash
curl -s -X POST "http://localhost:9944" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_syncState","params":[],"id":1}' | jq .
```

**Expected output (synced):**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "startingBlock": 0,
    "currentBlock": 5000,
    "highestBlock": 5000
  },
  "id": 1
}
```

**Check block production:**
```bash
# Should increment every ~6 seconds
watch -n 1 'curl -s -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"chain_getHeader\",\"params\":[],\"id\":1}" | jq .result.number'
```

### 2.5 Common Issues & Solutions

**Issue: "Port 9944 already in use"**
```bash
# Find process using port
lsof -i :9944

# Kill it
kill -9 <PID>

# Or use different port
--rpc-port=9945
```

**Issue: "Database locked"**
```bash
# Another instance is running, or unclean shutdown
ps aux | grep zenith-node

# If truly stuck:
rm -rf data/alice/db
# Node will resync from network
```

**Issue: "Peer connection refused"**
```bash
# Check firewall
sudo ufw status
sudo ufw allow 30333/tcp

# Check NAT/router (port forwarding)
# Ensure 30333 is forwarded to your node
```

**Exercise 2.5:** Start validator, verify blocks produced for 5 minutes, confirm sync status.

---

## Module 3: Running the Gateway (1 hour)

### 3.1 Build Gateway

```bash
cd /home/user/ZENNITH/zenith

# Build release
cargo build --release -p zenith-gateway

# Binary location
target/release/gateway
```

### 3.2 Start Gateway

```bash
# Production startup
/home/user/ZENNITH/zenith/target/release/gateway \
  --listen 0.0.0.0:8000 \
  --node-rpc ws://localhost:9944 \
  --data-dir ./gateway-data \
  --log=info

# Expected output:
# 2026-04-30T10:30:45Z INFO: Gateway listening on 0.0.0.0:8000
# 2026-04-30T10:30:46Z INFO: Connected to node at ws://localhost:9944
# 2026-04-30T10:30:47Z INFO: Rate limiting: 1000 req/s global, 100 req/s per IP
```

### 3.3 Verify Gateway Health

```bash
# Health check endpoint
curl http://localhost:8000/health

# Expected:
# {"status":"ok","node_connected":true,"latency_ms":1.2}

# Check version
curl http://localhost:8000/version | jq .

# List metrics
curl http://localhost:8000/v1/metrics | head -20
```

### 3.4 Test API Endpoints

```bash
# Deploy a canister
JWT_TOKEN="<from-auth-endpoint>"

curl -X POST http://localhost:8000/v1/canisters \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "wasm_base64": "AGFzbQEAAAA=",
    "init_args_base64": "",
    "cycles": 1000000
  }' | jq .

# List canisters
curl http://localhost:8000/v1/canisters | jq .

# Check proofs
curl http://localhost:8000/v1/proofs | jq .
```

### 3.5 Gateway Monitoring

**Memory usage:**
```bash
watch -n 1 'ps aux | grep gateway | grep -v grep | awk "{print \"Memory: \" \$6 \"KB, CPU: \" \$3 \"%\"}"'
```

**Request latency (via prometheus metrics):**
```bash
curl http://localhost:8000/v1/metrics | grep "http_request_duration"
```

**Error rate:**
```bash
curl http://localhost:8000/v1/metrics | grep "http_errors_total"
```

**Exercise 3.5:** Start gateway, verify health, deploy example canister, check metrics for 2 minutes.

---

## Module 4: Database Operations (45 min)

### 4.1 Understanding sled

sled is an embedded key-value database (like RocksDB):
- **ACID compliance** (Atomic, Consistent, Isolated, Durable)
- **Survives restarts** (data persisted to disk)
- **No backups needed** (data is safe immediately)

Data stored in: `gateway-data/` directory

### 4.2 Database Backup

**Manual backup:**
```bash
# Stop gateway first
pkill -f zenith-gateway

# Copy database directory
cp -r gateway-data gateway-data-backup-2026-04-30

# Tar it for storage
tar -czf gateway-data-backup-2026-04-30.tar.gz gateway-data-backup-2026-04-30

# Verify backup
tar -tzf gateway-data-backup-2026-04-30.tar.gz | head -20

# Restart gateway
/path/to/gateway --listen 0.0.0.0:8000 ...
```

**Automated daily backup script:**
```bash
#!/bin/bash
# backup-daily.sh

BACKUP_DIR="/backups/zenith"
RETENTION_DAYS=30

# Create backup
mkdir -p "$BACKUP_DIR"
TIMESTAMP=$(date '+%Y-%m-%d')
tar -czf "$BACKUP_DIR/gateway-data-$TIMESTAMP.tar.gz" gateway-data

# Cleanup old backups
find "$BACKUP_DIR" -name "gateway-data-*.tar.gz" -mtime +$RETENTION_DAYS -delete

echo "Backup completed: gateway-data-$TIMESTAMP.tar.gz"
```

### 4.3 Database Recovery

**Restore from backup:**
```bash
# Stop gateway
pkill -f zenith-gateway

# Remove corrupted database
rm -rf gateway-data

# Extract backup
tar -xzf gateway-data-backup-2026-04-30.tar.gz
mv gateway-data-backup-2026-04-30 gateway-data

# Verify recovery
ls -la gateway-data/
# Should show files: db, manifest, log, etc.

# Restart gateway
/path/to/gateway --listen 0.0.0.0:8000 ...

# Verify functionality
curl http://localhost:8000/health
```

**Verify backup is restorable:**
```bash
# Test restore to different location
mkdir -p /tmp/restore-test
cd /tmp/restore-test
tar -xzf /backups/zenith/gateway-data-2026-04-30.tar.gz

# Check files present
ls -la gateway-data/
# Should not be empty

# Success if you see db files
```

### 4.4 Monitoring Database Health

**Check disk usage:**
```bash
du -sh gateway-data/
# Expected: <1GB for typical usage

# If growing too fast:
ls -lt gateway-data/ | head -10
# Look for which files are newest/largest
```

**Check for corruption:**
```bash
# If database won't open after crash:
# Option 1: Restore from backup (preferred)
# Option 2: Delete and let node resync from chain

rm -rf gateway-data
# Gateway will rebuild from blockchain state
```

**Exercise 4.4:** Create backup, verify it extracts, confirm canisters in restored backup.

---

## Module 5: Incident Response (1.5 hours)

### 5.1 Incident Response Plan

**Priority Levels:**
- **P1 (Critical):** Mainnet down, funds at risk
- **P2 (High):** Performance degraded, users impacted
- **P3 (Medium):** Partial outage, workaround exists
- **P4 (Low):** Cosmetic issues, no impact on users

### 5.2 Common Incidents & Solutions

**Incident 1: Gateway Not Responding**

**Symptoms:**
```
curl http://localhost:8000/health
# Connection refused
```

**Diagnosis (5 min):**
```bash
# 1. Is it running?
ps aux | grep zenith-gateway

# 2. Check logs
tail -100 gateway.log | grep -i error

# 3. Check if port is in use
lsof -i :8000

# 4. Check disk space
df -h | grep -E "/|data"
```

**Solutions:**
```bash
# Solution 1: Process crashed (likely)
pkill -f zenith-gateway
/path/to/gateway --listen 0.0.0.0:8000 ... &

# Solution 2: Port in use
lsof -i :8000 | tail -1 | awk '{print $2}' | xargs kill -9
# Then restart gateway

# Solution 3: Disk full
# Delete old logs, request more storage

# Solution 4: Database corrupted
rm -rf gateway-data  # Will rebuild from blockchain
```

**Escalation:** If still down after 15 min, pages on-call engineer.

---

**Incident 2: Validator Not Producing Blocks**

**Symptoms:**
```
# Block height not incrementing
curl -s http://localhost:9944 ... | jq .result.number
# Same block for >1 minute
```

**Diagnosis (5 min):**
```bash
# 1. Is validator running?
ps aux | grep zenith-node | grep -v grep

# 2. Check sync status
curl -s http://localhost:9944 ... -d '{"jsonrpc":"2.0","method":"system_syncState"}' | jq .

# 3. Check peer count
curl -s http://localhost:9944 ... -d '{"jsonrpc":"2.0","method":"system_networkState"}' | jq .

# 4. Check logs
tail -100 validator.log | grep -i "error\|warn\|authority"
```

**Solutions:**
```bash
# Solution 1: Out of sync (likely)
# Just wait, consensus will resume as it syncs
# Monitor: watch for network state to show >2 peers

# Solution 2: Network partition
# Check firewall: sudo ufw allow 30333/tcp
# Check NAT: ensure port 30333 forwarded

# Solution 3: Database corrupted
pkill -f zenith-node
rm -rf data/alice/db
# Restart, will resync from network

# Solution 4: Not in validator set
# Need to stake and be elected (mainnet only)
```

**Escalation:** If >30 min without blocks and >2 peers, check blockchain state.

---

**Incident 3: High Memory Usage**

**Symptoms:**
```
watch -n 1 'ps aux | grep zenith-gateway'
# Memory gradually increases from 250MB → 500MB+
```

**Diagnosis (5 min):**
```bash
# 1. Check memory trend
ps aux | grep gateway | awk '{print $6}'  # Check multiple times

# 2. Check active connections
netstat -an | grep ESTABLISHED | wc -l

# 3. Check database size
du -sh gateway-data/

# 4. Check number of canisters
curl http://localhost:8000/v1/canisters | jq '. | length'
```

**Solutions:**
```bash
# Solution 1: Normal growth (memory cache filling)
# Nothing needed if still <1GB, gateway is efficient

# Solution 2: Memory leak (memory keeps growing)
pkill -f zenith-gateway
sleep 5
/path/to/gateway ...  # Restart

# Solution 3: Malicious load (too many canisters)
# Contact ops lead, may need to delete old canisters

# Solution 4: Not enough RAM
# Shut down other services or add more RAM
```

**Escalation:** If memory >4GB or still growing after restart, pages ops lead.

---

### 5.3 Incident Response Checklist

**When an incident occurs:**

```
[ ] Page appropriate on-call engineer
[ ] Create incident Slack channel
[ ] Post initial status
[ ] Run diagnostics (5 min max)
[ ] Attempt fix (15 min max)
[ ] If not resolved: escalate to senior engineer
[ ] Document what was tried
[ ] Update status every 5 minutes
[ ] Root cause analysis after resolved
```

**Communication Template:**

```
🚨 INCIDENT: [Brief description]
Status: INVESTIGATING / IN PROGRESS / RESOLVED
Severity: P1 / P2 / P3 / P4
Affected: [Users/Services impacted]
ETA: [Minutes until resolved]
Updates: [Latest status]
```

### 5.4 Runbook Examples

**Runbook: Restart Gateway**
```bash
#!/bin/bash
echo "Shutting down gateway..."
pkill -f zenith-gateway

# Wait for clean shutdown
sleep 5

echo "Clearing any locks..."
rm -f gateway-data/lock

echo "Restarting gateway..."
/home/user/ZENNITH/zenith/target/release/gateway \
  --listen 0.0.0.0:8000 \
  --node-rpc ws://localhost:9944 \
  --data-dir ./gateway-data \
  --log=info &

# Verify
sleep 2
curl http://localhost:8000/health
echo "Gateway restarted"
```

**Runbook: Failover to Secondary Gateway**
```bash
# If primary gateway down >5 min
# Start secondary gateway on different host

ssh ops@secondary-host.internal

# Ensure node is in sync
curl -s http://localhost:9944 ... -d ... | jq .

# Start gateway
/home/zenith/gateway --listen 0.0.0.0:8000 ...

# Update DNS to point to secondary
# (Ops lead does this)

# Monitor
watch -n 1 'curl http://localhost:8000/v1/metrics'
```

**Exercise 5.4:** Simulate incident: kill gateway, run diagnostics, restart it, verify recovery.

---

## Module 6: Monitoring & Alerting (45 min)

### 6.1 Key Metrics to Monitor

| Metric | Target | Alert If | Critical |
|--------|--------|----------|----------|
| Gateway Health | UP | DOWN | Yes |
| Block Time | 6s ± 1s | >12s | Yes |
| Finality Latency | <30s | >60s | Yes |
| API Latency (p95) | <200ms | >500ms | Yes |
| Error Rate | <1% | >5% | Yes |
| Memory Usage | <1GB | >4GB | Yes |
| Disk Free | >20GB | <5GB | Yes |
| Connected Peers | >2 | <1 | Yes |
| Validator Active | Yes | No | Yes |

### 6.2 Prometheus Metrics

**Gateway metrics:**
```bash
# View all metrics
curl http://localhost:8000/v1/metrics

# Key metrics:
# zenith_http_request_duration_seconds (latency)
# zenith_http_requests_total (throughput)
# zenith_http_errors_total (errors)
# zenith_active_canisters (state)
```

### 6.3 Alert Rules (Prometheus)

Create file: `prometheus-alerts.yml`

```yaml
groups:
  - name: zenith_alerts
    interval: 30s
    rules:

      # Gateway down
      - alert: GatewayDown
        expr: up{job="zenith-gateway"} == 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Gateway is down"
          runbook: "Run: pkill -f zenith-gateway && restart-gateway.sh"

      # High latency
      - alert: HighLatency
        expr: histogram_quantile(0.95, zenith_http_request_duration_seconds) > 0.5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "API latency high (p95 > 500ms)"

      # High error rate
      - alert: HighErrorRate
        expr: (rate(zenith_http_errors_total[5m]) / rate(zenith_http_requests_total[5m])) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Error rate > 5%"

      # Memory leak
      - alert: HighMemory
        expr: process_resident_memory_bytes / (1024*1024) > 4096
        for: 10m
        labels:
          severity: critical
        annotations:
          summary: "Memory usage > 4GB"
          runbook: "Restart gateway: pkill -f zenith-gateway"

      # Blocks not produced
      - alert: NoBlockProduction
        expr: increase(zenith_blocks_produced_total[5m]) == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "No new blocks produced in 5 minutes"
```

### 6.4 Setting Up Monitoring

**Install Prometheus:**
```bash
wget https://github.com/prometheus/prometheus/releases/download/v2.40.0/prometheus-2.40.0.linux-amd64.tar.gz
tar xzvf prometheus-2.40.0.linux-amd64.tar.gz
cd prometheus-2.40.0.linux-amd64
```

**Configure Prometheus:**
Create: `prometheus.yml`
```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'zenith-gateway'
    static_configs:
      - targets: ['localhost:8000']
    metrics_path: '/v1/metrics'
```

**Start Prometheus:**
```bash
./prometheus --config.file=prometheus.yml
# Open http://localhost:9090
```

**Exercise 6.4:** Setup Prometheus, query metrics, verify alert rules work.

---

## Module 7: Deployment to Production (1 hour)

### 7.1 Pre-Deployment Checklist

```
[ ] Code reviewed and approved
[ ] All tests passing
[ ] Security audit completed
[ ] Performance baselines established
[ ] Runbooks written and tested
[ ] Team trained and certified
[ ] Monitoring configured
[ ] Backup tested and verified
[ ] Firewall rules configured
[ ] DNS configured
[ ] Load balancer configured (if needed)
[ ] Rollback plan documented
```

### 7.2 Deployment Steps

**Step 1: Prepare Infrastructure (4 hours)**
```bash
# 1. Provision servers
# - 3+ validator nodes (4 CPU, 8GB RAM minimum)
# - 2+ gateway nodes (4 CPU, 8GB RAM)
# - 1 load balancer (nginx)

# 2. Configure networking
sudo ufw enable
sudo ufw allow 22/tcp      # SSH
sudo ufw allow 9944/tcp    # RPC
sudo ufw allow 30333/tcp   # P2P
sudo ufw allow 80/tcp      # HTTP
sudo ufw allow 443/tcp     # HTTPS

# 3. Setup monitoring
# Install Prometheus + Grafana

# 4. Setup backups
# Setup daily backup cron jobs
```

**Step 2: Deploy Blockchain Nodes (2 hours)**
```bash
# On each validator node:

# 1. Clone and build
git clone https://github.com/yourgit/zenith.git
cd zenith
cargo build --release -p zenith-node

# 2. Create data directory
mkdir -p /var/lib/zenith/data
chmod 700 /var/lib/zenith

# 3. Create systemd service
sudo tee /etc/systemd/system/zenith-validator.service > /dev/null <<EOF
[Unit]
Description=Zenith Validator
After=network.target

[Service]
Type=simple
User=zenith
WorkingDirectory=/home/zenith
ExecStart=/home/zenith/zenith/target/release/zenith-node \
  --validator \
  --chain=mainnet \
  --data-dir=/var/lib/zenith/data \
  --rpc-port=9944 \
  --p2p-port=30333 \
  --unsafe-rpc-external \
  --log=info
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# 4. Start service
sudo systemctl daemon-reload
sudo systemctl enable zenith-validator
sudo systemctl start zenith-validator

# 5. Verify
journalctl -u zenith-validator -f
```

**Step 3: Deploy Gateways (1 hour)**
```bash
# On each gateway node:

# 1. Build gateway
cargo build --release -p zenith-gateway

# 2. Create systemd service
sudo tee /etc/systemd/system/zenith-gateway.service > /dev/null <<EOF
[Unit]
Description=Zenith Gateway
After=network.target

[Service]
Type=simple
User=zenith
WorkingDirectory=/home/zenith
ExecStart=/home/zenith/zenith/target/release/gateway \
  --listen 127.0.0.1:8000 \
  --node-rpc ws://localhost:9944 \
  --data-dir /var/lib/zenith/gateway-data \
  --log=info
Restart=always
RestartSec=10
Environment="JWT_SECRET=your-secret-from-env"

[Install]
WantedBy=multi-user.target
EOF

# 3. Start
sudo systemctl enable zenith-gateway
sudo systemctl start zenith-gateway
```

**Step 4: Setup Load Balancer (30 min)**
```bash
# On nginx load balancer:

sudo tee /etc/nginx/sites-available/zenith > /dev/null <<'EOF'
upstream zenith_backend {
    server 10.0.0.2:8000 weight=1;
    server 10.0.0.3:8000 weight=1;
    keepalive 32;
}

server {
    listen 80;
    server_name api.zenith.network;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name api.zenith.network;

    ssl_certificate /etc/letsencrypt/live/api.zenith.network/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.zenith.network/privkey.pem;
    ssl_protocols TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api:10m rate=100r/s;
    limit_req zone=api burst=10 nodelay;

    location / {
        proxy_pass http://zenith_backend;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
}
EOF

sudo ln -s /etc/nginx/sites-available/zenith /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl restart nginx
```

**Step 5: Verify Deployment (1 hour)**
```bash
# Test via load balancer
curl https://api.zenith.network/health

# Monitor logs
journalctl -u zenith-validator -f
journalctl -u zenith-gateway -f

# Check metrics
curl https://api.zenith.network/v1/metrics | head -20

# Run smoke tests
k6 run tests/smoke-test.js

# Monitor for 1 hour
watch -n 5 'curl -s https://api.zenith.network/health | jq .'
```

**Step 6: Announce Launch**
```bash
# Post to social media / blog:
"🚀 Zenith Mainnet is LIVE!

Deploy your first canister:
curl -X POST https://api.zenith.network/v1/canisters ...

Documentation: https://zenith.network/docs
Status Page: https://status.zenith.network
Community: https://discord.gg/zenith"
```

### 7.3 Rollback Plan

**If something goes wrong:**
```bash
# Immediate action (first 5 min):
# Stop traffic to problematic gateway
sudo systemctl stop zenith-gateway

# Revert to last known good build
git checkout v1.0.0  # Last release tag
cargo build --release -p zenith-gateway

# Restart
sudo systemctl start zenith-gateway

# If still broken:
# Failover to secondary gateway
# (Already configured in load balancer)

# Notify team + users
# Incident = data corruption or consensus fork (rare)
```

**Exercise 7.3:** Practice deployment steps in staging environment.

---

## Module 8: Final Quiz & Certification

### 8.1 Knowledge Check

**Question 1:** What are the 5 layers of Zenith?
```
Answer: User APIs → Execution Engine → ZK Provers → Blockchain → Network & Storage
```

**Question 2:** How do you check if a validator is producing blocks?
```bash
# Answer:
curl -s http://localhost:9944 ... | jq .result.number
# Block number should increment every ~6 seconds
```

**Question 3:** What's the procedure if gateway memory keeps growing?
```
Answer: 
1. Check memory trend (multiple ps commands)
2. If >4GB, restart gateway: pkill -f zenith-gateway
3. If still growing, escalate to senior engineer
```

**Question 4:** How would you backup the database?
```bash
# Answer:
pkill -f zenith-gateway
tar -czf gateway-data-backup-$(date +%Y-%m-%d).tar.gz gateway-data
# Start gateway
```

**Question 5:** What's the first step in incident response?
```
Answer: Page on-call engineer and create incident Slack channel
```

### 8.2 Hands-On Exercises

Complete all of these to earn certification:

- [ ] **Exercise 1:** Build node binary, start validator, produce 10 blocks
- [ ] **Exercise 2:** Build gateway, deploy canister, call it successfully
- [ ] **Exercise 3:** Create database backup, verify it restores
- [ ] **Exercise 4:** Simulate incident (kill gateway), diagnose, fix
- [ ] **Exercise 5:** Setup Prometheus, query metrics, test alerts
- [ ] **Exercise 6:** Stress test (load tool), monitor response

### 8.3 Certification

```
ZENITH OPERATIONS CERTIFICATION

Name: _______________________
Date: ______________________

You have completed:
[✓] Module 1: Architecture (30 min)
[✓] Module 2: Validator Node (1 hour)
[✓] Module 3: Gateway Operations (1 hour)
[✓] Module 4: Database Operations (45 min)
[✓] Module 5: Incident Response (1.5 hours)
[✓] Module 6: Monitoring & Alerting (45 min)
[✓] Module 7: Production Deployment (1 hour)
[✓] Module 8: Final Quiz & Exercises (1 hour)

Total Training: 8 hours

This certifies that the above individual is qualified to operate
Zenith mainnet infrastructure.

Authorized by: ______________________
Title: Operations Lead
Date: ______________________
```

---

## Appendix A: Quick Reference

**Start Gateway:**
```bash
/path/to/gateway --listen 0.0.0.0:8000 --node-rpc ws://localhost:9944
```

**Check Health:**
```bash
curl http://localhost:8000/health
```

**View Metrics:**
```bash
curl http://localhost:8000/v1/metrics
```

**Restart Service:**
```bash
systemctl restart zenith-gateway
```

**View Logs:**
```bash
journalctl -u zenith-gateway -f
```

**Backup Database:**
```bash
tar -czf backup-$(date +%Y-%m-%d).tar.gz gateway-data
```

---

## Appendix B: Contacts & Escalation

**On-Call Schedule:**
- Primary: Alice Chen (alice@zenith.network)
- Secondary: Bob Smith (bob@zenith.network)
- Manager: Charlie Davis (charlie@zenith.network)

**Escalation Levels:**
- P1: Page primary immediately
- P2: Contact within 15 minutes
- P3: Contact within 1 hour
- P4: Handle during business hours

---

**Next Step:** Complete exercises and earn certification.
