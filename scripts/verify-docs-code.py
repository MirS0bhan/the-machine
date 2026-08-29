#!/usr/bin/env python3
"""Verify documentation matches the codebase (and vice versa)."""

from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    import subprocess

    subprocess.check_call([sys.executable, "-m", "pip", "install", "-q", "pyyaml"])
    import yaml

ROOT = Path(__file__).resolve().parent.parent
INVENTORY = ROOT / "docs/reference/component-inventory.yaml"

RED = "\033[0;31m"
GREEN = "\033[0;32m"
NC = "\033[0m"


def fail(msg: str) -> None:
    print(f"{RED}✗{NC} {msg}", file=sys.stderr)
    sys.exit(1)


def pass_(msg: str) -> None:
    print(f"{GREEN}✓{NC} {msg}")


def load_inventory() -> dict:
    if not INVENTORY.exists():
        fail(f"Missing canonical inventory: {INVENTORY}")
    return yaml.safe_load(INVENTORY.read_text())


def parse_mkinitramfs_services() -> list[str]:
    text = (ROOT / "build/mkinitramfs.sh").read_text()
    m = re.search(r"SERVICES=\(\n((?:\s+[^\n]+\n)+)\)", text)
    if not m:
        fail("Could not parse SERVICES from build/mkinitramfs.sh")
    return [line.strip() for line in m.group(1).splitlines() if line.strip()]


def parse_verify_all_bins() -> list[str]:
    text = (ROOT / "scripts/verify-all.sh").read_text()
    m = re.search(r"RUST_BINS=\(\n((?:\s+[^\n]+\n)+)\)", text)
    if not m:
        fail("Could not parse RUST_BINS from scripts/verify-all.sh")
    bins: list[str] = []
    for line in m.group(1).splitlines():
        bins.extend(line.strip().split())
    return bins


def parse_cargo_members() -> list[str]:
    text = (ROOT / "Cargo.toml").read_text()
    members: list[str] = []
    in_members = False
    for line in text.splitlines():
        if line.strip() == "members = [":
            in_members = True
            continue
        if in_members:
            if line.strip() == "]":
                break
            m = re.match(r'\s*"([^"]+)"', line)
            if m:
                members.append(m.group(1))
    return members


def grep_method_in_source(method: str) -> bool:
    """Return True if method string appears in Rust service sources."""
    pattern = re.escape(f'"{method}"')
    for path in ROOT.glob("*/src/**/*.rs"):
        if path.read_text(errors="replace").find(f'"{method}"') >= 0:
            return True
    return False


def norm(text: str) -> str:
    return text.lower().replace("-", " ").replace("_", " ")


def doc_mentions(doc_text: str, term: str) -> bool:
    return term in doc_text.lower() or term.replace("-", " ") in norm(doc_text)


def main() -> None:
    inv = load_inventory()
    errors: list[str] = []

    boot_inv = inv["boot_services"]
    boot_script = parse_mkinitramfs_services()
    if boot_inv != boot_script:
        errors.append(
            f"boot_services mismatch: inventory={boot_inv} mkinitramfs={boot_script}"
        )

    rust_bins = parse_verify_all_bins()
    expected_bins = [c for c in inv["rust_crates"] if c not in ("common", "lambda-examples")]
    missing_bins = sorted(set(expected_bins) - set(rust_bins))
    extra_bins = sorted(set(rust_bins) - set(expected_bins))
    if missing_bins:
        errors.append(f"verify-all.sh missing binaries: {missing_bins}")
    if extra_bins:
        errors.append(f"verify-all.sh unexpected binaries: {extra_bins}")

    members = parse_cargo_members()
    for crate in inv["rust_crates"]:
        if crate not in members:
            errors.append(f"Cargo workspace missing crate: {crate}")

    for pkg in inv["python_packages"]:
        if not (ROOT / pkg / "pyproject.toml").exists():
            errors.append(f"Python package missing pyproject.toml: {pkg}")

    doc_files = [
        ROOT / "README.md",
        ROOT / "docs/index.md",
        ROOT / "docs/book/index.md",
        ROOT / "docs/guides/python-rust-overlap.md",
        ROOT / "docs/architecture/runtime-model.md",
    ]
    key_terms = ["local-model-daemon", "marketplace", "policy-broker", "agent-core"]
    for doc in doc_files:
        text = doc.read_text()
        for term in key_terms:
            if not doc_mentions(text, term):
                errors.append(f"{doc.relative_to(ROOT)} missing mention of {term}")

    runtime = (ROOT / "docs/architecture/runtime-model.md").read_text()
    stale_phrases = [
        "heuristic today; LLM-backed in full deployment",
        "wlroots integration is planned",
        "Planned adapter",
        "Rust (stub)",
        "wire `local-model` into agent-core",
    ]
    for phrase in stale_phrases:
        if phrase in runtime:
            errors.append(f"runtime-model.md stale phrase: {phrase!r}")

    overlap = (ROOT / "docs/guides/python-rust-overlap.md").read_text()
    if "local-model-daemon" not in overlap:
        errors.append("python-rust-overlap.md missing local-model-daemon row")
    if "## Known stale references" in overlap:
        errors.append("python-rust-overlap.md still has 'Known stale references' section")

    for service, methods in inv.get("mcp_services", {}).items():
        for method in methods:
            if not grep_method_in_source(method):
                errors.append(f"MCP method {method!r} ({service}) not found in Rust sources")

    readme = (ROOT / "README.md").read_text().lower()
    status_markers = ["local-model-daemon", "marketplace", "framebuffer", "evdev"]
    for marker in status_markers:
        if marker not in readme:
            errors.append(f"README.md status table missing: {marker}")

    gap = (ROOT / "docs/architecture/gap-analysis.md").read_text()
    for gap_id in ["G2", "G5", "G11", "G15"]:
        if f"**{gap_id}**" not in gap and f"[x] **{gap_id}**" not in gap:
            errors.append(f"gap-analysis.md missing closed gap {gap_id}")

    env_docs = "\n".join(
        p.read_text() for p in [
            ROOT / "docs/guides/getting-started.md",
            ROOT / "docs/guides/python-rust-overlap.md",
            ROOT / "build/mkinitramfs.sh",
        ]
    )
    for var in inv["env_vars"]:
        if var not in env_docs and var not in (ROOT / "scripts/start-services.sh").read_text():
            errors.append(f"env var {var} not documented in guides or start scripts")

    if errors:
        print("Documentation ↔ code verification FAILED:\n", file=sys.stderr)
        for e in errors:
            print(f"  ✗ {e}", file=sys.stderr)
        sys.exit(1)

    pass_(f"boot services ({len(boot_inv)}) match mkinitramfs.sh")
    pass_(f"Rust binaries ({len(rust_bins)}) match component inventory")
    pass_(f"Cargo workspace ({len(members)} members) matches inventory")
    pass_(f"Python packages ({len(inv['python_packages'])}) present")
    pass_(f"Key docs mention new services (local-model-daemon, marketplace)")
    pass_(f"MCP methods ({sum(len(v) for v in inv['mcp_services'].values())}) found in sources")
    pass_(f"Environment variables ({len(inv['env_vars'])}) documented")
    print(f"{GREEN}✓{NC} Documentation ↔ code verification PASSED")


if __name__ == "__main__":
    main()
