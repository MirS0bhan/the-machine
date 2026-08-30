"""Full QEMU E2E proof — opt-in (slow, requires qemu-system-x86_64)."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]


@pytest.mark.skipif(
    os.environ.get("THE_MACHINE_RUN_E2E_PROOF") != "1",
    reason="set THE_MACHINE_RUN_E2E_PROOF=1 to run full QEMU E2E proof",
)
def test_boot_e2e_proof_script():
    """Runs build/test-boot-e2e-proof.sh (ISO build + QEMU virtio-vga + screendump)."""
    script = ROOT / "build" / "test-boot-e2e-proof.sh"
    subprocess.run(
        ["bash", str(script)],
        cwd=ROOT,
        check=True,
        timeout=900,
    )
