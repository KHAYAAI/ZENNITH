#!/bin/bash
# Zenith Monitoring Stack Setup
# Installs and configures Prometheus, Grafana, AlertManager, and node_exporter
# Run once during infrastructure setup

set -euo pipefail

PROMETHEUS_VERSION="2.48.0"
GRAFANA_VERSION="10.2.0"
ALERTMANAGER_VERSION="0.26.0"
NODE_EXPORTER_VERSION="1.7.0"

LOG_FILE="/var/log/zenith/monitoring-setup.log"
DATETIME=$(date '+%Y-%m-%d %H:%M:%S')

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${BLUE}${DATETIME} [INFO]${NC} $1" | tee -a "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}${DATETIME} [OK]${NC} $1" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}${DATETIME} [ERROR]${NC} $1" | tee -a "$LOG_FILE"
}

log "=========================================="
log "Zenith Monitoring Stack Setup"
log "=========================================="

# Create monitoring user
log "Creating monitoring user..."
sudo useradd --no-create-home --shell /bin/false prometheus || log "prometheus user already exists"
sudo useradd --no-create-home --shell /bin/false grafana || log "grafana user already exists"

# Create directories
log "Creating directories..."
sudo mkdir -p /etc/prometheus
sudo mkdir -p /var/lib/prometheus
sudo mkdir -p /etc/grafana
sudo mkdir -p /var/lib/grafana

sudo chown prometheus:prometheus /etc/prometheus
sudo chown prometheus:prometheus /var/lib/prometheus
sudo chown grafana:grafana /var/lib/grafana

# Install node_exporter on all nodes
log "Installing node_exporter..."
cd /tmp
wget -q https://github.com/prometheus/node_exporter/releases/download/v${NODE_EXPORTER_VERSION}/node_exporter-${NODE_EXPORTER_VERSION}.linux-amd64.tar.gz
tar -xzf node_exporter-${NODE_EXPORTER_VERSION}.linux-amd64.tar.gz
sudo cp node_exporter-${NODE_EXPORTER_VERSION}.linux-amd64/node_exporter /usr/local/bin/
sudo chmod +x /usr/local/bin/node_exporter
log_success "node_exporter installed"

# Create node_exporter systemd service
sudo tee /etc/systemd/system/node_exporter.service > /dev/null <<EOF
[Unit]
Description=Prometheus Node Exporter
After=network.target

[Service]
User=prometheus
Group=prometheus
Type=simple
ExecStart=/usr/local/bin/node_exporter \
  --collector.filesystem.mount-points-exclude=^/(sys|proc|dev|host|etc)(\$|/) \
  --collector.netdev.device-exclude=^(veth.*)$

Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable node_exporter
sudo systemctl start node_exporter
log_success "node_exporter service created and started"

# Install Prometheus
log "Installing Prometheus..."
cd /tmp
wget -q https://github.com/prometheus/prometheus/releases/download/v${PROMETHEUS_VERSION}/prometheus-${PROMETHEUS_VERSION}.linux-amd64.tar.gz
tar -xzf prometheus-${PROMETHEUS_VERSION}.linux-amd64.tar.gz

sudo cp prometheus-${PROMETHEUS_VERSION}.linux-amd64/prometheus /usr/local/bin/
sudo cp prometheus-${PROMETHEUS_VERSION}.linux-amd64/promtool /usr/local/bin/
sudo chmod +x /usr/local/bin/prometheus /usr/local/bin/promtool

sudo cp prometheus-${PROMETHEUS_VERSION}.linux-amd64/consoles -r /etc/prometheus
sudo cp prometheus-${PROMETHEUS_VERSION}.linux-amd64/console_libraries -r /etc/prometheus
sudo chown -R prometheus:prometheus /etc/prometheus

log_success "Prometheus installed"

# Create Prometheus configuration
log "Creating Prometheus configuration..."
sudo tee /etc/prometheus/prometheus.yml > /dev/null <<'EOF'
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  external_labels:
    cluster: 'zenith-mainnet'
    environment: 'production'

# Alertmanager configuration
alerting:
  alertmanagers:
    - static_configs:
        - targets:
            - 'localhost:9093'

# Load alerting rules
rule_files:
  - '/etc/prometheus/alert-rules.yml'

