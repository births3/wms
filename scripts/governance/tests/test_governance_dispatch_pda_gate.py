"""PDA production gate 的 diff 入口与文档边界测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_pda_production_gate_diff_rules_cover_pda_decision_paths():
    """PDA 关键路径变更必须在 diff-driven T1 中触发生产门禁。"""
    from _diff import load_gate_rules, match_rules

    changed = [
        "docs/adr/0001-tech-stack.md",
        "docs/adr/0027-pda-offline-model.md",
        "docs/spikes/README.md",
        "docs/spikes/spike-005-rn-scanner.md",
        "docs/spikes/spike-005b-webview-capacitor-pda.md",
        "docs/runbooks/wave-3-pda-readiness.md",
        "docs/retros/wave-3-pda-runtime-evidence.json",
        "apps/pda-mobile/package.json",
        "package.json",
        "pnpm-workspace.yaml",
        "pnpm-lock.yaml",
    ]
    rules = [rule for rule in load_gate_rules() if rule.tier == "T1"]
    triggered = match_rules(changed, rules)

    assert set(changed) <= set(triggered.get("check_pda_production_gate", []))


def test_governance_doc_describes_full_pda_production_gate_scope():
    """governance.md 的 PDA production gate 描述必须覆盖脚本实际阻断范围。"""
    text = Path("docs/governance.md").read_text(encoding="utf-8")
    row = next(
        line for line in text.splitlines()
        if "`check_pda_production_gate.py`" in line
    )

    for term in ("workspace", "lockfile", "Spike accepted evidence", "候选一致性"):
        assert term in row


def test_adr0027_pda_gate_rejects_prod_and_production_boundaries():
    """ADR-0027 的 PDA gate 禁用边界必须同时覆盖 prod / production。"""
    text = Path("docs/adr/0027-pda-offline-model.md").read_text(encoding="utf-8")
    boundary_line = next(
        line for line in text.splitlines()
        if "禁止用 local / prod" in line and "PDA gate" in line
    )

    assert "prod / production" in boundary_line
