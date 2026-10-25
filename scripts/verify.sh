#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
hostname
rustc --version
cargo --version
python3 scripts/source_manifest.py
cargo fmt --all --check
cargo test --locked --all-targets
cargo test --locked --all-features --all-targets
cargo test --locked --release --all-features --all-targets
cargo clippy --locked --all-features --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked --all-features --no-deps
cargo run --locked --release --bin yellowstone-replay -- fixtures/agreement.jsonl
for fixture in conflict malformed; do
  status=0
  cargo run --locked --release --bin yellowstone-replay -- "fixtures/$fixture.jsonl" || status=$?
  if [ "$status" -ne 1 ]; then
    echo "FAIL: $fixture replay returned $status, expected exactly 1" >&2
    exit 1
  fi
done
cargo run --locked --release --example benchmark
cargo run --locked --features upstream --example upstream_bridge
cargo run --locked --features simulation --example dependency_probe
cargo run --locked --release --features sqlite --example sqlite_sink
cargo run --locked --release --example durable_sink
cargo run --locked --release --example stall_budget -- 212
cargo run --locked --release --example payload_variance
python3 -m unittest scripts/test_pilot.py
python3 scripts/pilot.py --preflight
