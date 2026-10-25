"""Print deterministic hashes of executable inputs; no code execution or source mutation."""
import hashlib
from pathlib import Path

root = Path(__file__).resolve().parent.parent
paths = [root / name for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")]
for directory in ("src", "tests", "examples", "fixtures", "scripts", ".github/workflows"):
    paths.extend(path for path in (root / directory).rglob("*") if path.is_file())
for path in sorted(set(paths)):
    if path.exists() and "__pycache__" not in path.parts:
        print(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.relative_to(root)}")
