# Browser demo

The replay driver from `src/bin/replay.rs`, compiled to WebAssembly and used by the project site
(<https://reconcile.satyawansingh.com/demo>). The reconciliation is this repository's own `Reconciler`;
`src/lib.rs` here only parses events and returns one JSON document. It makes no network requests.

Separate crate, not part of the library package or `scripts/verify.sh`. Built and tested on the
maintainer's build machine:

```sh
cd demo
cargo test                                          # 4 tests against ../fixtures
cargo build --release --target wasm32-unknown-unknown
```

The `.wasm` it produces is copied to the site by hand; the site is not built from CI.
