# Development Container

This directory contains a **editor-agnostic** development container setup for Membench.

Includes full support for eBPF development on Linux with tcpdump and necessary kernel tools.

## For All Developers (Docker/Podman)

The `Dockerfile` provides a clean Rust development environment with eBPF support that works with any editor.

### Build and run the container:

```bash
# Using Docker
docker build -t membench-dev .devcontainer
docker run -it --rm \
  -v "$(pwd):/workspace" \
  -w /workspace \
  --cap-add=SYS_NICE \
  --cap-add=SYS_RESOURCE \
  --cap-add=CAP_BPF \
  --cap-add=CAP_PERFMON \
  --cap-add=CAP_SYS_ADMIN \
  --ulimit memlock=-1:-1 \
  -e RUST_BACKTRACE=1 \
  membench-dev

# Using Podman
podman build -t membench-dev .devcontainer
podman run -it --rm \
  -v "$(pwd):/workspace:Z" \
  -w /workspace \
  --cap-add=SYS_NICE \
  --cap-add=SYS_RESOURCE \
  --cap-add=CAP_BPF \
  --cap-add=CAP_PERFMON \
  --cap-add=CAP_SYS_ADMIN \
  --ulimit memlock=-1:-1 \
  -e RUST_BACKTRACE=1 \
  membench-dev
```

### What's included:

- Rust latest (slim base)
- Git and Build essentials
- **memcached**: For replay testing and benchmarks
- **tcpdump**: Packet capture and PCAP generation
- **eBPF tools**: llvm, clang, libelf-dev, libpcap-dev
- **BPF support**: CAP_BPF, CAP_PERFMON for kernel packet capture
- Standard development tools (htop, net-tools, jq, etc.)

## For VSCode Users

The `devcontainer.json` provides VSCode-specific conveniences:

1. Install the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
2. Open this project in VSCode
3. Click "Reopen in Container" when prompted

### VSCode Extensions Included:

- rust-analyzer
- anthropic.claude-code

## For Other Editors

Use the Dockerfile directly with your preferred setup:

- **Neovim/Vim**: Mount your config and use inside the container
- **IntelliJ/CLion**: Configure Docker as a remote toolchain
- **Emacs**: Use TRAMP or similar remote editing
- **Any editor**: SSH into the running container

## eBPF Development

Inside the container, you can:

```bash
# Build with eBPF support
cargo build --release --features ebpf

# Capture traffic to PCAP file
tcpdump -i eth0 -w traffic.pcap tcp port 11211

# Record with live eBPF capture (when implemented)
sudo membench record "ebpf:eth0" profile.bin

# Record from PCAP file
membench record traffic.pcap profile.bin
```

## Architecture

The setup is intentionally minimal and transparent:

- No hidden features or magic
- Standard Rust toolchain
- Explicit dependencies for eBPF
- Works with Docker, Podman, or any OCI-compatible runtime
- Includes kernel capabilities needed for BPF operations
