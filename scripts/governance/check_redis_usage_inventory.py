#!/usr/bin/env python3
"""检查仓库中的 Redis 入口是否已登记到治理清单。

该检查只读、不连接外部服务，保证新增 Redis 代码/配置先进入 REDIS-01 评估。
"""
from __future__ import annotations

import argparse
import json
import sys
try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 in the governance runner.
    import tomli as tomllib
from dataclasses import asdict, dataclass
from pathlib import Path


THIS = Path(__file__).resolve()
REPO_ROOT = THIS.parent.parent.parent
MANIFEST = REPO_ROOT / "governance" / "redis-usage-inventory.toml"

SCAN_ROOTS = (
    "backend/Cargo.toml",
    "backend/Cargo.lock",
    "backend/crates/api/Cargo.toml",
    "backend/crates/api/src",
    "backend/crates/api/examples",
    "backend/crates/api/tests",
    "backend/crates/domain/src",
    "apps/web-admin/src",
    "packages/api-client/src",
    "deploy/docker-compose.staging.yml",
)


@dataclass(frozen=True)
class Entry:
    path: str
    category: str
    required_terms: tuple[str, ...]


@dataclass(frozen=True)
class Issue:
    path: str
    kind: str
    detail: str


def load_entries(manifest: Path = MANIFEST) -> tuple[Entry, ...]:
    payload = tomllib.loads(manifest.read_text(encoding="utf-8"))
    return tuple(
        Entry(
            path=item["path"],
            category=item["category"],
            required_terms=tuple(item.get("required_terms", ())),
        )
        for item in payload.get("entry", ())
    )


def _iter_scan_files(repo_root: Path) -> set[str]:
    files: set[str] = set()
    for root in SCAN_ROOTS:
        path = repo_root / root
        if path.is_file():
            files.add(root)
        elif path.is_dir():
            for child in path.rglob("*"):
                if child.is_file() and ".git" not in child.parts:
                    files.add(child.relative_to(repo_root).as_posix())
    return files


def _contains_redis(path: Path) -> bool:
    try:
        return "redis" in path.read_text(encoding="utf-8").lower()
    except (OSError, UnicodeDecodeError):
        return False


def check_inventory(
    repo_root: Path = REPO_ROOT,
    manifest: Path = MANIFEST,
) -> tuple[list[Issue], dict[str, int]]:
    entries = load_entries(manifest)
    by_path = {entry.path: entry for entry in entries}
    issues: list[Issue] = []

    for entry in entries:
        path = repo_root / entry.path
        if not path.is_file():
            issues.append(Issue(entry.path, "missing_file", "清单路径不存在"))
            continue
        text = path.read_text(encoding="utf-8")
        for term in entry.required_terms:
            if term not in text:
                issues.append(Issue(entry.path, "missing_term", f"未找到登记锚点: {term}"))

    discovered = {
        path
        for path in _iter_scan_files(repo_root)
        if _contains_redis(repo_root / path)
    }
    for path in sorted(discovered - by_path.keys()):
        issues.append(Issue(path, "unregistered_path", "包含 Redis 引用但未登记"))

    return issues, {
        "manifest_entries": len(entries),
        "discovered_paths": len(discovered),
        "registered_paths": len(discovered & by_path.keys()),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    args = parser.parse_args(argv)

    issues, stats = check_inventory()
    payload = {
        "check": "check_redis_usage_inventory",
        "tier": "T1",
        "category": "基础设施治理",
        "scanned": stats,
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_redis_usage_inventory (T1, 基础设施治理)")
        print(
            "  · scanned:"
            f" manifest={stats['manifest_entries']},"
            f" discovered={stats['discovered_paths']},"
            f" registered={stats['registered_paths']}"
        )
        if issues:
            print(f"  ✘ 发现 {len(issues)} 个问题:")
            for issue in issues:
                print(f"    [{issue.kind}] {issue.path}: {issue.detail}")
        else:
            print("  ✓ Redis 引用均已登记并通过锚点检查")
    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
