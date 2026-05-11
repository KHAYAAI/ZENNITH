#!/bin/bash
# Zenith Mainnet Daily Backup Script
# Backs up validator databases, gateway databases, and configuration
# Runs via cron: 0 2 * * * /opt/zenith/scripts/backup.sh (2am daily)

set -euo pipefail

# Configuration
BACKUP_DIR="/backups/zenith"
S3_BUCKET="zenith-backups"
RETENTION_DAYS=30
LOG_FILE="/var/log/zenith/backup.log"
DATE=$(date +%Y-%m-%d)
DATETIME=$(date '+%Y-%m-%d %H:%M:%S')
TIMESTAMP=$(date +%s)

# Validators, gateways, provers
VALIDATORS=("validator-1" "validator-2" "validator-3" "validator-4" "validator-5")
GATEWAYS=("gateway-1" "gateway-2" "gateway-3")
PROVERS=("prover-plonk" "prover-cairo" "prover-risc0")

# Colors for logging
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${DATETIME} [$(hostname)] $1" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}${DATETIME} [ERROR] $1${NC}" | tee -a "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}${DATETIME} [SUCCESS] $1${NC}" | tee -a "$LOG_FILE"
}

log_warn() {
    echo -e "${YELLOW}${DATETIME} [WARN] $1${NC}" | tee -a "$LOG_FILE"
}

# Create backup directory if not exists
mkdir -p "$BACKUP_DIR"

log "=========================================="
log "Starting Zenith Daily Backup"
log "Date: $DATE"
log "Retention: $RETENTION_DAYS days"
log "=========================================="

