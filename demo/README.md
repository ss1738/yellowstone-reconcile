# Browser demo

The replay driver from `src/bin/replay.rs`, compiled to WebAssembly and used by the project site
(<https://reconcile.satyawansingh.com>). The reconciliation is this repository's own `Reconciler`;
`src/lib.rs` here only parses events and returns one JSON document. It makes no network requests.

This is a separate crate. It is **not** part of the library package, `scripts/verify.sh` or
`evidence/source-manifest.sha256`, and a pass of the main suite says nothing about it. Its own
verification and manifest are described below.

## Verify it

On a build machine with the pinned Rust toolchain and the `wasm32-unknown-unknown` target for that
toolchain (`rustup target add wasm32-unknown-unknown --toolchain 1.95.0`):

```sh
bash -o pipefail -c 'bash demo/verify.sh 2>&1 | tee /tmp/yellowstone-demo-verify.log'
python3 demo/manifest.py | diff -u evidence/demo/source-manifest-demo.sha256 -
```

`demo/verify.sh` runs the native tests (4, against `../fixtures`), formatting and clippy, then
builds the WebAssembly twice from clean and fails if the two builds differ or if the binary names the
build machine (home directory, account name or checkout path). It prints the SHA-256 of the build.

The path check exists because cargo records source paths in panic messages. The script therefore
builds with `--remap-path-prefix` for the home directory, the cargo registry and the checkout.
Building by hand without those flags produces a binary that contains your local paths.

## Publish it

The site copies the `.wasm` by hand and is deployed separately, so a passing verification does not
show that the deployed file is the one it built. Compare hashes explicitly:

```sh
curl -s https://reconcile.satyawansingh.com/reconcile.wasm | shasum -a 256
```

and record both values next to the log. Dated results are in `evidence/demo/`.
