# Zenith DevOps Scripts & Automation

This directory contains production-ready scripts for operating Zenith mainnet. All scripts are designed for 24/7 reliability with minimal manual intervention.

## Quick Start (5 minutes)

### 1. Initialize Infrastructure (Once)

```bash
# 1. Copy environment template and customize
cp deployment/config/env-template.sh /etc/zenith/validator.env
cp deployment/config/env-template.sh /etc/zenith/gateway.env
nano /etc/zenith/validator.env  # Edit with your values

# 2. Setup monitoring stack (Prometheus, Grafana, AlertManager)
chmod +x deployment/scripts/monitoring-setup.sh
sudo ./deployment/scripts/monitoring-setup.sh

# 3. Setup cron jobs for automated backups
chmod +x deployment/scripts/cron-setup.sh
sudo ./deployment/scripts/cron-setup.sh

# 4. Copy systemd service files
sudo cp deployment/systemd/*.service /etc/systemd/system/
sudo systemctl daemon-reload

# 5. Copy deployment script and make executable
chmod +x deployment/scripts/*.sh
sudo cp deployment/scripts/*.sh /opt/zenith/scripts/
```

### 2. Deploy a Release (Minutes)

```bash
# Build binaries
cargo build --release -p zenith-node -p zenith-gateway

# Run rolling deployment (safe)
./deployment/scripts/deploy.sh v1.2.0

# For dry-run first:
./deployment/scripts/deploy.sh v1.2.0 --dry-run

# With auto-fix for common issues:
./deployment/scripts/deploy.sh v1.2.0 --force
```

### 3. Check System Health (Anytime)

```bash
# Full incident diagnosis
./deployment/scripts/incident-response.sh

# With auto-fixes
./deployment/scripts/incident-response.sh --auto-fix

# View monitoring dashboards
# - Prometheus: http://localhost:9090
# - Grafana: http://localhost:3000
```

---

## Scripts Overview

### 1. `backup.sh` - Daily Automated Backups

**Purpose:** Backs up all critical state (validators, gateways, provers, configs)

**Schedule:** Daily at 2:00 AM (configured via cron)

**What it backs up:**
- ✅ Validator blockchain databases
- ✅ Gateway sled caches
- ✅ Prover caches
- ✅ Configuration files
- ✅ Systemd service definitions

**Storage:**
- Encrypted at rest (AES-256)
- Stored in AWS S3 with GLACIER archival
- 30-day retention on-disk, 7 years in Glacier

**Recovery:**
```bash
# All backups are tested automatically
# To manually restore:
ssh validator-1
aws s3 cp s3://zenith-backups/validators/validator-1/db-2024-12-01.tar.gz /tmp/
tar -xzf /tmp/db-2024-12-01.tar.gz -C /data/validator/
systemctl restart zenith-validator
```

**Monitoring:**
- Logs saved to `/var/log/zenith/backup.log`
- Prometheus metric: `zenith_backup_total_size_bytes`
- Alert if backup fails: `BackupFailed`

---

### 2. `deploy.sh` - Safe Rolling Deployment

**Purpose:** Deploy new versions without downtime or data loss

**Strategy:** Multi-phase rolling updates with consensus safety

**Phases:**
1. RPC Sentinel (5 min) - Lightweight monitoring node, low risk
2. Archive Node (5 min) - Full history node, no consensus participation
3. Validators (2-4 min) - **CRITICAL PHASE**, staggered 1-by-1
4. Gateways (2 min) - Parallel deployment, stateless
5. Provers (2 min) - Parallel deployment, async processing

**Usage:**

```bash
# Dry-run (see what would happen)
./deploy.sh v1.2.0 --dry-run

# Real deployment
./deploy.sh v1.2.0

# Force deployment even if pre-checks fail
./deploy.sh v1.2.0 --force
```

**Safety Features:**
- Pre-deployment checks (all nodes reachable, block height correct)
- Automatic backups before each deployment
- Consensus monitoring (finality must resume after each validator)
- Automatic rollback on critical failure
- 30-second wait between validator deployments

**Monitoring:**
```bash
# Watch deployment progress
tail -f /var/log/zenith/deployment.log

# View deployed version
zenith-node --version
```

**Rollback (if needed):**
```bash
# Automatic rollback on failure, or manual:
for node in validator-{1..5} gateway-{1..3} prover-{1..3}; do
  ssh $node "sudo systemctl stop zenith-*"
  ssh $node "tar -xzf /data/backup/db-pre-deploy-*.tar.gz -C /data/"
  ssh $node "sudo systemctl start zenith-*"
done
```

