#!/bin/bash
# Zenith Testnet Deployment Script
# Sets up 3-validator testnet locally

set -e

echo "🚀 Zenith Testnet Setup"
echo "======================="

# Configuration
TESTNET_DIR="${1:-.}/testnet"
VALIDATOR_DIRS=("alice" "bob" "charlie")
RPC_PORT_BASE=9944
P2P_PORT_BASE=30333
HTTP_PORT_BASE=8000

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create testnet directory
mkdir -p "$TESTNET_DIR"
cd "$TESTNET_DIR"

echo -e "${GREEN}✓ Created testnet directory: $TESTNET_DIR${NC}"

# Build Zenith node (production binary)
echo -e "${YELLOW}→ Building Zenith node...${NC}"
cd ../../
cargo build --release -p zenith-node
NODE_BIN="$(pwd)/target/release/zenith-node"

echo -e "${GREEN}✓ Node built: $NODE_BIN${NC}"

# Create validator directories
echo -e "${YELLOW}→ Creating validator directories...${NC}"
cd "$TESTNET_DIR"
for validator in "${VALIDATOR_DIRS[@]}"; do
    mkdir -p "$validator/data"
done
echo -e "${GREEN}✓ Validator directories created${NC}"

# Start validators
echo -e "${YELLOW}→ Starting validators...${NC}"

for i in "${!VALIDATOR_DIRS[@]}"; do
    validator="${VALIDATOR_DIRS[$i]}"
    rpc_port=$((RPC_PORT_BASE + i))
    p2p_port=$((P2P_PORT_BASE + i))
    http_port=$((HTTP_PORT_BASE + i))

    echo -e "${YELLOW}  Starting $validator on RPC:$rpc_port, P2P:$p2p_port...${NC}"

    # Build startup command based on validator name
    case "$validator" in
        "alice")
            VALIDATOR_KEY="--alice"
            ;;
        "bob")
            VALIDATOR_KEY="--bob"
            ;;
        "charlie")
            VALIDATOR_KEY="--charlie"
            ;;
    esac

    # Start validator in background
    nohup "$NODE_BIN" \
        "$VALIDATOR_KEY" \
        --validator \
        --chain=custom-spec.json \
        --data-dir="./$validator/data" \
        --rpc-port=$rpc_port \
        --p2p-port=$p2p_port \
        --http-port=$http_port \
        --unsafe-rpc-external \
        --ws-external \
        --log=info,zenith=debug \
        > "./$validator/output.log" 2>&1 &

    echo "$!" > "./$validator/pid"
    echo -e "${GREEN}✓ $validator started (PID: $(cat ./$validator/pid))${NC}"

    # Wait a bit between startups
    sleep 2
done

echo -e "${YELLOW}→ Waiting for validators to sync...${NC}"
sleep 10

# Check validator status
echo -e "${YELLOW}→ Checking validator status...${NC}"
for validator in "${VALIDATOR_DIRS[@]}"; do
    rpc_port=$((RPC_PORT_BASE + ${VALIDATOR_DIRS[@]%%$validator*} | wc -w))

    # Simple health check
    if ps -p "$(cat ./$validator/pid)" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ $validator is running${NC}"
    else
        echo -e "${RED}✗ $validator failed to start${NC}"
        cat "./$validator/output.log" | tail -20
        exit 1
    fi
done

# Create monitoring script
cat > "monitor.sh" << 'MONITOR_EOF'
#!/bin/bash
# Monitor testnet validators

VALIDATORS=("alice" "bob" "charlie")
RPC_PORT_BASE=9944

while true; do
    echo "=== Zenith Testnet Status ($(date)) ==="

    for i in "${!VALIDATORS[@]}"; do
        validator="${VALIDATORS[$i]}"
        rpc_port=$((RPC_PORT_BASE + i))

        if ps -p "$(cat ./$validator/pid 2>/dev/null)" > /dev/null 2>&1; then
            # Get block height via RPC
            block_height=$(curl -s -X POST "http://localhost:$rpc_port" \
                -H "Content-Type: application/json" \
                -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
                | grep -o '"number":"0x[^"]*"' | cut -d'"' -f4)

            echo "✓ $validator (PID: $(cat ./$validator/pid)) - Block: $block_height"
        else
            echo "✗ $validator (NOT RUNNING)"
        fi
    done

    echo ""
    sleep 10
done
MONITOR_EOF

chmod +x monitor.sh

echo -e "${YELLOW}→ Testing RPC connectivity...${NC}"
sleep 5

# Test RPC
TEST_RESULT=$(curl -s -X POST "http://localhost:9944" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' 2>/dev/null || echo "")

if echo "$TEST_RESULT" | grep -q "result"; then
    echo -e "${GREEN}✓ RPC is responding${NC}"
else
    echo -e "${YELLOW}⚠ RPC may take a moment to respond, trying again...${NC}"
    sleep 5
    curl -s -X POST "http://localhost:9944" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' | head -20
fi

# Create stop script
cat > "stop.sh" << 'STOP_EOF'
#!/bin/bash
# Stop all validators

VALIDATORS=("alice" "bob" "charlie")

echo "Stopping validators..."
for validator in "${VALIDATORS[@]}"; do
    if [ -f "./$validator/pid" ]; then
        PID=$(cat "./$validator/pid")
        if ps -p "$PID" > /dev/null 2>&1; then
            kill "$PID"
            echo "✓ Stopped $validator (PID: $PID)"
        fi
    fi
done

echo "All validators stopped."
STOP_EOF

chmod +x stop.sh

# Summary
echo -e "${GREEN}"
echo "======================================"
echo "✓ Testnet Started Successfully!"
echo "======================================"
echo ""
echo "Validators Running:"
echo "  alice:   RPC=9944, P2P=30333"
echo "  bob:     RPC=9945, P2P=30334"
echo "  charlie: RPC=9946, P2P=30335"
echo ""
echo "Test RPC Connection:"
echo "  curl -X POST http://localhost:9944 -H 'Content-Type: application/json' \\"
echo "    -d '{\"jsonrpc\":\"2.0\",\"method\":\"system_health\",\"params\":[],\"id\":1}'"
echo ""
echo "Monitor Validators:"
echo "  ./monitor.sh"
echo ""
echo "Stop All Validators:"
echo "  ./stop.sh"
echo ""
echo "Gateway Setup:"
echo "  cargo run --release -p zenith-gateway -- \\"
echo "    --listen 0.0.0.0:8000 \\"
echo "    --node-rpc ws://localhost:9944 \\"
echo "    --data-dir ./gateway-data"
echo ""
echo -e "${NC}"