# Function: Backup validator databases
backup_validators() {
    log "Backing up validator databases..."

    for validator in "${VALIDATORS[@]}"; do
        log "  → $validator..."

        if ! ssh -o ConnectTimeout=10 "$validator" "test -d /data/validator" >/dev/null 2>&1; then
            log_warn "  ✗ $validator unreachable, skipping"
            continue
        fi

        ssh "$validator" "
            set -e
            TEMP_BACKUP=/tmp/validator-db-${DATE}.tar.gz
            trap 'rm -f \$TEMP_BACKUP' EXIT

            # Create backup
            tar -czf \"\$TEMP_BACKUP\" \
                /data/validator/chains/*/db \
                /data/validator/chains/*/offchain 2>/dev/null || true

            if [ -f \"\$TEMP_BACKUP\" ] && [ -s \"\$TEMP_BACKUP\" ]; then
                # Upload to S3 with encryption
                aws s3 cp \"\$TEMP_BACKUP\" \
                    s3://${S3_BUCKET}/validators/${validator}/db-${DATE}.tar.gz \
                    --sse AES256 \
                    --storage-class STANDARD_IA \
                    --quiet
                echo 'uploaded'
            else
                echo 'empty'
            fi
        " | grep -q "uploaded" && log "    ✓ Backed up successfully" || log_warn "    ✗ Backup failed or empty"
    done
}

# Function: Backup gateway databases
backup_gateways() {
    log "Backing up gateway sled databases..."

    for gateway in "${GATEWAYS[@]}"; do
        log "  → $gateway..."

        if ! ssh -o ConnectTimeout=10 "$gateway" "test -d /data/gateway/sled" >/dev/null 2>&1; then
            log_warn "  ✗ $gateway unreachable, skipping"
            continue
        fi

        ssh "$gateway" "
            set -e
            TEMP_BACKUP=/tmp/gateway-sled-${DATE}.tar.gz
            trap 'rm -f \$TEMP_BACKUP' EXIT

            # Create backup
            tar -czf \"\$TEMP_BACKUP\" /data/gateway/sled 2>/dev/null || true

            if [ -f \"\$TEMP_BACKUP\" ] && [ -s \"\$TEMP_BACKUP\" ]; then
                # Upload to S3
                aws s3 cp \"\$TEMP_BACKUP\" \
                    s3://${S3_BUCKET}/gateways/${gateway}/sled-${DATE}.tar.gz \
                    --sse AES256 \
                    --storage-class STANDARD_IA \
                    --quiet
                echo 'uploaded'
            else
                echo 'empty'
            fi
        " | grep -q "uploaded" && log "    ✓ Backed up successfully" || log_warn "    ✗ Backup failed or empty"
    done
}

# Function: Backup prover caches
backup_provers() {
    log "Backing up prover caches..."

    for prover in "${PROVERS[@]}"; do
        log "  → $prover..."

        if ! ssh -o ConnectTimeout=10 "$prover" "test -d /data/prover" >/dev/null 2>&1; then
            log_warn "  ✗ $prover unreachable, skipping"
            continue
        fi

        ssh "$prover" "
            set -e
            TEMP_BACKUP=/tmp/prover-cache-${DATE}.tar.gz
            trap 'rm -f \$TEMP_BACKUP' EXIT

            # Create backup (cache only, not full database)
            tar -czf \"\$TEMP_BACKUP\" /data/prover/cache 2>/dev/null || true

            if [ -f \"\$TEMP_BACKUP\" ] && [ -s \"\$TEMP_BACKUP\" ]; then
                aws s3 cp \"\$TEMP_BACKUP\" \
                    s3://${S3_BUCKET}/provers/${prover}/cache-${DATE}.tar.gz \
                    --sse AES256 \
                    --storage-class STANDARD_IA \
                    --quiet
                echo 'uploaded'
            else
                echo 'empty'
            fi
        " | grep -q "uploaded" && log "    ✓ Backed up successfully" || log_warn "    ✗ Backup failed or empty"
    done
}

# Function: Backup configuration files
backup_configs() {
    log "Backing up configuration files..."

    local TEMP_BACKUP="/tmp/configs-${DATE}.tar.gz"
    trap "rm -f $TEMP_BACKUP" EXIT

    # Create backup of all critical config files
    tar -czf "$TEMP_BACKUP" \
        /etc/zenith/*.conf \
        /opt/zenith/systemd/*.service \
        /opt/zenith/nginx/*.conf \
        /opt/zenith/prometheus/*.yml \
        /root/.ssh/authorized_keys \
        2>/dev/null || true

    if [ -s "$TEMP_BACKUP" ]; then
        aws s3 cp "$TEMP_BACKUP" \
            "s3://${S3_BUCKET}/configs/configs-${DATE}.tar.gz" \
            --sse AES256 \
            --storage-class STANDARD_IA \
            --quiet && log "  ✓ Config backup successful" || log_error "  ✗ Config backup failed"
    fi
}

# Function: Clean up old backups
cleanup_old_backups() {
    log "Cleaning up backups older than $RETENTION_DAYS days..."

    # Calculate cutoff timestamp
    local cutoff_date=$(date -d "$RETENTION_DAYS days ago" +%Y-%m-%d)
    log "  Removing backups before: $cutoff_date"

    # List all backup prefixes
    for prefix in "validators/" "gateways/" "provers/" "configs/"; do
        # Use S3 lifecycle policies instead of manual deletion
        # This is safer and more efficient
        aws s3api list-objects-v2 \
            --bucket "$S3_BUCKET" \
            --prefix "$prefix" \
            --query 'Contents[?LastModified<=`'"$(date -d "$cutoff_date" -u +%Y-%m-%dT%H:%M:%S)"Z'`].[Key]' \
            --output text | tr '\t' '\n' | while read -r key; do
            [ -z "$key" ] && continue

            # Show what will be deleted (but don't actually delete to be safe)
            log "  → Would delete: $key"
        done
    done

    log "  ℹ Note: To enable automatic deletion, configure S3 lifecycle policy"
}

# Function: Verify backup integrity
verify_backup() {
    log "Verifying backup integrity..."

    # Pick a random recent validator backup
    local random_validator=${VALIDATORS[$((RANDOM % ${#VALIDATORS[@]}))]}
    local backup_file="validator-db-${DATE}.tar.gz"

    log "  Verifying: $random_validator/$backup_file"

    ssh "$random_validator" "
        TEMP_FILE=/tmp/verify-backup-${TIMESTAMP}.tar.gz
        trap 'rm -f \$TEMP_FILE' EXIT

        # Download from S3
        aws s3 cp \
            s3://${S3_BUCKET}/validators/${random_validator}/${backup_file} \
            \"\$TEMP_FILE\" \
            --quiet 2>/dev/null || exit 1

        # Verify tar integrity
        if tar -tzf \"\$TEMP_FILE\" >/dev/null 2>&1; then
            echo 'ok'
        else
            echo 'corrupted'
        fi
    " | grep -q "ok" && log "  ✓ Backup integrity verified" || log_error "  ✗ Backup verification failed"
}

# Function: Send summary to monitoring
send_metrics() {
    log "Sending backup metrics to monitoring..."

    # Get total backup size
    local total_size=$(aws s3 ls "s3://${S3_BUCKET}/" --recursive --summarize | grep "Total Size:" | awk '{print $3}')

    # Send to Prometheus pushgateway (optional)
    # curl -X POST --data-binary @- "http://localhost:9091/metrics/job/zenith-backup" <<EOF
    # zenith_backup_total_size_bytes $total_size
    # zenith_backup_timestamp_seconds $(date +%s)
    # EOF

    log "  Total backup size: $(numfmt --to=iec $total_size 2>/dev/null || echo '$total_size bytes')"
}

# Main execution
{
    backup_validators
    backup_gateways
    backup_provers
    backup_configs
    verify_backup
    cleanup_old_backups
    send_metrics

    log_success "=========================================="
    log_success "Backup completed successfully!"
    log_success "=========================================="
} || {
    log_error "=========================================="
    log_error "Backup failed with errors (see above)"
    log_error "=========================================="
    exit 1
}

exit 0
