#!/usr/bin/env python3
"""Write manifest.json for a packaged artifact directory."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def file_entry(path: Path) -> dict:
    return {"name": path.name, "size": path.stat().st_size, "sha256": sha256(path)}


def main() -> None:
    out = Path(sys.argv[1])
    manifest = {
        "rust": [file_entry(p) for p in sorted((out / "rust").glob("*")) if p.is_file()],
        "rust_examples": [
            file_entry(p) for p in sorted((out / "rust" / "examples").glob("*")) if p.is_file()
        ],
        "python": [
            file_entry(p) for p in sorted((out / "python" / "wheels").glob("*.whl"))
        ],
        "iso": [
            file_entry(out / "iso" / name)
            for name in ("initramfs.cpio.gz", "the-machine.iso")
            if (out / "iso" / name).exists()
        ],
        "coverage": [
            file_entry(p)
            for p in sorted((out / "coverage").glob("*.lcov"))
            if p.is_file()
        ],
    }
    with open(out / "manifest.json", "w") as f:
        json.dump(manifest, f, indent=2)
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
