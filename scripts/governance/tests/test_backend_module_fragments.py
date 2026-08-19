from __future__ import annotations

import importlib.util
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "check_backend_module_fragments.py"
SPEC = importlib.util.spec_from_file_location("check_backend_module_fragments", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


def write_fixture(
    tmp_path: Path, files: dict[str, str], baseline: list[tuple[str, str]]
) -> tuple[Path, Path, Path]:
    root = tmp_path / "repo"
    source = root / "backend" / "crates" / "api" / "src"
    source.mkdir(parents=True)
    for name, content in files.items():
        path = source / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
    baseline_path = root / "governance" / "backend-module-fragments-baseline.toml"
    baseline_path.parent.mkdir(parents=True)
    baseline_path.write_text(
        "version = 1\n"
        + "scope = \"backend.no-new-production-include\"\n"
        + "\n".join(
            f'[[fragments]]\nparent = "{parent}"\ninclude = "{include}"'
            for parent, include in baseline
        ),
        encoding="utf-8",
    )
    return root, source, baseline_path


def test_new_production_fragment_must_be_in_baseline(tmp_path: Path):
    root, source, baseline = write_fixture(
        tmp_path,
        {"repository.rs": 'include!("repository_part1.rs");'},
        [],
    )

    result = MODULE.check(source_root=source, root=root, baseline_path=baseline)

    assert result["ok"] is False
    assert result["new_violations"] == [
        "backend/crates/api/src/repository.rs::repository_part1.rs"
    ]


def test_new_semantic_fragment_is_also_baselined(tmp_path: Path):
    root, source, baseline = write_fixture(
        tmp_path,
        {"repository.rs": 'include!("domain/repository.rs");'},
        [],
    )

    result = MODULE.check(source_root=source, root=root, baseline_path=baseline)

    assert result["ok"] is False
    assert result["new_violations"] == [
        "backend/crates/api/src/repository.rs::domain/repository.rs"
    ]


def test_test_only_fixture_is_ignored(tmp_path: Path):
    root, source, baseline = write_fixture(
        tmp_path,
        {
            "tests.rs": '#[cfg(test)]\nmod tests { include!("tests_part1.rs"); }',
        },
        [],
    )

    result = MODULE.check(source_root=source, root=root, baseline_path=baseline)

    assert result["ok"] is True
    assert result["new_violations"] == []


def test_resolved_baseline_is_reported(tmp_path: Path):
    root, source, baseline = write_fixture(
        tmp_path,
        {"repository.rs": "// migrated to semantic modules"},
        [("backend/crates/api/src/repository.rs", "repository_part1.rs")],
    )

    result = MODULE.check(source_root=source, root=root, baseline_path=baseline)

    assert result["ok"] is True
    assert result["resolved"] == [
        "backend/crates/api/src/repository.rs::repository_part1.rs"
    ]
    assert result["remaining_count"] == 0
