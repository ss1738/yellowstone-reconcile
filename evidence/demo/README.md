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

Deployment, 26 October 2026: before it, the file served at
`https://reconcile.satyawansingh.com/reconcile.wasm` had sha256
`adefc9a0fb351dbc20912d76d02280fa6254d677d471be31f8d546f817c71fd8` (136,015 bytes). It predated path
remapping and embedded the build machine's account name and checkout path. It was replaced with the
build recorded above. A download of the served file after the deployment had sha256
`a41f378f…ea2b` and 135,855 bytes, and a scan for the home directory, account name, `mini-work`,
`.cargo` and hostname found nothing. That matches this evidence as of that check. The site is a
separate deployment, so re-check the served hash rather than assuming it still holds.

Recheck the manifest with:

```sh
python3 demo/manifest.py | diff -u evidence/demo/source-manifest-demo.sha256 -
```
