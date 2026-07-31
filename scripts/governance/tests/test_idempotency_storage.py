from __future__ import annotations

import importlib.util
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "check_idempotency_storage.py"
SPEC = importlib.util.spec_from_file_location("check_idempotency_storage", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


def write_fixture(tmp_path: Path, files: dict[str, str], baseline: list[str]) -> tuple[Path, Path, Path]:
    root = tmp_path / "repo"
    source = root / "backend" / "crates" / "api" / "src"
    source.mkdir(parents=True)
    for name, content in files.items():
        path = source / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
    baseline_path = root / "governance" / "baseline.toml"
    baseline_path.parent.mkdir(parents=True)
    baseline_path.write_text(
        "version = 1\n" + "\n".join(f'[[direct_access]]\npath = "{path}"' for path in baseline),
        encoding="utf-8",
    )
    return root, source, baseline_path


def test_direct_access_must_be_in_baseline(tmp_path: Path):
    root, source, baseline = write_fixture(
        tmp_path,
        {
            "idempotency.rs": "SELECT * FROM idempotency_request",
            "legacy.rs": "SELECT * FROM idempotency_request",
            "new.rs": "SELECT * FROM idempotency_request",
        },
        ["backend/crates/api/src/legacy.rs"],
    )

    result = MODULE.check(source_root=source, root=root, baseline_path=baseline)

    assert result["ok"] is False
    assert result["new_violations"] == ["backend/crates/api/src/new.rs"]


def test_resolved_baseline_is_reported_and_does_not_fail(tmp_path: Path):
    root, source, baseline = write_fixture(
        tmp_path,
        {
            "idempotency.rs": "SELECT * FROM idempotency_request",
            "legacy.rs": "// migrated to shared module",
        },
        ["backend/crates/api/src/legacy.rs"],
    )

    result = MODULE.check(source_root=source, root=root, baseline_path=baseline)

    assert result["ok"] is True
    assert result["resolved"] == ["backend/crates/api/src/legacy.rs"]
    assert result["remaining_count"] == 0


def test_project_idempotency_baseline_is_empty_after_all_migrations():
    assert MODULE.load_baseline() == set()
    result = MODULE.check()
    assert result["message"] == "幂等表直接访问基线已归零"
