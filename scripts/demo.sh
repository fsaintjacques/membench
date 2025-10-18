#!/bin/bash
# Demo script: Capture memcache traffic with membench and replay it
#
# This script demonstrates the full membench workflow:
# 1. Start memcached daemon
# 2. Start membench in record mode (capture traffic)
# 3. Generate load with memtier_benchmark
# 4. Stop membench record
# 5. Replay the captured profile against memcached
#
# Usage:
#   ./scripts/demo.sh [OPTIONS]
#
# Options:
#   --help              Show this help message
#   --port PORT         Memcached port (default: 11211)
#   --output FILE       Profile output file (default: /tmp/membench_demo.bin)
#   --clients N         Number of memtier clients (default: 4)
#   --requests N        Requests per client (default: 1000)
#   --keep-profile      Don't delete profile after replay
#   --verbose           Show more detailed output

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
MEMCACHED_PORT=11211
PROFILE_OUTPUT="/tmp/membench_demo.bin"
MEMTIER_CLIENTS=4
MEMTIER_REQUESTS=1000
KEEP_PROFILE=false
VERBOSE=false
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help)
                show_help
                exit 0
                ;;
            --port)
                MEMCACHED_PORT="$2"
                shift 2
                ;;
            --output)
                PROFILE_OUTPUT="$2"
                shift 2
                ;;
            --clients)
                MEMTIER_CLIENTS="$2"
                shift 2
                ;;
            --requests)
                MEMTIER_REQUESTS="$2"
                shift 2
                ;;
            --keep-profile)
                KEEP_PROFILE=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

show_help() {
    head -n 35 "$0" | tail -n +2
}