scrape_configs:
  # Prometheus itself
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  # Node exporter (system metrics)
  - job_name: 'nodes'
    static_configs:
      - targets:
          - 'validator-1:9100'
          - 'validator-2:9100'
          - 'validator-3:9100'
          - 'validator-4:9100'
          - 'validator-5:9100'
          - 'gateway-1:9100'
          - 'gateway-2:9100'
          - 'gateway-3:9100'
          - 'prover-plonk:9100'
          - 'prover-cairo:9100'
          - 'prover-risc0:9100'
          - 'archive-1:9100'
          - 'rpc-sentinel-1:9100'
    relabel_configs:
      - source_labels: [__address__]
        regex: '([^:]+)(?::\d+)?'
        target_label: instance

  # Validator metrics (via RPC)
  - job_name: 'validators'
    metrics_path: '/metrics'
    static_configs:
      - targets:
          - 'validator-1:9090'
          - 'validator-2:9090'
          - 'validator-3:9090'
          - 'validator-4:9090'
          - 'validator-5:9090'

  # Gateway metrics
  - job_name: 'gateways'
    metrics_path: '/metrics'
    static_configs:
      - targets:
          - 'gateway-1:9000'
          - 'gateway-2:9000'
          - 'gateway-3:9000'

  # Prover metrics
  - job_name: 'provers'
    metrics_path: '/metrics'
    static_configs:
      - targets:
          - 'prover-plonk:9001'
          - 'prover-cairo:9001'
          - 'prover-risc0:9001'

  # Archive node
  - job_name: 'archive'
    metrics_path: '/metrics'
    static_configs:
      - targets:
          - 'archive-1:9090'
EOF

sudo chown prometheus:prometheus /etc/prometheus/prometheus.yml

log_success "Prometheus configuration created"

# Create alerting rules
log "Creating alerting rules..."
sudo tee /etc/prometheus/alert-rules.yml > /dev/null <<'EOF'
groups:
  - name: zenith-alerts
    interval: 30s
    rules:
      # Validator alerts
      - alert: BlockProductionStalled
        expr: increase(substrate_blockchain_height[5m]) == 0
        for: 2m
        annotations:
          summary: "Block production stalled"
          description: "No new blocks in the last 5 minutes"
          severity: "critical"

      - alert: FinalityStalled
        expr: increase(substrate_finalized_height[10m]) == 0
        for: 5m
        annotations:
          summary: "Finality stalled"
          description: "No finalized blocks in the last 10 minutes"
          severity: "critical"

      - alert: ValidatorDown
        expr: up{job="validators"} == 0
        for: 2m
        annotations:
          summary: "Validator {{ $labels.instance }} is down"
          description: "Validator has been unavailable for 2 minutes"
          severity: "critical"

      - alert: LowPeerCount
        expr: substrate_peers_total < 3
        for: 2m
        annotations:
          summary: "Low peer count"
          description: "Connected peers < 3, possible network issue"
          severity: "high"

      # Gateway alerts
      - alert: GatewayHighErrorRate
        expr: rate(zenith_gateway_requests_error_total[5m]) > 0.05
        for: 2m
        annotations:
          summary: "Gateway high error rate"
          description: "Error rate > 5% on {{ $labels.instance }}"
          severity: "high"

      - alert: GatewayHighLatency
        expr: histogram_quantile(0.95, zenith_gateway_request_duration_seconds) > 0.5
        for: 5m
        annotations:
          summary: "Gateway high latency (p95 > 500ms)"
          description: "Request latency on {{ $labels.instance }} is degraded"
          severity: "high"

      - alert: GatewayDown
        expr: up{job="gateways"} == 0
        for: 1m
        annotations:
          summary: "Gateway {{ $labels.instance }} is down"
          description: "Gateway has been unavailable for 1 minute"
          severity: "critical"

      - alert: RateLimitExceeded
        expr: rate(zenith_gateway_rate_limit_exceeded_total[5m]) > 1
        for: 2m
        annotations:
          summary: "High rate limit violations"
          description: "Rate limit violations > 1/sec, possible DDoS"
          severity: "high"

      # System alerts
      - alert: HighCPUUsage
        expr: (100 - (avg by (instance) (rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)) > 80
        for: 5m
        annotations:
          summary: "High CPU usage on {{ $labels.instance }}"
          description: "CPU usage > 80% for 5 minutes"
          severity: "high"

      - alert: HighMemoryUsage
        expr: (1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes)) * 100 > 90
        for: 5m
        annotations:
          summary: "High memory usage on {{ $labels.instance }}"
          description: "Memory usage > 90%"
          severity: "high"

      - alert: LowDiskSpace
        expr: node_filesystem_avail_bytes{mountpoint="/"} / node_filesystem_size_bytes < 0.1
        for: 5m
        annotations:
          summary: "Low disk space on {{ $labels.instance }}"
          description: "Disk space < 10%"
          severity: "high"

      - alert: DiskFull
        expr: node_filesystem_avail_bytes{mountpoint="/"} / node_filesystem_size_bytes < 0.05
        for: 2m
        annotations:
          summary: "Disk nearly full on {{ $labels.instance }}"
          description: "Disk space < 5%"
          severity: "critical"

      - alert: NodeDown
        expr: up{job="nodes"} == 0
        for: 1m
        annotations:
          summary: "Node {{ $labels.instance }} is down"
          description: "Node is unreachable for 1 minute"
          severity: "critical"

      # Prover alerts
      - alert: ProverQueueBacklog
        expr: zenith_prover_queue_depth > 100
        for: 5m
        annotations:
          summary: "Prover queue backlog"
          description: "{{ $labels.instance }} has {{ $value }} pending proofs"
          severity: "high"

      - alert: ProofGenerationTimeout
        expr: histogram_quantile(0.95, zenith_prover_proof_generation_time_seconds) > 30
        for: 5m
        annotations:
          summary: "Slow proof generation"
          description: "{{ $labels.instance }} p95 latency > 30s"
          severity: "high"
