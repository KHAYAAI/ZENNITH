# Zenith Platform - Production Deployment Guide

## Overview

This guide provides step-by-step instructions for deploying Zenith to production with full hardening, monitoring, and backup capabilities.

---

## Infrastructure Setup (2 hours)

### Prerequisites

**System Requirements:**
- Ubuntu 20.04 LTS or later
- 4+ CPU cores, 16GB+ RAM, 100GB+ SSD
- Static IP address
- Domain name

**Install Core Dependencies:**
```bash
sudo apt-get update
sudo apt-get install -y nginx certbot python3-certbot-nginx k6
```

### TLS/HTTPS Configuration (1 hour)

**1. Copy nginx config:**
```bash
sudo cp deployment/nginx.conf /etc/nginx/sites-available/zenith
sudo ln -s /etc/nginx/sites-available/zenith /etc/nginx/sites-enabled/
```

**2. Setup Let's Encrypt:**
```bash
sudo certbot certonly --nginx -d api.zenith.io
```

**3. Reload nginx:**
```bash
sudo nginx -t && sudo systemctl reload nginx
```

**4. Verify HTTPS:**
```bash
curl -I https://api.zenith.io/health  # Should return 200 OK
```

---

## Backup & Recovery Setup (30 min)

### Automated Backups

**1. Install backup script:**
```bash
sudo cp deployment/backup.sh /usr/local/bin/zenith-backup
sudo chmod +x /usr/local/bin/zenith-backup
```

**2. Schedule daily backups (crontab):**
```bash
0 2 * * * /usr/local/bin/zenith-backup backup >> /var/log/zenith-backup.log 2>&1
```

**3. Test backup:**
```bash
/usr/local/bin/zenith-backup backup
/usr/local/bin/zenith-backup verify
```

**4. Restore procedure:**
```bash
/usr/local/bin/zenith-backup restore <backup_date>
systemctl restart zenith-gateway
```

---

## Monitoring Setup (1 hour)

### Prometheus & Grafana

**1. Start Prometheus:**
```bash
/opt/prometheus/prometheus --config.file=/opt/prometheus/prometheus.yml
```

**2. Start Grafana:**
```bash
sudo systemctl start grafana-server
# Access: http://localhost:3000 (admin/admin)
```

**3. Add Prometheus datasource in Grafana:**
- Settings → Data Sources → Add Prometheus
- URL: http://localhost:9090
- Save & Test

**4. Import dashboard:**
- Dashboards → Import
- Upload: deployment/grafana-dashboard.json
- Select datasource: Prometheus

**5. View dashboard:** http://localhost:3000/d/zenith-production

---

## Load Testing & Baselines (1 hour)

### Performance Testing

**1. Generate auth token:**
```bash
export TOKEN=$(cargo run --quiet -p zenith-cli -- auth generate --role deployer --exp 3600)
```

**2. Run load test:**
```bash
k6 run tests/load_test.js \
  --vus 50 \
  --duration 5m \
  --env BASE_URL=http://localhost:8000 \
  --env AUTH_TOKEN="$TOKEN"
```

**3. Expected Results (SLA Targets):**
- Deployment latency p95: < 500ms ✓
- Call latency p95: < 200ms ✓
- Error rate: < 10% ✓
- Throughput: 100+ ops/sec ✓

**4. Document baselines:**
```bash
k6 run tests/load_test.js --out csv=baseline-$(date +%Y%m%d).csv
```

---

## Security Hardening

### Environment Variables

**Set JWT secret in systemd service:**
```bash
sudo systemctl edit zenith-gateway
# Add: Environment="JWT_SECRET=your-secret-key-here"
```

### Firewall Configuration

```bash
sudo ufw default deny incoming
sudo ufw allow 22/tcp        # SSH
sudo ufw allow 80/tcp        # HTTP (Let's Encrypt)
sudo ufw allow 443/tcp       # HTTPS
sudo ufw allow 127.0.0.1:9090/tcp   # Prometheus (local only)
sudo ufw allow 127.0.0.1:3000/tcp   # Grafana (local only)
sudo ufw enable
```

---

## Health Checks & Monitoring

### Daily Operations

**Check gateway health:**
```bash
curl -I https://api.zenith.io/health
systemctl status zenith-gateway
```

**View audit logs:**
```bash
tail -f /var/lib/zenith/gateway-data/audit.log
```

**Monitor performance:**
```bash
# Baseline comparison (weekly)
k6 run tests/load_test.js --vus 50 --duration 5m
```

### Alert Thresholds (Set in Grafana)

- Error rate > 1% for 5 min → Warning
- Error rate > 5% for 5 min → Critical
- p95 latency > 1000ms for 10 min → Warning
- Disk usage > 80% → Warning

---

## Emergency Procedures

### High Error Rate

**Diagnosis:**
```bash
journalctl -u zenith-gateway -p err -n 100
curl http://localhost:8000/v1/metrics | grep zenith_errors
```

**Recovery:**
```bash
systemctl restart zenith-gateway
```

### Disk Full

**Solution:**
```bash
/usr/local/bin/zenith-backup backup
du -sh /var/lib/zenith/gateway-data/*
find /var/lib/zenith/gateway-data -mtime +90 -delete
df -h
```

### SSL Certificate Expiring

**Check & renew:**
```bash
certbot certificates
sudo certbot renew --force-renewal
sudo systemctl reload nginx
```

### Complete Rollback

**Procedure:**
```bash
sudo systemctl stop zenith-gateway
/usr/local/bin/zenith-backup restore <recent_backup_date>
sudo systemctl start zenith-gateway
curl https://api.zenith.io/health
```

---

## Success Criteria

Production deployment is complete when:

- ✅ HTTPS endpoint responds with valid certificate (A+ rating)
- ✅ All load test SLA thresholds met
- ✅ Monitoring dashboard shows all metrics
- ✅ Daily backups automated and verified
- ✅ Audit logging enabled for all operations
- ✅ 24+ hours uptime without errors
- ✅ Firewall rules configured
- ✅ JWT secret in environment variables

---

## Support & Escalation

**Check these logs/dashboards:**
1. `/var/log/nginx/zenith_*.log` - Reverse proxy logs
2. `journalctl -u zenith-gateway` - Service logs
3. `/var/lib/zenith/gateway-data/audit.log` - Audit trail
4. `http://localhost:9090` - Prometheus metrics
5. `http://localhost:3000` - Grafana dashboard

**Troubleshooting steps:**
1. Check service status: `systemctl status zenith-gateway`
2. View recent logs: `journalctl -u zenith-gateway -n 50 -f`
3. Test health endpoint: `curl https://api.zenith.io/health`
4. Verify database: `/usr/local/bin/zenith-backup verify`
5. Review audit trail: `tail -100 /var/lib/zenith/gateway-data/audit.log`