# Check if required tools are available
check_requirements() {
    local missing_tools=()

    if ! command -v memcached &> /dev/null; then
        missing_tools+=("memcached")
    fi

    if ! command -v memtier_benchmark &> /dev/null; then
        missing_tools+=("memtier_benchmark")
    fi

    if ! command -v cargo &> /dev/null; then
        missing_tools+=("cargo")
    fi

    if [ ${#missing_tools[@]} -gt 0 ]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        echo ""
        echo "Install with:"
        echo "  macOS:  brew install memcached memtier_benchmark"
        echo "  Ubuntu: sudo apt-get install memcached memtier"
        exit 1
    fi

    log_success "All required tools found"
}

# Build membench if not already built
build_membench() {
    log_info "Building membench..."
    cd "$PROJECT_ROOT"

    if cargo build --release 2>&1 | grep -q "Finished"; then
        log_success "membench built successfully"
    else
        log_error "Failed to build membench"
        exit 1
    fi
}

# Check if port is already in use
check_port() {
    if nc -z 127.0.0.1 "$MEMCACHED_PORT" 2>/dev/null; then
        log_warning "Port $MEMCACHED_PORT already in use (memcached may already be running)"
        log_info "Attempting to use existing memcached instance..."
        return 0
    fi
    return 1
}

# Start memcached in background
start_memcached() {
    log_info "Starting memcached on port $MEMCACHED_PORT..."

    if check_port; then
        MEMCACHED_STARTED=false
        return 0
    fi

    memcached -l 127.0.0.1 -p "$MEMCACHED_PORT" -m 256 &
    MEMCACHED_PID=$!
    MEMCACHED_STARTED=true

    # Wait for memcached to be ready
    local max_attempts=10
    local attempt=0
    while ! nc -z 127.0.0.1 "$MEMCACHED_PORT" 2>/dev/null; do
        if [ $attempt -ge $max_attempts ]; then
            log_error "memcached failed to start within ${max_attempts}s"
            exit 1
        fi
        sleep 0.5
        ((attempt++))
    done

    log_success "memcached started (PID: $MEMCACHED_PID)"
}

# Start membench in record mode
start_membench_record() {
    log_info "Starting membench record mode..."
    log_info "  Target: 127.0.0.1:$MEMCACHED_PORT"
    log_info "  Output: $PROFILE_OUTPUT"

    # Note: membench record requires network interface name
    # Using 'lo' for localhost traffic (may need adjustment on some systems)
    local interface="lo"

    # Check if interface exists (macOS vs Linux differences)
    if ! ifconfig "$interface" &> /dev/null; then
        interface="lo0"  # macOS uses lo0
        if ! ifconfig "$interface" &> /dev/null; then
            log_warning "Could not find loopback interface, attempting with 'lo'"
            interface="lo"
        fi
    fi

    # Start membench record in background
    # Note: This requires sudo for packet capture
    # For demo purposes, we'll attempt without sudo first
    sudo "${PROJECT_ROOT}/target/release/membench" record \
        --interface "$interface" \
        --port "$MEMCACHED_PORT" \
        --output "$PROFILE_OUTPUT" \
        &
    MEMBENCH_RECORD_PID=$!

    sleep 1  # Give membench time to start
    log_success "membench record started (PID: $MEMBENCH_RECORD_PID)"
}

# Generate load with memtier_benchmark
generate_load() {
    log_info "Generating load with memtier_benchmark..."
    log_info "  Clients: $MEMTIER_CLIENTS"
    log_info "  Requests per client: $MEMTIER_REQUESTS"

    memtier_benchmark \
        --server 127.0.0.1 \
        --port "$MEMCACHED_PORT" \
        --protocol memcache_text \
        --clients "$MEMTIER_CLIENTS" \
        --requests "$MEMTIER_REQUESTS" \
        --quiet > /dev/null 2>&1

    log_success "Load generation complete"
}

# Stop membench record
stop_membench_record() {
    log_info "Stopping membench record..."

    if [ -n "$MEMBENCH_RECORD_PID" ]; then
        # Send SIGTERM to membench
        if kill -0 "$MEMBENCH_RECORD_PID" 2>/dev/null; then
            kill "$MEMBENCH_RECORD_PID" 2>/dev/null || true

            # Wait for it to finish (max 5 seconds)
            local max_attempts=50
            local attempt=0
            while kill -0 "$MEMBENCH_RECORD_PID" 2>/dev/null; do
                if [ $attempt -ge $max_attempts ]; then
                    log_warning "membench record did not stop cleanly, forcing..."
                    kill -9 "$MEMBENCH_RECORD_PID" 2>/dev/null || true
                    break
                fi
                sleep 0.1
                ((attempt++))
            done
        fi
    fi

    log_success "membench record stopped"
}

# Check if profile was created
check_profile() {
    if [ -f "$PROFILE_OUTPUT" ]; then
        local size=$(ls -lh "$PROFILE_OUTPUT" | awk '{print $5}')
        log_success "Profile created: $PROFILE_OUTPUT ($size)"
        return 0
    else
        log_warning "Profile file not found: $PROFILE_OUTPUT"
        log_warning "This may be because:"
        log_warning "  - Network packet capture requires elevated privileges (sudo)"
        log_warning "  - The loopback interface name may differ on your system"
        log_warning "  - membench record encountered an error"
        return 1
    fi
}

# Replay the captured profile
replay_profile() {
    if [ ! -f "$PROFILE_OUTPUT" ]; then
        log_warning "Skipping replay: profile not found"
        return 1
    fi

    log_info "Replaying profile..."
    log_info "  Profile: $PROFILE_OUTPUT"
    log_info "  Target: 127.0.0.1:$MEMCACHED_PORT"
    log_info "  (Replaying 1000 commands, press Ctrl+C to stop)"

    # Run replay for a short time then stop
    timeout 10 "${PROJECT_ROOT}/target/release/membench" replay \
        --input "$PROFILE_OUTPUT" \
        --target "127.0.0.1:$MEMCACHED_PORT" \
        --concurrency 4 \
        || true  # timeout returns non-zero

    log_success "Replay complete"
}

# Cleanup
cleanup() {
    log_info "Cleaning up..."

    # Stop membench if still running
    if [ -n "$MEMBENCH_RECORD_PID" ] && kill -0 "$MEMBENCH_RECORD_PID" 2>/dev/null; then
        log_info "Stopping membench record..."
        kill "$MEMBENCH_RECORD_PID" 2>/dev/null || true
    fi

    # Stop memcached if we started it
    if [ "$MEMCACHED_STARTED" = true ] && [ -n "$MEMCACHED_PID" ]; then
        log_info "Stopping memcached..."
        kill "$MEMCACHED_PID" 2>/dev/null || true
    fi

    # Clean up profile if not keeping it
    if [ "$KEEP_PROFILE" = false ] && [ -f "$PROFILE_OUTPUT" ]; then
        log_info "Removing profile: $PROFILE_OUTPUT"
        rm -f "$PROFILE_OUTPUT"
    else
        log_info "Keeping profile: $PROFILE_OUTPUT"
    fi

    log_success "Cleanup complete"
}

# Trap cleanup on exit
trap cleanup EXIT INT TERM

# Main workflow
main() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║         membench Demo: Capture and Replay Workflow             ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Parse arguments
    parse_args "$@"

    # Verify configuration
    log_info "Configuration:"
    log_info "  Memcached port: $MEMCACHED_PORT"
    log_info "  Profile output: $PROFILE_OUTPUT"
    log_info "  memtier_benchmark: $MEMTIER_CLIENTS clients, $MEMTIER_REQUESTS requests"
    echo ""

    # Check requirements
    check_requirements
    echo ""

    # Build membench
    build_membench
    echo ""

    # Execute workflow
    start_memcached
    echo ""

    start_membench_record
    echo ""

    generate_load
    echo ""

    stop_membench_record
    echo ""

    check_profile
    echo ""

    replay_profile
    echo ""

    echo -e "${GREEN}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                    Demo Workflow Complete!                      ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

# Run main
main "$@"
