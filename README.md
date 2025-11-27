# cargo-tako

Command-line tool for developing TAKO smart contracts on the TOS blockchain.

## Features

- **Build contracts** with TBPF V0-V4 architecture support (default: V3)
- **Verify ELF** files for correct e_flags and format
- **Project scaffolding** with templates (default, erc20, erc721)
- **Automatic toolchain detection** for TOS platform-tools

## Installation

### From crates.io (coming soon)

```bash
cargo install cargo-tako
```

### From source

```bash
git clone https://github.com/tos-network/cargo-tako
cd cargo-tako
cargo install --path .
```

### Prerequisites

You need the TOS platform-tools installed. The tool looks for it in:

1. `~/tos-network/platform-tools/out/rust/bin/`
2. `~/.tos/platform-tools/rust/bin/`
3. `/usr/local/tos/platform-tools/rust/bin/`

## Usage

### Create a new project

```bash
# Create a new contract project
cargo tako new my-contract

# Use a specific template
cargo tako new my-token --template erc20
```

### Build a contract

```bash
# Build with default settings (V3, release mode)
cargo tako build --release

# Specify architecture version
cargo tako build --arch v3 --release

# Build and verify
cargo tako build --release --verify

# Build with ELF dump
cargo tako build --release --dump
```

### Available architectures

| Arch | e_flags | Description |
|------|---------|-------------|
| v0 | 0x0 | Legacy version |
| v1 | 0x1 | Dynamic stack frames |
| v2 | 0x2 | Arithmetic improvements |
| v3 | 0x3 | Static syscalls, strict ELF (default, production) |
| v4 | 0x4 | ABI v2 (experimental) |

### Other commands

```bash
# Initialize TAKO in existing project
cargo tako init

# Run tests
cargo tako test

# Clean build artifacts
cargo tako clean

# Show contract info
cargo tako info
```

## TBPF V3 Memory Layout

V3 contracts use a strict memory layout with 4GB boundaries:

```
0x000000000 - TEXT   (code, executable)
0x100000000 - RODATA (read-only data)
0x200000000 - STACK  (read-write)
0x300000000 - HEAP   (read-write)
```

## License

Apache-2.0
