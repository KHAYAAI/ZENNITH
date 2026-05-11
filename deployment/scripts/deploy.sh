#!/bin/bash
# Zenith Safe Rolling Deployment Orchestration
# Handles multi-phase deployment with consensus safety for validators
# Usage: ./deploy.sh <version> [--dry-run] [--force]

set -euo pipefail

# Configuration
VERSION="${1:-}"
DRY_RUN="${2:-}"
FORCE_MODE="${3:-}"
LOG_FILE="/var/log/zenith/deployment.log"
DATETIME=$(date '+%Y-%m-%d %H:%M:%S')

# Node lists
VALIDATORS=("validator-1" "validator-2" "validator-3" "validator-4" "validator-5")
GATEWAYS=("gateway-1" "gateway-2" "gateway-3")
PROVERS=("prover-plonk" "prover-cairo" "prover-risc0")
ARCHIVE_NODES=("archive-1")
SENTINELS=("rpc-sentinel-1")

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Helper functions
log() {
    echo -e "${BLUE}${DATETIME} [INFO]${NC} $1" | tee -a "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}${DATETIME} [OK]${NC} $1" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}${DATETIME} [ERROR]${NC} $1" | tee -a "$LOG_FILE"
}

log_warn() {
    echo -e "${YELLOW}${DATETIME} [WARN]${NC} $1" | tee -a "$LOG_FILE"
}

# Validate inputs
if [ -z "$VERSION" ]; then
    log_error "Usage: $0 <version> [--dry-run] [--force]"
    log_error "Example: $0 v1.2.0 --dry-run"
    exit 1
fi

# Check if binary exists
if [ ! -f "target/release/zenith-node" ] || [ ! -f "target/release/zenith-gateway" ]; then
    log_error "Binaries not found. Run: cargo build --release"
    exit 1
fi

log "=========================================="
log "ZENITH DEPLOYMENT: $VERSION"
log "=========================================="
log "Dry-run mode: ${DRY_RUN:-disabled}"
log "Force mode: ${FORCE_MODE:-disabled}"

# Pre-deployment checks
pre_deployment_checks() {
    log "Running pre-deployment checks..."

    # Check all validators are reachable
    for validator in "${VALIDATORS[@]}"; do
        if ! ping -c 1 -W 2 "$validator" >/dev/null 2>&1; then
            log_error "Cannot reach $validator"
            if [ "$FORCE_MODE" != "--force" ]; then
                return 1
            fi
        fi
    done

    # Check block production is happening
    local block_height=$(curl -s -X POST "http://localhost:9944" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
        | jq -r '.result.number' 2>/dev/null || echo "0")

    if [ "$block_height" == "0" ] || [ -z "$block_height" ]; then
        log_error "Cannot get current block height from RPC"
        if [ "$FORCE_MODE" != "--force" ]; then
            return 1
        fi
    fi

    log_success "Pre-deployment checks passed"
    return 0
}

# Deploy to single node
deploy_node() {
    local node=$1
    local node_type=$2  # validator, gateway, prover, archive, sentinel
    local binary_name=$3

    log "Deploying to $node ($node_type)..."

    if [ "$DRY_RUN" == "--dry-run" ]; then
        log "  [DRY-RUN] Would deploy $binary_name to $node"
        return 0
    fi

    ssh -o ConnectTimeout=10 "$node" "
        set -e
        echo 'Stopping service...'
        sudo systemctl stop zenith-${node_type} || true

        echo 'Backing up database...'
        [ -d /data/backup ] || mkdir -p /data/backup
        if [ -d /data/${node_type}/db ]; then
            tar -czf /data/backup/db-pre-deploy-\$(date +%s).tar.gz /data/${node_type}/db || true
        fi

        echo 'Deploying binary...'
        sudo cp /tmp/${binary_name} /opt/zenith/${binary_name}
        sudo chmod +x /opt/zenith/${binary_name}

        echo 'Starting service...'
        sudo systemctl start zenith-${node_type}

        # Wait for service to be ready
        for i in {1..30}; do
            if sudo systemctl is-active zenith-${node_type} >/dev/null 2>&1; then
                echo 'Service started'
                break
            fi
            sleep 1
        done

        if ! sudo systemctl is-active zenith-${node_type} >/dev/null 2>&1; then
            echo 'FAILED'
            exit 1
        fi
        echo 'OK'
    " 2>&1 | tail -5

    if [ $? -ne 0 ]; then
        log_error "Failed to deploy to $node"
        return 1
    fi

    log_success "Deployed to $node"
}

