"""Print deterministic hashes of the inputs of the browser demo build; no code execution.

Separate from scripts/source_manifest.py on purpose: that manifest describes the main suite and
excludes the demo. This one covers the demo crate, the library files it compiles (the demo uses
the default features, so only src/lib.rs is built), the toolchain pin and the fixtures its tests read.
"""
import hashlib
from pathlib import Path

root = Path(__file__).resolve().parent.parent
paths = [
    root / name
    for name in (
        "Cargo.toml",
        "rust-toolchain.toml",
        "src/lib.rs",
        "demo/Cargo.toml",
        "demo/Cargo.lock",
        "demo/manifest.py",
        "demo/src/lib.rs",
        "demo/verify.sh",
    )
]
paths.extend(path for path in (root / "fixtures").rglob("*") if path.is_file())
for path in sorted(set(paths)):
    if path.exists():
        print(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.relative_to(root)}")
