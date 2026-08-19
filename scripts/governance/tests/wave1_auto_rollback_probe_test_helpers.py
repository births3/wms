"""Shared helpers for Wave 1 auto rollback probe tests."""
import subprocess
from pathlib import Path


def auto_rollback_script() -> Path:
    return (
        Path(__file__).resolve().parents[3]
        / "deploy"
        / "scripts"
        / "wave1_auto_rollback_probe.sh"
    )


def run_auto_rollback_probe(
    args: list[str],
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(auto_rollback_script()), *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )


def write_fake_bin(bin_dir: Path, name: str, script: str = "#!/usr/bin/env bash\nexit 0\n") -> None:
    path = bin_dir / name
    path.write_text(script, encoding="utf-8")
    path.chmod(0o755)
