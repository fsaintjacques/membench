#!/bin/bash
# Rebuild eBPF bytecode from source
# Run this script whenever ebpf/src/programs.rs changes

set -e

echo "Building eBPF bytecode..."

# Check for required tools
if ! command -v rustup &> /dev/null; then
    echo "Error: rustup not found"
    exit 1
fi

if ! rustup toolchain list | grep -q nightly; then
    echo "Installing nightly toolchain..."
    rustup toolchain install nightly --component rust-src
fi

if ! command -v bpf-linker &> /dev/null; then
    echo "Installing bpf-linker..."
    cargo install bpf-linker
fi

# Build eBPF program
echo "Compiling eBPF program..."
RUSTUP_TOOLCHAIN=nightly cargo build --release

# Copy to committed location
cp target/bpfel-unknown-none/release/programs programs.bpf

echo "✓ eBPF bytecode rebuilt successfully: programs.bpf ($(stat -c%s programs.bpf 2>/dev/null || stat -f%z programs.bpf) bytes)"
echo ""
echo "Don't forget to:"
echo "  1. Test the changes: cargo test --features ebpf"
echo "  2. Commit the updated bytecode: git add programs.bpf"
