#!/bin/bash
set -e

# Verify eBPF support
echo ""
echo "Verifying eBPF support..."
kernel_version=$(uname -r)
echo "Kernel version: $kernel_version"

if grep -q "bpf" /proc/kallsyms 2>/dev/null; then
    echo "✓ eBPF syscalls available"
else
    echo "⚠ Warning: eBPF syscalls not found in kernel"
    echo "  eBPF capture may not work"
fi

# Verify BPF Type Format (BTF) support
if [ -f /sys/kernel/btf/vmlinux ]; then
    echo "✓ BPF Type Format (BTF) available"
else
    echo "⚠ Warning: BTF not available in kernel"
    echo "  This is needed for eBPF program loading"
fi

# Verify tcpdump installation
echo ""
echo "Verifying tcpdump installation..."
if command -v tcpdump &> /dev/null; then
    tcpdump -v
    echo "✓ tcpdump available"
else
    echo "⚠ Warning: tcpdump not found"
fi

# Verify Rust installation
echo ""
echo "Verifying Rust installation..."
rustc --version
cargo --version

# Install useful cargo tools
echo ""
echo "Installing cargo tools..."
cargo install cargo-watch cargo-edit cargo-expand 2>/dev/null || echo "Cargo tools already installed or failed to install"

sudo chown -R vscode:vscode /workspaces