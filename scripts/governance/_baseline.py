"""治理脚本公共库：baseline 机制

详细规则：见 docs/adr/0003-governance-model.md §机制 3

baseline 文件位置：governance/baselines/<check_name>.json

文件格式：
{
  "check": "<check_name>",
  "version": 1,
  "generated_at": "ISO-8601",
  "ignored": [
    {
      "id": "<violation_id>",
      "reason": "<why temporarily ignored>",
      "added_at": "YYYY-MM-DD",
      "expires_at": "YYYY-MM-DD"  // optional
    }
  ]
}

接口：
- load_baseline(check_name)               读取 baseline
- save_baseline(check_name, ignored)      写入 baseline（自动收缩）
- diff_violations(current, baseline)      返回 (新增, 已修复)
- evaluate(check_name, current_ids, ...)  完整评估并按需写回

退出码贡献：
- 有新增违规 → 1
- 仅 baseline 收缩 → 0（但写回 baseline 文件）
"""
from __future__ import annotations

import json
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable


# 仓库根目录（脚本被符号链接调用时仍正确）
_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
BASELINE_DIR = REPO_ROOT / "governance" / "baselines"


@dataclass
class BaselineEntry:
    id: str
    reason: str = ""
    added_at: str = ""
    expires_at: str | None = None

    @classmethod
    def from_dict(cls, d: dict) -> "BaselineEntry":
        return cls(
            id=d["id"],
            reason=d.get("reason", ""),
            added_at=d.get("added_at", ""),
            expires_at=d.get("expires_at"),
        )

    def to_dict(self) -> dict:
        out = {"id": self.id, "reason": self.reason, "added_at": self.added_at}
        if self.expires_at:
            out["expires_at"] = self.expires_at
        return out


@dataclass
class Baseline:
    check: str
    version: int = 1
    generated_at: str = ""
    ignored: list[BaselineEntry] = field(default_factory=list)

    @classmethod
    def empty(cls, check: str) -> "Baseline":
        return cls(check=check, generated_at=_now_iso())

    @classmethod
    def from_dict(cls, d: dict) -> "Baseline":
        return cls(
            check=d.get("check", ""),
            version=d.get("version", 1),
            generated_at=d.get("generated_at", ""),
            ignored=[BaselineEntry.from_dict(x) for x in d.get("ignored", [])],
        )

    def to_dict(self) -> dict:
        return {
            "check": self.check,
            "version": self.version,
            "generated_at": self.generated_at,
            "ignored": [e.to_dict() for e in self.ignored],
        }

    def ignored_ids(self) -> set[str]:
        return {e.id for e in self.ignored}


def _now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def baseline_path(check_name: str) -> Path:
    return BASELINE_DIR / f"{check_name}.json"


def load_baseline(check_name: str) -> Baseline:
    """读取 baseline 文件；不存在则返回空 baseline。"""
    p = baseline_path(check_name)
    if not p.exists():
        return Baseline.empty(check_name)
    try:
        data = json.loads(p.read_text(encoding="utf-8"))
        return Baseline.from_dict(data)
    except (json.JSONDecodeError, KeyError) as e:
        raise RuntimeError(f"invalid baseline file {p}: {e}") from e


def save_baseline(baseline: Baseline) -> None:
    """写回 baseline 文件（按 id 排序，确保稳定 diff）。"""
    BASELINE_DIR.mkdir(parents=True, exist_ok=True)
    baseline.ignored.sort(key=lambda e: e.id)
    baseline.generated_at = _now_iso()
    p = baseline_path(baseline.check)
    p.write_text(
        json.dumps(baseline.to_dict(), ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


@dataclass
class EvaluateResult:
    new_violations: list[str]
    resolved: list[str]
    still_present: list[str]
    expired: list[str]

    @property
    def has_failure(self) -> bool:
        # 新增违规或已过期 → 失败
        return bool(self.new_violations) or bool(self.expired)


def evaluate(
    check_name: str,
    current_ids: Iterable[str],
    *,
    auto_shrink: bool = True,
) -> EvaluateResult:
    """与 baseline 对比当前违规集。

    - current_ids: 本次扫描发现的所有违规 id
    - auto_shrink: 默认 True，已修复的从 baseline 移除并写回

    返回:
    - new_violations: 既不在 baseline 也不在 current 之外的新违规（fail 触发）
    - resolved: 在 baseline 中但当前已不存在（自动收缩）
    - still_present: baseline 中且当前仍存在
    - expired: 已过期但仍存在的 baseline 条目（fail 触发）
    """
    baseline = load_baseline(check_name)
    current_set = set(current_ids)
    baseline_ids = baseline.ignored_ids()

    new_violations = sorted(current_set - baseline_ids)
    resolved = sorted(baseline_ids - current_set)
    still_present = sorted(baseline_ids & current_set)

    today = datetime.now(timezone.utc).date().isoformat()
    expired: list[str] = []
    for e in baseline.ignored:
        if e.expires_at and e.expires_at < today and e.id in current_set:
            expired.append(e.id)

    if auto_shrink and resolved:
        baseline.ignored = [e for e in baseline.ignored if e.id not in resolved]
        save_baseline(baseline)

    return EvaluateResult(
        new_violations=new_violations,
        resolved=resolved,
        still_present=still_present,
        expired=expired,
    )


def format_report(check_name: str, result: EvaluateResult) -> str:
    lines = [f"baseline report: {check_name}"]
    if result.new_violations:
        lines.append(f"  ✘ new violations ({len(result.new_violations)}):")
        for v in result.new_violations:
            lines.append(f"      + {v}")
    if result.expired:
        lines.append(f"  ✘ expired baseline entries ({len(result.expired)}):")
        for v in result.expired:
            lines.append(f"      ! {v}")
    if result.resolved:
        lines.append(f"  ✓ resolved & shrunk ({len(result.resolved)}):")
        for v in result.resolved:
            lines.append(f"      - {v}")
    if result.still_present:
        lines.append(f"  · still in baseline ({len(result.still_present)}): suppressed")
    if not (result.new_violations or result.expired):
        lines.append("  ✓ no new violations")
    return "\n".join(lines)