---

### 3. `monitoring-setup.sh` - Deploy Monitoring Stack

**Purpose:** Install Prometheus, Grafana, AlertManager, and node_exporter

**Installs:**
- ✅ Prometheus (metrics collection)
- ✅ AlertManager (alert routing to Slack/PagerDuty)
- ✅ Grafana (dashboards & visualization)
- ✅ node_exporter (system metrics on all nodes)

**Dashboards:**
1. **Validator Health** - Block production, finality, peer count, validator participation
2. **Gateway Performance** - Request rate, latency p95/p99, error rate, proof generation time
3. **Resource Utilization** - CPU, memory, disk, network per node

**Alerting Rules:**
- CRITICAL: Block production stalled, all gateways down, disk full
- HIGH: Single validator down, high latency, high error rate
- MEDIUM: Rate limit violations, prover queue backlog, TLS cert expiring

**Configuration:**

```bash
# Edit alert routing (Slack/PagerDuty)
sudo nano /etc/prometheus/alertmanager.yml
  # Replace YOUR_SLACK_WEBHOOK and YOUR_PAGERDUTY_KEY

# Add custom alerts
sudo nano /etc/prometheus/alert-rules.yml

# Reload Prometheus
curl -X POST http://localhost:9090/-/reload
```

**Accessing Dashboards:**

| Service | URL | Default Login |
|---------|-----|---|
| Prometheus | http://localhost:9090 | None (no auth) |
| Grafana | http://localhost:3000 | admin/admin |
| AlertManager | http://localhost:9093 | None (no auth) |

**Import Dashboards:**
```bash
# Pre-built dashboards available in /deployment/grafana/
# Import via Grafana UI: Dashboard → Import JSON
```

---

### 4. `incident-response.sh` - Automated Diagnosis

**Purpose:** Quickly diagnose system health and suggest fixes

**Checks Performed:**

| Check | Details |
|-------|---------|
| **Consensus** | Block production, finality, peer count |
| **Validators** | Service status, reachability, logs |
| **Gateways** | API responses, latency, error rate |
| **Resources** | CPU, memory, disk space per node |
| **Provers** | Service status, proof generation latency |
| **Database** | sled size, blockchain state size |
| **Network** | RPC connectivity, inter-node reachability |
| **Logs** | Recent errors in validator/gateway logs |

**Usage:**

```bash
# Full diagnosis (no changes)
./incident-response.sh

# Auto-fix common issues (restarts services, clears caches)
./incident-response.sh --auto-fix

# View report
cat /tmp/incident-report-*.md
```

**Sample Output:**

```markdown
# Zenith Incident Response Report
Generated: 2024-12-01 14:30:00

## Incident Diagnosis

### 1. Consensus Health
✅ Current block height: 1523450
✅ Block production working (1523450 → 1523451)
✅ Finality is working
✅ Peer count: 8

### 2. Validator Node Status
✅ validator-1: running
✅ validator-2: running
❌ validator-3: stopped or unreachable
✅ validator-4: running
✅ validator-5: running

Summary: 4/5 validators healthy

## Recommendations

🚨 CRITICAL ISSUES DETECTED

1. Validator Down
   - Check if validator-3 process crashed
   - SSH into validator-3 and run: systemctl status zenith-validator
   - Run: deployment/scripts/incident-response.sh --auto-fix
```

**Integration with On-Call:**
```bash
# Run hourly via cron (configured by cron-setup.sh)
0 * * * * /opt/zenith/scripts/incident-response.sh | mail -s "Zenith Status" oncall@zenith.network

# Or pipe to monitoring system
0 * * * * /opt/zenith/scripts/incident-response.sh | \
  curl -X POST -d @- http://monitoring.internal/webhooks/zenith
```

---

### 5. Systemd Service Files

**Files:**
- `zenith-validator.service` - Validator node
- `zenith-gateway.service` - Gateway API
- `zenith-prover.service` - Prover (if separate)

**Features:**
- Automatic restart on failure
- Resource limits (memory, CPU)
- Security hardening (ProtectSystem, ProtectHome)
- Health checks
- Structured logging

**Usage:**

```bash
# Start/stop/restart
sudo systemctl start zenith-validator
sudo systemctl stop zenith-gateway
sudo systemctl restart zenith-prover

# Check status
sudo systemctl status zenith-validator

# View logs
sudo journalctl -u zenith-validator -f

# Enable on boot
sudo systemctl enable zenith-validator
```

**Customization:**

