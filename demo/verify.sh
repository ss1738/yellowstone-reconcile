#!/usr/bin/env bash
# Verifies the browser demo crate only. The main suite is scripts/verify.sh and does not cover this.
# Output goes to the terminal; capture it with: bash -o pipefail -c 'bash demo/verify.sh 2>&1 | tee LOG'
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
root="$PWD"

hash_of() { shasum -a 256 "$1" | cut -d' ' -f1; }

hostname
rustc --version
cargo --version
python3 demo/manifest.py

cd demo
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings

# Cargo records source paths in panic messages. Map the build machine's home directory, cargo
# registry and checkout to fixed names, most general first (the last matching mapping wins), so
# the published binary names neither the account nor the directory layout of the machine.
build_wasm() {
  rm -rf target/wasm32-unknown-unknown
  RUSTFLAGS="--remap-path-prefix=$HOME=/home/build --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo --remap-path-prefix=$root=/src/yellowstone-reconcile" \
    cargo build --locked --release --target wasm32-unknown-unknown
}
wasm=target/wasm32-unknown-unknown/release/reconcile_demo.wasm

build_wasm
first="$(hash_of "$wasm")"
bytes="$(wc -c <"$wasm" | tr -d ' ')"
build_wasm
second="$(hash_of "$wasm")"
if [ "$first" != "$second" ]; then
  echo "FAIL: two clean builds differ ($first vs $second)" >&2
  exit 1
fi

if strings -n 4 "$wasm" | grep -F -e "$HOME" -e "$root" -e "$(whoami)" >&2; then
  echo "FAIL: the wasm names the build machine (lines above)" >&2
  exit 1
fi

echo "WASM sha256=$first bytes=$bytes (two clean builds identical; no build-machine path embedded)"
