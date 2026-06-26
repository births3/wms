"""Shared helpers for Wave 1 rollback script tests."""
import subprocess
from pathlib import Path


def rollback_script() -> Path:
    return Path(__file__).resolve().parents[3] / "deploy" / "scripts" / "wave1_rollback.sh"


def run_rollback_script(
    *args: str,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(rollback_script()), *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )
