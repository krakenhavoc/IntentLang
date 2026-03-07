# Installation

## From crates.io

If you have Rust installed:

```bash
cargo install intent-cli
```

This installs the `intent` binary to `~/.cargo/bin/`.

## Build from source

Requires [Rust](https://rustup.rs/) 1.70+.

```bash
git clone https://github.com/krakenhavoc/IntentLang.git
cd IntentLang
cargo build --release -p intent-cli
# Binary at target/release/intent
```

## Pre-built binary (Linux x86\_64)

Download from the [latest release](https://github.com/krakenhavoc/IntentLang/releases):

```bash
chmod +x intent-linux-x86_64
./intent-linux-x86_64 check examples/transfer.intent
```

## Docker

```bash
docker build -t intent .
docker run -v $(pwd)/examples:/work intent check /work/transfer.intent
```

## Verify installation

```bash
intent --help
```

You should see the list of available subcommands: `check`, `render`, `render-html`, `compile`, `verify`, `audit`, `coverage`.