# Wait for validator consensus
wait_validator_consensus() {
    local validator=$1
    local timeout=60
    local elapsed=0

    log "Waiting for $validator to rejoin consensus..."

    while [ $elapsed -lt $timeout ]; do
        local block_height=$(ssh "$validator" "curl -s -X POST http://localhost:9944 \
            -H 'Content-Type: application/json' \
            -d '{\"jsonrpc\":\"2.0\",\"method\":\"chain_getHeader\",\"params\":[],\"id\":1}' \
            2>/dev/null | jq -r '.result.number' 2>/dev/null" || echo "0")

        if [ "$block_height" != "0" ] && [ -n "$block_height" ]; then
            log_success "$validator caught up (block: $block_height)"
            return 0
        fi

        sleep 2
        elapsed=$((elapsed + 2))
    done

    log_error "$validator did not rejoin consensus within ${timeout}s"
    return 1
}

# Check consensus is still healthy
check_consensus_health() {
    log "Checking consensus health..."

    local finalized=$(curl -s -X POST "http://localhost:9944" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[],"id":1}' \
        | jq -r '.result' 2>/dev/null || echo "")

    if [ -z "$finalized" ]; then
        log_error "Cannot query finalized head"
        return 1
    fi

    log_success "Finality is working"
    return 0
}

# Deploy RPC Sentinel (lowest risk)
phase_1_sentinel() {
    log ""
    log "PHASE 1: RPC Sentinel Deployment"
    log "Risk: LOW (no consensus participation)"
    log "Time: ~5 minutes"
    log ""

    for node in "${SENTINELS[@]}"; do
        deploy_node "$node" "sentinel" "zenith-node" || {
            log_warn "Failed to deploy to $node, continuing..."
        }
        sleep 10
    done

    check_consensus_health || log_warn "Consensus check failed, but continuing..."
}

# Deploy Archive Node (low risk)
phase_2_archive() {
    log ""
    log "PHASE 2: Archive Node Deployment"
    log "Risk: LOW (no consensus participation)"
    log "Time: ~5 minutes"
    log ""

    for node in "${ARCHIVE_NODES[@]}"; do
        deploy_node "$node" "validator" "zenith-node" || {
            log_warn "Failed to deploy to $node, continuing..."
        }
        sleep 10
    done

    check_consensus_health || log_warn "Consensus check failed, but continuing..."
}

# Deploy Validators (HIGH RISK - staggered)
phase_3_validators() {
    log ""
    log "PHASE 3: Validator Deployment (STAGGERED)"
    log "Risk: HIGH (consensus participants)"
    log "Strategy: One-at-a-time, wait 30s between each"
    log "Time: ~3 minutes"
    log ""

    for validator in "${VALIDATORS[@]}"; do
        log "Deploying validator $validator (${#VALIDATORS[@]} total)..."

        # Get current block height
        local pre_height=$(curl -s -X POST "http://localhost:9944" \
            -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
            | jq -r '.result.number' 2>/dev/null || echo "0")

        log "  Pre-deploy block height: $pre_height"

        deploy_node "$validator" "validator" "zenith-node" || {
            log_error "Failed to deploy to $validator"
            if [ "$FORCE_MODE" != "--force" ]; then
                return 1
            fi
        }

        # Wait for consensus to continue
        if ! wait_validator_consensus "$validator"; then
            log_error "Validator $validator did not rejoin consensus"
            if [ "$FORCE_MODE" != "--force" ]; then
                return 1
            fi
        fi

        # Verify finality is still happening
        sleep 10
        local post_height=$(curl -s -X POST "http://localhost:9944" \
            -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
            | jq -r '.result.number' 2>/dev/null || echo "0")

        log "  Post-deploy block height: $post_height"

        if [ "$post_height" -le "$pre_height" ]; then
            log_warn "Block height not advancing after deploying $validator"
        fi

        # Wait before next validator (to ensure consensus stability)
        if [ "$validator" != "${VALIDATORS[-1]}" ]; then
            log "  Waiting 30s before next validator..."
            sleep 30
        fi
    done

    log_success "All validators deployed successfully"
}

