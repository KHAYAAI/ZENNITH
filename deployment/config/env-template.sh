# Zenith Configuration Template
# Copy to /etc/zenith/ and customize for your deployment
# DO NOT commit actual secrets to git!

# ==============================================
# VALIDATOR CONFIGURATION
# ==============================================

# Validator identity
ZENITH_VALIDATOR_NAME="zenith-validator-1"
ZENITH_VALIDATOR_CHAIN="mainnet"

# Validator staking
ZENITH_VALIDATOR_BOND=1000           # ZEN (minimum stake)
ZENITH_VALIDATOR_COMMISSION=5        # Commission percentage (5%)

# RPC endpoints
ZENITH_VALIDATOR_RPC_PORT=9944
ZENITH_VALIDATOR_P2P_PORT=30333
ZENITH_VALIDATOR_HTTP_PORT=8000

# Database paths
ZENITH_VALIDATOR_DATA_DIR="/data/validator"
ZENITH_VALIDATOR_PRUNING="archive"   # Keep all blocks

# Session keys (rotate weekly)
# Generate with: subkey generate
ZENITH_VALIDATOR_SESSION_KEY="0x..."

# Stash account (cold storage, very secure)
ZENITH_VALIDATOR_STASH_KEY="0x..."

# Controller account (warm, slightly less secure)
ZENITH_VALIDATOR_CONTROLLER_KEY="0x..."

# ==============================================
# GATEWAY CONFIGURATION
# ==============================================

# Gateway identity
ZENITH_GATEWAY_NAME="gateway-1"
ZENITH_GATEWAY_LISTEN="0.0.0.0:8000"
ZENITH_GATEWAY_DATA_DIR="/data/gateway"

# RPC connection to validators
ZENITH_GATEWAY_NODE_RPC="ws://validator-1:9944"
ZENITH_GATEWAY_NODE_RPC_BACKUP="ws://validator-2:9944,ws://validator-3:9944"

# Proof generation
ZENITH_GATEWAY_PROOF_TIMEOUT=30000        # milliseconds
ZENITH_GATEWAY_PROOF_QUEUE_SIZE=1000      # pending proofs

# Authentication (rotate monthly)
ZENITH_GATEWAY_JWT_SECRET="$(openssl rand -hex 32)"
ZENITH_GATEWAY_JWT_EXPIRY=86400            # 24 hours

# Rate limiting
ZENITH_GATEWAY_RATE_LIMIT_GLOBAL=1000     # req/s
ZENITH_GATEWAY_RATE_LIMIT_PER_IP=100      # req/s

# Database encryption (rotate quarterly)
ZENITH_GATEWAY_DATABASE_ENCRYPTION_KEY="$(openssl rand -hex 32)"

# Caching
ZENITH_GATEWAY_CACHE_SIZE=1000000         # Maximum cache entries
ZENITH_GATEWAY_CACHE_TTL=300              # 5 minutes

# Logging
ZENITH_GATEWAY_LOG_LEVEL="info"           # debug, info, warn, error
ZENITH_GATEWAY_LOG_DIR="/var/log/zenith"

# ==============================================
# PROVER CONFIGURATION
# ==============================================

# Prover type
ZENITH_PROVER_TYPE="plonk"                # plonk, cairo, or risc0

# GPU acceleration (if available)
ZENITH_PROVER_GPU_ENABLED=1               # 1 for yes, 0 for no
ZENITH_PROVER_GPU_DEVICE=0                # Which GPU (0 = first)

# Proof generation limits
ZENITH_PROVER_TIMEOUT_SIMPLE=1000         # Simple ops: 1 second
ZENITH_PROVER_TIMEOUT_COMPLEX=30000       # Complex ops: 30 seconds

# Queue management
ZENITH_PROVER_QUEUE_PATH="/data/prover-queue"
ZENITH_PROVER_BATCH_SIZE=5                # Proofs per batch

# Memory limits
ZENITH_PROVER_CACHE_SIZE_GB=8             # Proof cache
ZENITH_PROVER_MAX_MEMORY_GB=16            # Total memory limit

# ==============================================
# SECURITY & OPERATIONS
# ==============================================

# TLS certificates (Let's Encrypt)
ZENITH_TLS_CERT="/etc/letsencrypt/live/zenith.network/fullchain.pem"
ZENITH_TLS_KEY="/etc/letsencrypt/live/zenith.network/privkey.pem"

# Audit logging
ZENITH_AUDIT_LOG_DIR="/var/log/zenith/audit"
ZENITH_AUDIT_LOG_RETENTION_DAYS=30

# Backup configuration
ZENITH_BACKUP_S3_BUCKET="zenith-backups"
ZENITH_BACKUP_S3_REGION="us-east-1"
ZENITH_BACKUP_RETENTION_DAYS=30

# Monitoring
ZENITH_PROMETHEUS_PORT=9090
ZENITH_PROMETHEUS_RETENTION="30d"
ZENITH_PROMETHEUS_SCRAPE_INTERVAL="15s"

ZENITH_GRAFANA_PORT=3000
ZENITH_GRAFANA_ADMIN_PASSWORD="$(openssl rand -hex 16)"

ZENITH_ALERTMANAGER_PORT=9093
ZENITH_ALERTMANAGER_SLACK_WEBHOOK="https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK"

# ==============================================
# AWS CREDENTIALS (for backups)
# ==============================================

# AWS IAM user with S3 access
AWS_ACCESS_KEY_ID="AKIA..."
AWS_SECRET_ACCESS_KEY="..."
AWS_DEFAULT_REGION="us-east-1"

# ==============================================
# LOGGING & TRACING
# ==============================================

RUST_LOG="zenith_node=info,zenith_gateway=info,hyper=warn"
RUST_BACKTRACE="1"
ZENITH_LOG_FORMAT="json"                  # json or text

# ==============================================
# ENVIRONMENT-SPECIFIC
# ==============================================

# Development
ENVIRONMENT="mainnet"                      # dev, testnet, or mainnet

# Telemetry
ZENITH_TELEMETRY_ENABLED=1
ZENITH_TELEMETRY_URL="wss://telemetry.polkadot.io/submit/"

# ==============================================
# PAGERDUTY INTEGRATION (for alerts)
# ==============================================

PAGERDUTY_API_KEY="..."
PAGERDUTY_SERVICE_KEY="..."

# ==============================================
# NOTES FOR PRODUCTION DEPLOYMENT
# ==============================================

# Before deploying:
# 1. Generate all keys securely (not shown in logs)
# 2. Store secrets in AWS Secrets Manager or HashiCorp Vault
# 3. Rotate secrets on schedule:
#    - Weekly: Session keys (ZENITH_VALIDATOR_SESSION_KEY)
#    - Monthly: JWT secret (ZENITH_GATEWAY_JWT_SECRET)
#    - Quarterly: Database encryption key (ZENITH_GATEWAY_DATABASE_ENCRYPTION_KEY)
#    - Annually: Cold storage keys (ZENITH_VALIDATOR_STASH_KEY)
#
# 4. Never commit this file with actual secrets to git
# 5. Use .gitignore to exclude environment files:
#    echo "/etc/zenith/*.env" >> .gitignore
#
# 6. Deploy via CI/CD that handles secrets securely
#    - GitHub Actions: Use Secrets
#    - GitLab: Use Protected Variables
#    - Jenkins: Use Credentials Plugin