Edit `/etc/zenith/validator.env` to customize:
- `ZENITH_VALIDATOR_NAME` - Node identity
- `ZENITH_VALIDATOR_BOND` - Stake amount
- Resource limits - `MemoryMax`, `CPUQuota`

---

### 6. Nginx TLS Reverse Proxy

**File:** `nginx-tls.conf`

**Features:**
- ✅ TLS termination (HTTPS)
- ✅ Load balancing (least connections)
- ✅ Rate limiting (global + per-IP)
- ✅ Health checks (auto remove unhealthy backends)
- ✅ Caching (5-minute cache for GET requests)
- ✅ Compression (gzip)
- ✅ Security headers

**Setup:**

```bash
# Install nginx
sudo apt-get install nginx

# Copy configuration
sudo cp deployment/nginx/nginx-tls.conf /etc/nginx/nginx.conf

# Get TLS certificate (Let's Encrypt)
sudo certbot certonly --standalone -d zenith.network

# Start nginx
sudo systemctl start nginx
sudo systemctl enable nginx
```

**Backend Status:**

```bash
# Check which backends are active
curl http://localhost:8080/nginx_status

# Monitor in Grafana
# Dashboard: Nginx Monitoring
```

---

### 7. Environment Configuration

**File:** `config/env-template.sh`

**Purpose:** Centralized configuration for all components

**Key Variables:**

| Variable | Default | Rotation |
|----------|---------|----------|
| `ZENITH_VALIDATOR_SESSION_KEY` | - | Weekly |
| `ZENITH_GATEWAY_JWT_SECRET` | - | Monthly |
| `ZENITH_GATEWAY_DATABASE_ENCRYPTION_KEY` | - | Quarterly |
| `ZENITH_VALIDATOR_STASH_KEY` | - | Annually |

**Deployment:**

```bash
# Copy template
cp deployment/config/env-template.sh /etc/zenith/validator.env

# Edit with actual values (NEVER commit these!)
nano /etc/zenith/validator.env

# Add to systemd service (already done in zenith-validator.service)
EnvironmentFile=/etc/zenith/validator.env

# Verify (no secrets logged)
systemctl show -p EnvironmentFiles zenith-validator
```

---

## Operational Procedures

### Daily Operations

```bash
# Morning: Check system health
./deployment/scripts/incident-response.sh

# Weekly: Rotate session keys
subkey generate  # Get new key
# Update ZENITH_VALIDATOR_SESSION_KEY in /etc/zenith/validator.env
sudo systemctl restart zenith-validator

# Monthly: Rotate JWT secrets
export ZENITH_GATEWAY_JWT_SECRET=$(openssl rand -hex 32)
sudo systemctl restart zenith-gateway

# Quarterly: Rotate database encryption
# Requires manual intervention - see disaster recovery docs
```

### Deployment Procedure

```bash
# 1. Build release
cargo build --release

# 2. Dry-run deployment
./deployment/scripts/deploy.sh v1.2.0 --dry-run

# 3. Actual deployment
./deployment/scripts/deploy.sh v1.2.0

# 4. Monitor for 10 minutes
watch -n 5 'curl -s http://localhost:9944 ... | jq .result.number'

# 5. Verify all services up
for svc in validator gateway prover; do
  sudo systemctl status zenith-$svc | grep -q active && echo "✓ $svc" || echo "✗ $svc"
done
```

### Incident Response

```bash
# 1. Get immediate diagnosis
./deployment/scripts/incident-response.sh

# 2. Review critical findings
less /tmp/incident-report-*.md

# 3. Auto-fix if safe
./deployment/scripts/incident-response.sh --auto-fix

# 4. Monitor resolution
tail -f /var/log/zenith/deployment.log

# 5. Document incident
# Create incident report, store in /var/log/zenith/incidents/
```

### Backup & Recovery

```bash
# Backups run automatically daily at 2am
# To manually trigger:
./deployment/scripts/backup.sh

# To verify recent backups
aws s3 ls s3://zenith-backups/ --recursive | tail -10

# To restore a validator
# See backup.sh for detailed procedure
```

---

## Troubleshooting

### "Block production stalled"

```bash
# 1. Check validator status
for v in validator-{1..5}; do
  ssh $v "systemctl status zenith-validator" | grep -E "active|failed"
done

# 2. Check peer connectivity
ssh validator-1 "for v in validator-{2..5}; do ping -c 1 $v; done"

# 3. Restart validators (staggered)
for v in validator-{1..5}; do
  ssh $v "sudo systemctl restart zenith-validator"
  sleep 30  # Wait for consensus to resume
done

# 4. Verify finality
curl -s http://localhost:9944 ... | jq '.result.number'  # Should increase
```

