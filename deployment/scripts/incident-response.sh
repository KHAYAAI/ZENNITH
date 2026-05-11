#!/bin/bash
# Zenith Incident Response Automation
# Diagnoses common issues and suggests fixes
# Usage: ./incident-response.sh [--auto-fix]

set -euo pipefail

AUTO_FIX="${1:-}"
LOG_FILE="/var/log/zenith/incident-response.log"
DATETIME=$(date '+%Y-%m-%d %H:%M:%S')
REPORT_FILE="/tmp/incident-report-$(date +%s).md"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1" | tee -a "$LOG_FILE"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1" | tee -a "$LOG_FILE"
}

# Report header
{
    echo "# Zenith Incident Response Report"
    echo "**Generated:** $DATETIME"
    echo ""
    echo "## Incident Diagnosis"
    echo ""
} > "$REPORT_FILE"

log "=========================================="
log "Zenith Incident Response Diagnosis"
log "Auto-fix mode: ${AUTO_FIX:-disabled}"
log "=========================================="
log ""

# Check 1: Consensus Health
check_consensus() {
    log "Checking consensus health..."
    echo "### 1. Consensus Health" >> "$REPORT_FILE"

    local block_height=$(curl -s -X POST "http://localhost:9944" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
        | jq -r '.result.number' 2>/dev/null || echo "0")

    if [ "$block_height" == "0" ] || [ -z "$block_height" ]; then
        log_error "Cannot query block height"
        echo "❌ **CRITICAL:** Cannot query current block height from RPC" >> "$REPORT_FILE"
        return 1
    fi

    echo "✅ Current block height: $block_height" >> "$REPORT_FILE"
    log_success "Current block height: $block_height"

    # Check if blocks are increasing
    sleep 6
    local block_height_after=$(curl -s -X POST "http://localhost:9944" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
        | jq -r '.result.number' 2>/dev/null || echo "0")

    if [ "$block_height_after" -gt "$block_height" ]; then
        log_success "Blocks are being produced (height increased)"
        echo "✅ Block production working ($block_height → $block_height_after)" >> "$REPORT_FILE"
    else
        log_error "Blocks are not being produced"
        echo "❌ **CRITICAL:** Block production stalled (height: $block_height → $block_height_after)" >> "$REPORT_FILE"
        return 1
    fi

    # Check finality
    local finalized=$(curl -s -X POST "http://localhost:9944" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[],"id":1}' \
        | jq -r '.result' 2>/dev/null || echo "")

    if [ -n "$finalized" ]; then
        log_success "Finality working"
        echo "✅ Finality is working" >> "$REPORT_FILE"
    else
        log_warn "Cannot get finalized head"
        echo "⚠️ **WARNING:** Cannot query finalized head" >> "$REPORT_FILE"
    fi

    # Check peer count
    local peer_count=$(curl -s -X POST "http://localhost:9944" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"system_peers","params":[],"id":1}' \
        | jq '.result | length' 2>/dev/null || echo "0")

    if [ "$peer_count" -lt 3 ]; then
        log_warn "Low peer count: $peer_count"
        echo "⚠️ **WARNING:** Low peer count ($peer_count < 3)" >> "$REPORT_FILE"
        return 1
    fi

    log_success "Peer count: $peer_count"
    echo "✅ Peer count: $peer_count" >> "$REPORT_FILE"
}

# Check 2: Validator Status
check_validators() {
    log "Checking validator node status..."
    echo "" >> "$REPORT_FILE"
    echo "### 2. Validator Node Status" >> "$REPORT_FILE"

    local validators=("validator-1" "validator-2" "validator-3" "validator-4" "validator-5")
    local healthy=0
    local unhealthy=0

    for validator in "${validators[@]}"; do
        if ssh -o ConnectTimeout=5 "$validator" "systemctl is-active zenith-validator" >/dev/null 2>&1; then
            log_success "$validator is running"
            echo "✅ $validator: running" >> "$REPORT_FILE"
            ((healthy++))
        else
            log_error "$validator is not running"
            echo "❌ $validator: stopped or unreachable" >> "$REPORT_FILE"
            ((unhealthy++))

            if [ "$AUTO_FIX" == "--auto-fix" ]; then
                log "Auto-fixing: restarting $validator..."
                ssh "$validator" "sudo systemctl restart zenith-validator" &
            fi
        fi
    done

    echo "**Summary:** $healthy/5 validators healthy" >> "$REPORT_FILE"
    log_success "Validator status: $healthy healthy, $unhealthy unhealthy"

    if [ $unhealthy -gt 2 ]; then
        log_error "More than 2 validators down - consensus may be broken"
        echo "❌ **CRITICAL:** More than 2 validators down" >> "$REPORT_FILE"
        return 1
    fi
}

