#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
hostname
rustc --version
cargo --version
python3 scripts/source_manifest.py
cargo fmt --all --check
cargo test --all-features --all-targets
cargo test --release --all-features --all-targets
cargo clippy --all-features --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --all-features --no-deps
cargo run --release --bin yellowstone-replay -- fixtures/agreement.jsonl
if cargo run --release --bin yellowstone-replay -- fixtures/conflict.jsonl; then
  echo 'FAIL: conflicting replay unexpectedly succeeded' >&2
  exit 1
fi
cargo run --release --example benchmark
cargo run --features upstream --example upstream_bridge
cargo run --features simulation --example dependency_probe
