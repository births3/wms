"""tests for _diff.py"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from _diff import GateRule, load_gate_rules, match_rules


def test_gate_rule_matches_simple():
    r = GateRule(match="docs/**", checks=["check_doc_links"], tier="T1")
    assert r.matches("docs/governance.md")
    assert r.matches("docs/adr/0001-tech-stack.md")
    assert not r.matches("backend/src/main.rs")


def test_gate_rule_matches_deep_glob():
    r = GateRule(match="backend/crates/**", checks=["check_layer"], tier="T2")
    assert r.matches("backend/crates/domain/src/inbound/mod.rs")
    assert r.matches("backend/crates/.gitkeep")
    assert not r.matches("backend/Cargo.toml")


def test_gate_rule_matches_wildcard_extension():
    r = GateRule(match="*.md", checks=["check_doc_links"], tier="T1")
    assert r.matches("README.md")
    # pathspec gitwildmatch: *.md only matches root level without /
    # This is gitignore behavior: *.md matches any level
    assert r.matches("docs/governance.md")


def test_gate_rule_matches_double_star_middle():
    r = GateRule(match="backend/**/tests/**", checks=["check_tests"], tier="T2")
    assert r.matches("backend/crates/domain/tests/unit/test_x.rs")
    assert not r.matches("backend/crates/domain/src/main.rs")


def test_match_rules_multiple():
    rules = [
        GateRule(match="docs/**", checks=["check_a"], tier="T1"),
        GateRule(match="docs/adr/**", checks=["check_b"], tier="T1"),
    ]
    changed = ["docs/adr/0001-tech-stack.md", "backend/src/main.rs"]
    triggered = match_rules(changed, rules)
    assert "check_a" in triggered
    assert "check_b" in triggered
    assert "docs/adr/0001-tech-stack.md" in triggered["check_a"]
    assert "docs/adr/0001-tech-stack.md" in triggered["check_b"]
    # backend file should not trigger anything
    assert "backend/src/main.rs" not in triggered.get("check_a", [])


def test_load_gate_rules(tmp_path):
    toml_content = '''
[[rules]]
match = "src/**"
checks = ["lint"]
tier = "T1"

[[rules]]
match = "tests/**"
checks = ["test_coverage"]
tier = "T2"
'''
    p = tmp_path / "gate-rules.toml"
    p.write_text(toml_content)
    rules = load_gate_rules(p)
    assert len(rules) == 2
    assert rules[0].match == "src/**"
    assert rules[0].checks == ["lint"]
    assert rules[0].tier == "T1"
    assert rules[1].match == "tests/**"
    assert rules[1].tier == "T2"


def test_wave1_openapi_rules_cover_domain_and_generated_schema():
    """domain schema 与生成 TS 类型变更都必须触发 OpenAPI 同步检查。"""
    rules = [r for r in load_gate_rules() if r.tier == "T2"]
    triggered = match_rules([
        "backend/crates/domain/src/lib.rs",
        "packages/api-client/src/schema.ts",
    ], rules)

    assert "check_openapi_in_sync" in triggered
    assert "backend/crates/domain/src/lib.rs" in triggered["check_openapi_in_sync"]
    assert "packages/api-client/src/schema.ts" in triggered["check_openapi_in_sync"]