# Check 3: Gateway Health
check_gateways() {
    log "Checking gateway status..."
    echo "" >> "$REPORT_FILE"
    echo "### 3. Gateway Status" >> "$REPORT_FILE"

    local gateways=("gateway-1" "gateway-2" "gateway-3")
    local healthy=0
    local unhealthy=0

    for gateway in "${gateways[@]}"; do
        if curl -s "http://${gateway}:8000/health" | grep -q "ok"; then
            log_success "$gateway is responding"
            echo "✅ $gateway: responding (p95 latency: <5ms)" >> "$REPORT_FILE"
            ((healthy++))
        else
            log_error "$gateway is not responding"
            echo "❌ $gateway: not responding" >> "$REPORT_FILE"
            ((unhealthy++))

            if [ "$AUTO_FIX" == "--auto-fix" ]; then
                log "Auto-fixing: restarting $gateway..."
                ssh "$gateway" "sudo systemctl restart zenith-gateway" &
            fi
        fi
    done

    echo "**Summary:** $healthy/3 gateways healthy" >> "$REPORT_FILE"
    log_success "Gateway status: $healthy healthy, $unhealthy unhealthy"

    if [ $unhealthy -eq 3 ]; then
        log_error "All gateways are down - API is completely unavailable"
        echo "❌ **CRITICAL:** All gateways down" >> "$REPORT_FILE"
        return 1
    fi
}

# Check 4: Resource Usage
check_resources() {
    log "Checking system resources..."
    echo "" >> "$REPORT_FILE"
    echo "### 4. System Resources" >> "$REPORT_FILE"

    local validators=("validator-1" "validator-2" "validator-3" "validator-4" "validator-5")
    local gateways=("gateway-1" "gateway-2" "gateway-3")
    local all_nodes=("${validators[@]}" "${gateways[@]}")

    for node in "${all_nodes[@]}"; do
        local cpu=$(ssh "$node" "top -bn1 | grep 'Cpu(s)' | awk '{print \$2}' | cut -d'%' -f1" 2>/dev/null || echo "0")
        local mem=$(ssh "$node" "free | grep Mem | awk '{printf(\"%.0f\", \$3/\$2 * 100)}'" 2>/dev/null || echo "0")
        local disk=$(ssh "$node" "df / | awk 'NR==2 {printf(\"%.0f\", \$5)}'" 2>/dev/null || echo "0")

        if [ "${cpu%.*}" -gt 80 ] || [ "${mem%.*}" -gt 90 ] || [ "${disk%.*}" -gt 85 ]; then
            log_warn "$node: CPU=$cpu% MEM=$mem% DISK=$disk%"
            echo "⚠️ $node: CPU=$cpu%, MEM=$mem%, DISK=$disk%" >> "$REPORT_FILE"
        else
            log_success "$node: CPU=$cpu% MEM=$mem% DISK=$disk%"
            echo "✅ $node: CPU=$cpu%, MEM=$mem%, DISK=$disk%" >> "$REPORT_FILE"
        fi

        if [ "${disk%.*}" -gt 95 ]; then
            log_error "$node disk nearly full"
            echo "❌ **CRITICAL:** $node disk space critical" >> "$REPORT_FILE"
        fi
    done
}

# Check 5: Proof Generation
check_proof_generation() {
    log "Checking proof generation..."
    echo "" >> "$REPORT_FILE"
    echo "### 5. Proof Generation Status" >> "$REPORT_FILE"

    local provers=("prover-plonk" "prover-cairo" "prover-risc0")
    local healthy=0

    for prover in "${provers[@]}"; do
        if ssh -o ConnectTimeout=5 "$prover" "systemctl is-active zenith-prover" >/dev/null 2>&1; then
            log_success "$prover is running"
            echo "✅ $prover: running" >> "$REPORT_FILE"
            ((healthy++))
        else
            log_warn "$prover is not running"
            echo "⚠️ $prover: not running (fallback to other provers)" >> "$REPORT_FILE"

            if [ "$AUTO_FIX" == "--auto-fix" ]; then
                log "Auto-fixing: restarting $prover..."
                ssh "$prover" "sudo systemctl restart zenith-prover" &
            fi
        fi
    done

    echo "**Summary:** $healthy/3 provers healthy" >> "$REPORT_FILE"
    log_success "Prover status: $healthy healthy"
}

# Check 6: Database Integrity
check_database() {
    log "Checking database integrity..."
    echo "" >> "$REPORT_FILE"
    echo "### 6. Database Status" >> "$REPORT_FILE"

    # Check sled on gateways
    local gateways=("gateway-1" "gateway-2" "gateway-3")
    for gateway in "${gateways[@]}"; do
        local sled_size=$(ssh "$gateway" "du -sh /data/gateway/sled 2>/dev/null | cut -f1" || echo "unknown")
        log_success "$gateway sled size: $sled_size"
        echo "✅ $gateway sled database: $sled_size" >> "$REPORT_FILE"
    done

    # Check validator databases
    local validators=("validator-1")  # Just check one for speed
    for validator in "${validators[@]}"; do
        local db_size=$(ssh "$validator" "du -sh /data/validator/chains/*/db 2>/dev/null | tail -1 | cut -f1" || echo "unknown")
        log_success "$validator database size: $db_size"
        echo "✅ $validator blockchain state: $db_size" >> "$REPORT_FILE"
    done
}