EOF

sudo chown prometheus:prometheus /etc/prometheus/alert-rules.yml
log_success "Alerting rules created"

# Create Prometheus systemd service
sudo tee /etc/systemd/system/prometheus.service > /dev/null <<EOF
[Unit]
Description=Prometheus
Wants=network-online.target
After=network-online.target

[Service]
User=prometheus
Group=prometheus
Type=simple
ExecStart=/usr/local/bin/prometheus \\
  --config.file=/etc/prometheus/prometheus.yml \\
  --storage.tsdb.path=/var/lib/prometheus/ \\
  --storage.tsdb.retention.time=30d \\
  --web.console.templates=/etc/prometheus/consoles \\
  --web.console.libraries=/etc/prometheus/console_libraries

Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable prometheus
sudo systemctl start prometheus
log_success "Prometheus service created and started"

# Install AlertManager
log "Installing AlertManager..."
cd /tmp
wget -q https://github.com/prometheus/alertmanager/releases/download/v${ALERTMANAGER_VERSION}/alertmanager-${ALERTMANAGER_VERSION}.linux-amd64.tar.gz
tar -xzf alertmanager-${ALERTMANAGER_VERSION}.linux-amd64.tar.gz

sudo cp alertmanager-${ALERTMANAGER_VERSION}.linux-amd64/alertmanager /usr/local/bin/
sudo chmod +x /usr/local/bin/alertmanager

# Create AlertManager configuration
sudo tee /etc/prometheus/alertmanager.yml > /dev/null <<'EOF'
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'cluster']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 6h
  receiver: 'default'
  routes:
    - match:
        severity: critical
      receiver: 'critical'

receivers:
  - name: 'default'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK'
        channel: '#incidents'
        title: '{{ .GroupLabels.alertname }}'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'

  - name: 'critical'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK'
        channel: '#critical-alerts'
        title: 'CRITICAL: {{ .GroupLabels.alertname }}'
        text: '{{ range .Alerts }}{{ .Annotations.summary }}\n{{ .Annotations.description }}{{ end }}'
    pagerduty_configs:
      - service_key: 'YOUR_PAGERDUTY_KEY'
EOF

sudo chown prometheus:prometheus /etc/prometheus/alertmanager.yml

# Create AlertManager systemd service
sudo tee /etc/systemd/system/alertmanager.service > /dev/null <<EOF
[Unit]
Description=Prometheus AlertManager
After=network.target

[Service]
User=prometheus
Group=prometheus
Type=simple
ExecStart=/usr/local/bin/alertmanager \\
  --config.file=/etc/prometheus/alertmanager.yml

Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable alertmanager
sudo systemctl start alertmanager
log_success "AlertManager installed and started"

# Install Grafana
log "Installing Grafana..."
cd /tmp
wget -q https://dl.grafana.com/oss/release/grafana-${GRAFANA_VERSION}.linux-amd64.tar.gz
tar -xzf grafana-${GRAFANA_VERSION}.linux-amd64.tar.gz
sudo cp -r grafana-${GRAFANA_VERSION}/conf /etc/grafana
sudo cp -r grafana-${GRAFANA_VERSION}/public /usr/share/grafana
sudo mkdir -p /usr/share/grafana/bin
sudo cp grafana-${GRAFANA_VERSION}/bin/grafana-server /usr/share/grafana/bin

sudo chown -R grafana:grafana /etc/grafana /var/lib/grafana

log_success "Grafana installed"

# Create Grafana systemd service
sudo tee /etc/systemd/system/grafana-server.service > /dev/null <<EOF
[Unit]
Description=Grafana
After=network.target

[Service]
User=grafana
Group=grafana
Type=simple
ExecStart=/usr/share/grafana/bin/grafana-server \\
  --config=/etc/grafana/conf/defaults.ini \\
  --homepath=/usr/share/grafana

Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable grafana-server
sudo systemctl start grafana-server
log_success "Grafana service created and started"

# Verify installation
log "Verifying installation..."
sleep 5

for service in prometheus alertmanager grafana-server node_exporter; do
    if sudo systemctl is-active $service >/dev/null 2>&1; then
        log_success "$service is running"
    else
        log_error "$service failed to start"
    fi
done

log "=========================================="
log_success "Monitoring Stack Setup Complete!"
log "=========================================="
log ""
log "Access points:"
log "  - Prometheus: http://localhost:9090"
log "  - AlertManager: http://localhost:9093"
log "  - Grafana: http://localhost:3000 (admin/admin)"
log ""
log "Next steps:"
log "  1. Login to Grafana and change default password"
log "  2. Add Prometheus as data source: http://localhost:9090"
log "  3. Import dashboards (see deployment/grafana/)"
log "  4. Configure Slack webhook in AlertManager config"
log "  5. Configure PagerDuty integration"
log ""
