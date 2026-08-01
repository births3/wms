"""KG-02 图谱新鲜度语义的正反例。"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

import pytest

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))

from check_knowledge_graph_freshness import (  # noqa: E402
    check_freshness,
    compute_current_input_fingerprint,
)


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=root, text=True).strip()


def commit_all(root: Path, message: str) -> str:
    subprocess.run(["git", "add", "--all"], cwd=root, check=True)
    subprocess.run(["git", "commit", "-m", message], cwd=root, check=True)
    return git(root, "rev-parse", "HEAD")


def create_repo(tmp_path: Path) -> tuple[Path, str]:
    root = tmp_path / "repo"
    root.mkdir()
    git(root, "init", "--quiet")
    git(root, "config", "user.email", "freshness@example.com")
    git(root, "config", "user.name", "Freshness Tests")
    (root / "src").mkdir()
    (root / "src" / "index.ts").write_text("export const value = 1;\n", encoding="utf-8")
    return root, commit_all(root, "baseline")


def write_meta(root: Path, source_commit: str, *, input_fingerprint: str | None = None) -> None:
    ua = root / ".ua"
    ua.mkdir(exist_ok=True)
    content_hash = hashlib.sha256(
        (root / "src" / "index.ts").read_bytes()
    ).hexdigest()
    (ua / "fingerprints.json").write_text(
        json.dumps(
            {
                "version": "1.0.0",
                "files": {
                    "src/index.ts": {"contentHash": content_hash},
                },
            }
        ),
        encoding="utf-8",
    )
    actual = compute_current_input_fingerprint(root)
    (ua / "meta.json").write_text(
        json.dumps(
            {
                "lastAnalyzedAt": "2026-07-31T00:00:00Z",
                "sourceCommitHash": source_commit,
                "inputFingerprint": actual if input_fingerprint is None else input_fingerprint,
                "version": "1.0.0",
                "analyzedFiles": 1,
            }
        ),
        encoding="utf-8",
    )


def test_exact_source_commit_is_fresh(tmp_path: Path):
    root, source = create_repo(tmp_path)
    write_meta(root, source)

    result = check_freshness(root)

    assert result["ok"] is True
    assert result["status"] == "fresh"
    assert result["changedFiles"] == []


def test_graph_only_commit_after_source_is_fresh(tmp_path: Path):
    root, source = create_repo(tmp_path)
    write_meta(root, source)
    commit_all(root, "graph artifact")

    result = check_freshness(root)

    assert result["ok"] is True
    assert result["status"] == "fresh"
    assert result["sourceCommitHash"] == source


def test_ancestor_with_input_commit_is_stale(tmp_path: Path):
    root, source = create_repo(tmp_path)
    write_meta(root, source)
    commit_all(root, "graph artifact")
    (root / "src" / "index.ts").write_text("export const value = 2;\n", encoding="utf-8")
    commit_all(root, "source change")

    result = check_freshness(root)

    assert result["ok"] is False
    assert result["status"] == "stale"
    assert result["changedFiles"] == ["src/index.ts"]


def test_input_fingerprint_mismatch_is_stale(tmp_path: Path):
    root, source = create_repo(tmp_path)
    write_meta(root, source, input_fingerprint="0" * 64)

    result = check_freshness(root)

    assert result["ok"] is False
    assert result["status"] == "stale"
    assert "input fingerprint" in result["reason"]


def test_old_meta_field_is_rejected_without_compatibility_read(tmp_path: Path):
    root, source = create_repo(tmp_path)
    ua = root / ".ua"
    ua.mkdir()
    (ua / "meta.json").write_text(
        json.dumps(
            {
                "lastAnalyzedAt": "2026-07-31T00:00:00Z",
                "gitCommitHash": source,
                "version": "1.0.0",
                "analyzedFiles": 1,
            }
        ),
        encoding="utf-8",
    )

    result = check_freshness(root)

    assert result["ok"] is False
    assert result["status"] == "invalid"
    assert "gitCommitHash" in result["reason"]


@pytest.mark.parametrize("path", ["meta.json", "fingerprints.json"])
def test_graph_metadata_is_not_an_input_change(tmp_path: Path, path: str):
    root, source = create_repo(tmp_path)
    write_meta(root, source)
    commit_all(root, "graph artifact")
    target = root / ".ua" / path
    target.write_text(target.read_text(encoding="utf-8") + "\n", encoding="utf-8")

    result = check_freshness(root)

    assert result["status"] == "fresh"
