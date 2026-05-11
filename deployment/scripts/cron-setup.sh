#!/bin/bash
# Setup cron jobs for Zenith operations
# Run this script to configure automated tasks

set -euo pipefail

LOG_FILE="/var/log/zenith/cron-setup.log"

echo "Setting up Zenith cron jobs..."

# Daily backup (2am every day)
echo "0 2 * * * /opt/zenith/scripts/backup.sh >> $LOG_FILE 2>&1" | crontab -

# Hourly health check (every hour)
echo "0 * * * * /opt/zenith/scripts/incident-response.sh >> $LOG_FILE 2>&1" | crontab -

# Restart services if down (every 30 minutes)
echo "*/30 * * * * pgrep -f 'zenith-validator' > /dev/null || systemctl restart zenith-validator" | crontab -
echo "*/30 * * * * pgrep -f 'zenith-gateway' > /dev/null || systemctl restart zenith-gateway" | crontab -

# Weekly backup verification (Sunday 3am)
echo "0 3 * * 0 /opt/zenith/scripts/verify-backup.sh >> $LOG_FILE 2>&1" | crontab -

# Rotate logs (daily 1am)
echo "0 1 * * * /opt/zenith/scripts/rotate-logs.sh >> $LOG_FILE 2>&1" | crontab -

# Certificate renewal check (daily 4am)
echo "0 4 * * * certbot renew --quiet" | crontab -

echo "✓ Cron jobs configured"
crontab -l
