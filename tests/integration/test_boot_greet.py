"""Boot greet user story — ISO boots, agent patches chat UI."""

from __future__ import annotations

import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def test_boot_greet_e2e_script_passes():
    """Runs build/test-boot-greet-e2e.sh (AUIL + planner + compositor dump)."""
    script = ROOT / "build" / "test-boot-greet-e2e.sh"
    subprocess.run(["bash", str(script)], cwd=ROOT, check=True)


def test_boot_greet_services_integration():
    """Starts live Rust services and verifies boot greet + chat send."""
    script = ROOT / "build" / "test-boot-greet-services.sh"
    subprocess.run(["bash", str(script)], cwd=ROOT, check=True, timeout=180)


def test_boot_auil_defines_chat_widgets():
    auil = (ROOT / "build" / "boot.auil").read_text()
    for widget in ("ui.greeting", "ui.chat_log", "ui.chat_input", "ui.chat_send"):
        assert widget in auil
    assert "agent.chat.send" in auil