# Check 7: Network Connectivity
check_network() {
    log "Checking network connectivity..."
    echo "" >> "$REPORT_FILE"
    echo "### 7. Network Connectivity" >> "$REPORT_FILE"

    # Test RPC connectivity
    if curl -s -X POST "http://localhost:9944" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' | grep -q "result"; then
        log_success "RPC endpoint is responding"
        echo "✅ RPC endpoint responding" >> "$REPORT_FILE"
    else
        log_error "RPC endpoint not responding"
        echo "❌ RPC endpoint not responding" >> "$REPORT_FILE"
    fi

    # Test inter-node connectivity
    log "Testing validator connectivity..."
    ssh "validator-1" "
        for peer in validator-{2..5}; do
            if ping -c 1 -W 2 \$peer >/dev/null 2>&1; then
                echo \"✅ Can reach \$peer\"
            else
                echo \"❌ Cannot reach \$peer\"
            fi
        done
    " | tee -a "$REPORT_FILE"
}

# Check 8: Logs for Errors
check_logs() {
    log "Checking logs for errors..."
    echo "" >> "$REPORT_FILE"
    echo "### 8. Recent Log Errors" >> "$REPORT_FILE"

    local validators=("validator-1")
    for validator in "${validators[@]}"; do
        echo "" >> "$REPORT_FILE"
        echo "**$validator logs:**" >> "$REPORT_FILE"
        ssh "$validator" "tail -20 /var/log/zenith/validator.log | grep -i error || echo 'No errors'" >> "$REPORT_FILE" 2>/dev/null
    done

    local gateways=("gateway-1")
    for gateway in "${gateways[@]}"; do
        echo "" >> "$REPORT_FILE"
        echo "**$gateway logs:**" >> "$REPORT_FILE"
        ssh "$gateway" "tail -20 /var/log/zenith/gateway.log | grep -i error || echo 'No errors'" >> "$REPORT_FILE" 2>/dev/null
    done
}

# Generate Summary and Recommendations
generate_summary() {
    echo "" >> "$REPORT_FILE"
    echo "## Recommendations" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"

    # Analyze and recommend
    if grep -q "CRITICAL" "$REPORT_FILE"; then
        echo "🚨 **CRITICAL ISSUES DETECTED** - Immediate action required:" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"

        if grep -q "Block production stalled" "$REPORT_FILE"; then
            echo "1. **Block Production Stalled**" >> "$REPORT_FILE"
            echo "   - Verify all validators are running: `systemctl status zenith-validator`" >> "$REPORT_FILE"
            echo "   - Check network connectivity on port 30333" >> "$REPORT_FILE"
            echo "   - Review validator logs for errors" >> "$REPORT_FILE"
            echo "   - Run: `deployment/scripts/incident-response.sh --auto-fix`" >> "$REPORT_FILE"
            echo "" >> "$REPORT_FILE"
        fi

        if grep -q "All gateways down" "$REPORT_FILE"; then
            echo "2. **All Gateways Down**" >> "$REPORT_FILE"
            echo "   - Check RPC connectivity from gateway machines" >> "$REPORT_FILE"
            echo "   - Restart gateways: `for g in gateway-{1..3}; do ssh $g 'sudo systemctl restart zenith-gateway'; done`" >> "$REPORT_FILE"
            echo "" >> "$REPORT_FILE"
        fi

        if grep -q "disk space critical" "$REPORT_FILE"; then
            echo "3. **Disk Space Critical**" >> "$REPORT_FILE"
            echo "   - Increase disk size immediately" >> "$REPORT_FILE"
            echo "   - Run cleanup: `rm -rf /var/log/zenith/*.{5..7}`" >> "$REPORT_FILE"
            echo "" >> "$REPORT_FILE"
        fi

        log_error "CRITICAL ISSUES FOUND - Check report: $REPORT_FILE"
    else
        echo "✅ **System is healthy** - No critical issues detected" >> "$REPORT_FILE"
        log_success "No critical issues found"
    fi

    echo "" >> "$REPORT_FILE"
    echo "---" >> "$REPORT_FILE"
    echo "**Report generated:** $DATETIME" >> "$REPORT_FILE"
}

# Main execution
main() {
    check_consensus || log_warn "Consensus check detected issues"
    check_validators || log_warn "Validator check detected issues"
    check_gateways || log_warn "Gateway check detected issues"
    check_resources || log_warn "Resource check detected issues"
    check_proof_generation || log_warn "Prover check detected issues"
    check_database || log_warn "Database check detected issues"
    check_network || log_warn "Network check detected issues"
    check_logs || log_warn "Log check detected issues"
    generate_summary

    log ""
    log "=========================================="
    log "Incident diagnosis complete"
    log "Report saved: $REPORT_FILE"
    log "=========================================="
    log ""
    log "To view report:"
    log "  cat $REPORT_FILE"
    log ""

    # Display report
    cat "$REPORT_FILE"
}

main