# Deploy Gateways (parallel safe)
phase_4_gateways() {
    log ""
    log "PHASE 4: Gateway Deployment (PARALLEL)"
    log "Risk: LOW (stateless, load balancer handles traffic)"
    log "Time: ~5 minutes"
    log ""

    for gateway in "${GATEWAYS[@]}"; do
        {
            log "Deploying gateway $gateway in background..."
            deploy_node "$gateway" "gateway" "zenith-gateway" || log_error "Failed to deploy $gateway"
        } &
    done

    # Wait for all gateway deployments
    wait

    log_success "All gateways deployed"

    # Verify gateways are responding
    sleep 5
    for gateway in "${GATEWAYS[@]}"; do
        if curl -s "http://${gateway}:8000/health" | grep -q "ok"; then
            log_success "$gateway is responding"
        else
            log_warn "$gateway health check failed"
        fi
    done
}

# Deploy Provers (parallel safe)
phase_5_provers() {
    log ""
    log "PHASE 5: Prover Deployment (PARALLEL)"
    log "Risk: LOW (async proof generation)"
    log "Time: ~5 minutes"
    log ""

    for prover in "${PROVERS[@]}"; do
        {
            log "Deploying prover $prover in background..."
            deploy_node "$prover" "prover" "zenith-prover" || log_error "Failed to deploy $prover"
        } &
    done

    # Wait for all prover deployments
    wait

    log_success "All provers deployed"
}

# Post-deployment validation
post_deployment_validation() {
    log ""
    log "PHASE 6: Post-Deployment Validation"
    log ""

    log "Checking consensus health..."
    check_consensus_health || return 1

    log "Checking gateway APIs..."
    for gateway in "${GATEWAYS[@]}"; do
        if curl -s "http://${gateway}:8000/health" > /dev/null 2>&1; then
            log_success "$gateway API responding"
        else
            log_error "$gateway API not responding"
        fi
    done

    log "Testing proof generation..."
    # Simple smoke test: deploy a canister and call it
    local canister_id=$(curl -s -X POST "http://localhost:8000/v1/canisters" \
        -H "Content-Type: application/json" \
        -d '{"wasm_base64":"AGFzbQEAAAA=","cycles":1000000}' \
        | jq -r '.canister_id' 2>/dev/null || echo "")

    if [ -n "$canister_id" ] && [ "$canister_id" != "null" ]; then
        log_success "Canister deployment works (ID: $canister_id)"
    else
        log_warn "Canister deployment test failed"
    fi

    log_success "Post-deployment validation complete"
}

# Rollback function (if needed)
rollback_deployment() {
    log_error "Initiating rollback..."

    for node in "${VALIDATORS[@]}" "${GATEWAYS[@]}" "${PROVERS[@]}" "${ARCHIVE_NODES[@]}"; do
        {
            ssh "$node" "
                echo 'Rolling back $node...'
                sudo systemctl stop zenith-* || true

                # Find most recent backup
                LATEST_BACKUP=\$(ls -t /data/backup/db-pre-deploy-*.tar.gz 2>/dev/null | head -1)
                if [ -n \"\$LATEST_BACKUP\" ]; then
                    echo 'Restoring from backup...'
                    tar -xzf \"\$LATEST_BACKUP\" -C /data/
                fi

                sudo systemctl start zenith-* || true
            "
        } &
    done

    wait

    log_error "Rollback complete. Please verify manually."
}

# Main execution
main() {
    log "=========================================="
    log "Starting deployment of version $VERSION"
    log "=========================================="

    # Pre-deployment checks
    pre_deployment_checks || {
        log_error "Pre-deployment checks failed"
        exit 1
    }

    # Phase 1: RPC Sentinel
    phase_1_sentinel || {
        log_error "Phase 1 failed"
        rollback_deployment
        exit 1
    }

    # Phase 2: Archive Node
    phase_2_archive || {
        log_warn "Phase 2 had issues, but continuing..."
    }

    # Phase 3: Validators (CRITICAL)
    if ! phase_3_validators; then
        log_error "Phase 3 (validators) failed - initiating rollback"
        rollback_deployment
        exit 1
    fi

    # Phase 4: Gateways
    phase_4_gateways || {
        log_warn "Phase 4 had issues"
    }

    # Phase 5: Provers
    phase_5_provers || {
        log_warn "Phase 5 had issues"
    }

    # Post-deployment validation
    post_deployment_validation || {
        log_warn "Post-deployment validation had issues"
    }

    log "=========================================="
    log_success "DEPLOYMENT COMPLETE: $VERSION"
    log "=========================================="
    log ""
    log "Next steps:"
    log "  1. Monitor consensus for 10 minutes"
    log "  2. Check all endpoints are responsive"
    log "  3. Verify proof generation is working"
    log "  4. Review deployment logs: $LOG_FILE"
}

# Run main function
main
