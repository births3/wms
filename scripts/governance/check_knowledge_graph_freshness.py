#!/usr/bin/env python3
"""校验 KG-02 的图谱新鲜度语义（源提交 + 输入指纹）。"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
UA_DIR_NAME = ".ua"
EXCLUDED_DIRS = (".ua", ".understand-anything")


def _git(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=root,
        text=True,
        capture_output=True,
        check=False,
    )


def _git_output(root: Path, *args: str) -> str:
    result = _git(root, *args)
    if result.returncode != 0:
        raise ValueError(result.stderr.strip() or f"git {' '.join(args)} failed")
    return result.stdout


def _paths(output: str) -> set[str]:
    return {path for path in output.split("\0") if path}


def _is_graph_output(path: str) -> bool:
    normalised = path.removeprefix("./")
    return any(
        normalised == directory or normalised.startswith(f"{directory}/")
        for directory in EXCLUDED_DIRS
    )


def _changed_input_paths(root: Path, source_commit: str, head_commit: str) -> list[str]:
    committed = _paths(
        _git_output(root, "diff", "--name-only", "-z", source_commit, head_commit, "--", ".")
    )
    unstaged = _paths(_git_output(root, "diff", "--name-only", "-z", "--", "."))
    staged = _paths(_git_output(root, "diff", "--cached", "--name-only", "-z", "--", "."))
    untracked = _paths(
        _git_output(root, "ls-files", "--others", "--exclude-standard", "-z", "--", ".")
    )
    return sorted(
        path
        for path in committed | unstaged | staged | untracked
        if not _is_graph_output(path)
    )


def _load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ValueError(f"missing {path}") from exc
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"cannot read {path}: {exc}") from exc


def _fingerprint_entries(root: Path) -> dict[str, Any]:
    payload = _load_json(root / UA_DIR_NAME / "fingerprints.json")
    files = payload.get("files") if isinstance(payload, dict) else None
    if not isinstance(files, dict) or not files:
        raise ValueError(".ua/fingerprints.json must contain a non-empty files object")
    if not all(
        isinstance(path, str)
        and not Path(path).is_absolute()
        and ".." not in Path(path).parts
        and isinstance(entry, dict)
        for path, entry in files.items()
    ):
        raise ValueError(".ua/fingerprints.json files entries are invalid")
    return files


def compute_current_input_fingerprint(root: Path | str) -> str:
    """按 fingerprints.json 的输入路径和当前内容计算稳定指纹。"""
    project_root = Path(root)
    files = _fingerprint_entries(project_root)
    entries: list[tuple[str, str]] = []
    for relative_path in sorted(files):
        file_path = project_root / relative_path
        content_hash = (
            hashlib.sha256(file_path.read_bytes()).hexdigest()
            if file_path.is_file()
            else "<missing>"
        )
        entries.append((relative_path, content_hash))
    canonical = json.dumps(entries, ensure_ascii=False, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def _invalid(reason: str) -> dict[str, Any]:
    return {"ok": False, "status": "invalid", "reason": reason, "changedFiles": []}


def check_freshness(root: Path | str) -> dict[str, Any]:
    project_root = Path(root).resolve()
    meta_path = project_root / UA_DIR_NAME / "meta.json"
    try:
        meta = _load_json(meta_path)
    except ValueError as exc:
        return _invalid(str(exc))
    if not isinstance(meta, dict):
        return _invalid(".ua/meta.json must be an object")
    if "gitCommitHash" in meta:
        return _invalid("legacy gitCommitHash is not accepted; use sourceCommitHash")
    source_commit = meta.get("sourceCommitHash")
    expected_fingerprint = meta.get("inputFingerprint")
    if not isinstance(source_commit, str) or not source_commit:
        return _invalid(".ua/meta.json requires sourceCommitHash")
    if not isinstance(expected_fingerprint, str) or not expected_fingerprint:
        return _invalid(".ua/meta.json requires inputFingerprint")
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        return _invalid("sourceCommitHash must be a full lowercase Git SHA-1")
    if not re.fullmatch(r"[0-9a-f]{64}", expected_fingerprint):
        return _invalid("inputFingerprint must be a SHA-256 hex digest")

    try:
        head_commit = _git_output(project_root, "rev-parse", "HEAD").strip()
        if _git(project_root, "cat-file", "-e", f"{source_commit}^{{commit}}").returncode != 0:
            return _invalid(f"source commit is unavailable: {source_commit}")
        current_fingerprint = compute_current_input_fingerprint(project_root)
        changed_files = _changed_input_paths(project_root, source_commit, head_commit)
    except ValueError as exc:
        return _invalid(str(exc))

    base: dict[str, Any] = {
        "sourceCommitHash": source_commit,
        "headCommitHash": head_commit,
        "inputFingerprint": expected_fingerprint,
        "currentInputFingerprint": current_fingerprint,
        "changedFiles": changed_files,
    }
    if _git(project_root, "merge-base", "--is-ancestor", source_commit, head_commit).returncode != 0:
        return {
            **base,
            "ok": False,
            "status": "stale",
            "reason": "source commit is not current HEAD or an ancestor",
        }
    if changed_files:
        return {
            **base,
            "ok": False,
            "status": "stale",
            "reason": "graph-relevant input changed after source commit",
        }
    if expected_fingerprint != current_fingerprint:
        return {
            **base,
            "ok": False,
            "status": "stale",
            "reason": "analysis input fingerprint differs from current inputs",
        }
    return {**base, "ok": True, "status": "fresh", "reason": "source and inputs are unchanged"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=REPO_ROOT)
    parser.add_argument("--json", action="store_true", dest="as_json")
    args = parser.parse_args()
    result = check_freshness(args.root)
    if args.as_json:
        print(json.dumps({
            "check": "check_knowledge_graph_freshness",
            "tier": "T2",
            "category": "图谱治理",
            **result,
        }, ensure_ascii=False, sort_keys=True))
    else:
        print(f"knowledge graph: {result['status']} — {result['reason']}")
        if result.get("changedFiles"):
            print("changed inputs: " + ", ".join(result["changedFiles"]))
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
