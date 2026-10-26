# Browser demo evidence

Scope: the `demo/` crate only. It is separate from the main suite (`scripts/verify.sh`,
`evidence/source-manifest.sha256`); a pass of one says nothing about the other.

| File | What it is |
|---|---|
| `verification-demo-20261026.log` | Raw output of `bash demo/verify.sh` on Ironman, 26 October 2026 (exit 0) |
| `source-manifest-demo.sha256` | SHA-256 of the 11 inputs of that run, from `python3 demo/manifest.py` |

Build recorded by that run: `reconcile_demo.wasm`, 135,855 bytes,
sha256 `a41f378f69efda10e92ee443e958e459bb9b9e1a2cebaa3c58046bf51984ea2b`. Two clean builds were
identical, and an independent `strings` scan of the file found no home directory, account name,
checkout path or hostname.

The file served at `https://reconcile.satyawansingh.com/reconcile.wasm` on 26 October 2026 had
sha256 `adefc9a0fb351dbc20912d76d02280fa6254d677d471be31f8d546f817c71fd8` (136,015 bytes). It is
**not** this build: it predates path remapping and embeds the build machine's account name and
checkout path. A rebuilt artifact has not been deployed, so the site does not match this evidence.

Recheck the manifest with:

```sh
python3 demo/manifest.py | diff -u evidence/demo/source-manifest-demo.sha256 -
```