### "Gateway returning 500 errors"

```bash
# 1. Check RPC connectivity
ssh gateway-1 "curl -s http://validator-1:9944 ... | jq ."

# 2. Check gateway logs for errors
ssh gateway-1 "tail -100 /var/log/zenith/gateway.log | grep -i error"

# 3. Restart gateways
for g in gateway-{1..3}; do
  ssh $g "sudo systemctl restart zenith-gateway"
  sleep 5
done

# 4. Verify API responding
curl http://localhost:8000/health
```

### "Out of disk space"

```bash
# 1. Check disk usage
ssh validator-1 "df -h /"

# 2. Find large files/directories
ssh validator-1 "du -sh /data/validator/*"

# 3. Clean old backups (if on same disk)
ssh validator-1 "rm -rf /data/backup/db-pre-deploy-*.tar.gz"

# 4. Increase disk size (cloud provider)
# AWS: Extend EBS volume
# GCP: Resize persistent disk
# Then resize filesystem: sudo resize2fs /dev/xvdf
```

---

## Monitoring & Alerting

### Prometheus Metrics

Access at `http://localhost:9090`

**Key Metrics:**
```
# Block production
substrate_blockchain_height

# Finality
substrate_finalized_height

# Peer count
substrate_peers_total

# API latency
zenith_gateway_request_duration_seconds

# Error rate
zenith_gateway_requests_error_total

# Proof generation
zenith_prover_proof_generation_time_seconds

# System
node_cpu_seconds_total
node_memory_MemAvailable_bytes
```

### Grafana Dashboards

Access at `http://localhost:3000` (admin/admin)

**Pre-built Dashboards:**
1. Validator Health (block production, finality, peers)
2. Gateway Performance (latency, errors, rate limits)
3. Resource Utilization (CPU, memory, disk)
4. Prover Status (queue depth, latency, utilization)

### Alerts

Alert rules defined in `/etc/prometheus/alert-rules.yml`

**Severity Levels:**
- 🔴 **CRITICAL** - Page on-call immediately (PagerDuty)
- 🟠 **HIGH** - Slack notification + page in 15 min
- 🟡 **MEDIUM** - Slack notification only

**Integration:**
```bash
# Edit AlertManager config for Slack/PagerDuty
sudo nano /etc/prometheus/alertmanager.yml

# Test alert
curl -X POST http://localhost:9093/api/v1/alerts \
  -H 'Content-Type: application/json' \
  -d '[{"labels":{"alertname":"TestAlert","severity":"critical"}}]'
```

---

## File Structure

```
deployment/
├── scripts/
│   ├── backup.sh                 # Daily backups
│   ├── deploy.sh                 # Safe rolling deployment
│   ├── monitoring-setup.sh       # Prometheus + Grafana setup
│   ├── incident-response.sh      # Automated diagnosis
│   ├── cron-setup.sh             # Configure cron jobs
│   ├── verify-backup.sh          # Test backup restore (weekly)
│   └── rotate-logs.sh            # Log rotation
├── systemd/
│   ├── zenith-validator.service  # Validator service
│   ├── zenith-gateway.service    # Gateway service
│   └── zenith-prover.service     # Prover service (optional)
├── nginx/
│   └── nginx-tls.conf            # TLS reverse proxy
├── config/
│   ├── env-template.sh           # Configuration template
│   └── alerting-rules.yml        # Prometheus alerts
├── grafana/
│   ├── dashboard-validator.json  # Validator dashboard
│   ├── dashboard-gateway.json    # Gateway dashboard
│   └── datasources.yml           # Prometheus datasource
└── DEVOPS_SCRIPTS_README.md      # This file
```

---

## Support & Escalation

**On-Call Contact:**
- Primary: oncall@zenith.network
- PagerDuty: https://zenith.pagerduty.com
- Slack: #incidents channel

**Documentation:**
- All runbooks in `/deployment/`
- Architecture docs in `/zenith/`
- Incident reports in `/var/log/zenith/incidents/`

**Getting Help:**
```bash
# Manual on any script
./deployment/scripts/deploy.sh --help

# View recent incidents
ls -ltr /tmp/incident-report-*.md | tail -5

# Search logs
grep -r "ERROR" /var/log/zenith/ | tail -20
```

---

**Last Updated:** December 1, 2024  
**Zenith Version:** 1.2.0  
**Maintained By:** DevOps Team
