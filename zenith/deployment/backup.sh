#!/bin/bash
# Zenith Platform - Automated Database Backup & Recovery Script
# Usage:
#   Backup: ./backup.sh backup
#   List:   ./backup.sh list
#   Restore: ./backup.sh restore <backup_date>
#   Verify: ./backup.sh verify

set -e

GATEWAY_DATA_DIR="${GATEWAY_DATA_DIR:-.}/gateway-data"
BACKUP_DIR="${BACKUP_DIR:-.}/backups"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
LOG_FILE="${BACKUP_DIR}/backup.log"

# Ensure backup directory exists
mkdir -p "$BACKUP_DIR"

log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Backup sled database
backup() {
    log "Starting database backup..."

    if [ ! -d "$GATEWAY_DATA_DIR" ]; then
        log "ERROR: Gateway data directory not found: $GATEWAY_DATA_DIR"
        exit 1
    fi

    BACKUP_DATE=$(date +%Y%m%d_%H%M%S)
    BACKUP_PATH="$BACKUP_DIR/zenith_${BACKUP_DATE}.tar.gz"

    # Create compressed archive of sled database
    if tar -czf "$BACKUP_PATH" -C "$(dirname "$GATEWAY_DATA_DIR")" "$(basename "$GATEWAY_DATA_DIR")" 2>/dev/null; then
        SIZE=$(du -h "$BACKUP_PATH" | cut -f1)
        log "✓ Backup completed: $BACKUP_PATH ($SIZE)"

        # Verify backup integrity
        if tar -tzf "$BACKUP_PATH" > /dev/null 2>&1; then
            log "✓ Backup integrity verified"

            # Clean up old backups (retention policy)
            find "$BACKUP_DIR" -name "zenith_*.tar.gz" -mtime +$RETENTION_DAYS -delete
            log "✓ Cleaned up backups older than $RETENTION_DAYS days"
        else
            log "ERROR: Backup integrity check failed"
            rm "$BACKUP_PATH"
            exit 1
        fi
    else
        log "ERROR: Failed to create backup"
        exit 1
    fi
}

# List all available backups
list_backups() {
    log "Available backups:"
    if [ -d "$BACKUP_DIR" ]; then
        ls -lh "$BACKUP_DIR"/zenith_*.tar.gz 2>/dev/null | awk '{print $9, "(" $5 ")"}' || log "No backups found"
    else
        log "No backups found"
    fi
}

# Restore from backup
restore() {
    BACKUP_DATE="$1"
    if [ -z "$BACKUP_DATE" ]; then
        log "ERROR: Backup date not specified"
        log "Usage: ./backup.sh restore <backup_date>"
        list_backups
        exit 1
    fi

    BACKUP_PATH="$BACKUP_DIR/zenith_${BACKUP_DATE}.tar.gz"

    if [ ! -f "$BACKUP_PATH" ]; then
        log "ERROR: Backup file not found: $BACKUP_PATH"
        list_backups
        exit 1
    fi

    log "WARNING: About to restore from backup, current data will be moved to recovery"
    read -p "Continue? (type 'yes' to confirm): " confirm
    if [ "$confirm" != "yes" ]; then
        log "Restore cancelled"
        exit 0
    fi

    # Move current data to recovery location
    if [ -d "$GATEWAY_DATA_DIR" ]; then
        RECOVERY_DIR="${GATEWAY_DATA_DIR}.recovery.$(date +%Y%m%d_%H%M%S)"
        log "Moving current data to: $RECOVERY_DIR"
        mv "$GATEWAY_DATA_DIR" "$RECOVERY_DIR"
    fi

    # Extract backup
    log "Extracting backup..."
    tar -xzf "$BACKUP_PATH" -C "$(dirname "$GATEWAY_DATA_DIR")"

    if [ -d "$GATEWAY_DATA_DIR" ]; then
        log "✓ Restore completed successfully"
        log "Previous data saved to: $RECOVERY_DIR"
        log "NOTE: Restart the gateway for changes to take effect"
    else
        log "ERROR: Restore failed - data directory not created"
        exit 1
    fi
}

# Verify latest backup integrity
verify() {
    LATEST_BACKUP=$(find "$BACKUP_DIR" -name "zenith_*.tar.gz" -type f -printf '%T@ %p\n' | sort -rn | head -1 | cut -d' ' -f2-)

    if [ -z "$LATEST_BACKUP" ]; then
        log "ERROR: No backups found"
        exit 1
    fi

    log "Verifying: $LATEST_BACKUP"
    if tar -tzf "$LATEST_BACKUP" > /dev/null 2>&1; then
        FILE_COUNT=$(tar -tzf "$LATEST_BACKUP" | wc -l)
        SIZE=$(du -h "$LATEST_BACKUP" | cut -f1)
        log "✓ Backup verification passed"
        log "  Files: $FILE_COUNT, Size: $SIZE"
    else
        log "ERROR: Backup verification failed"
        exit 1
    fi
}

# Print usage
usage() {
    echo "Zenith Platform - Database Backup & Recovery"
    echo ""
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  backup              Create a new backup of gateway-data"
    echo "  list                List all available backups"
    echo "  restore <date>      Restore from backup (format: YYYYMMDD_HHMMSS)"
    echo "  verify              Verify integrity of latest backup"
    echo ""
    echo "Environment Variables:"
    echo "  GATEWAY_DATA_DIR    Path to gateway data directory (default: ./gateway-data)"
    echo "  BACKUP_DIR          Path to backup directory (default: ./backups)"
    echo "  RETENTION_DAYS      Keep backups for N days (default: 30)"
    echo ""
    echo "Examples:"
    echo "  ./backup.sh backup                    # Create backup"
    echo "  ./backup.sh list                      # List backups"
    echo "  ./backup.sh restore 20260428_120000   # Restore from specific date"
    echo "  ./backup.sh verify                    # Verify latest backup"
}

# Main
case "${1:-help}" in
    backup)
        backup
        ;;
    list)
        list_backups
        ;;
    restore)
        restore "$2"
        ;;
    verify)
        verify
        ;;
    *)
        usage
        exit 1
        ;;
esac
