"""knowledge-graph.traceability 治理规则的正反例。"""

import json
import sys
from pathlib import Path

import pytest

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))

from check_knowledge_graph_traceability import validate_graph  # noqa: E402


FIXTURES = Path(__file__).parent / "fixtures" / "knowledge_graph_traceability"


def test_valid_fixture_passes():
    result = validate_graph(FIXTURES / "valid.json")

    assert result["ok"] is True
    assert result["counts"]["edges"] == 8
    assert result["traceability"]["resolvedEdgeCount"] == 8
    assert not result["issues"]


def test_invalid_fixture_reports_each_traceability_failure():
    result = validate_graph(FIXTURES / "invalid.json")

    assert result["ok"] is False
    messages = "\n".join(result["issues"])
    assert "duplicate node id" in messages
    assert "dangling edge" in messages
    assert "sourceSpan" in messages
    assert "confidence" in messages
    assert "resolvedEdgeCount" in messages
    assert "schemaVersion" in messages
    assert "canonicalIdScheme" in messages


def test_json_fixture_is_stable_and_has_no_unknown_top_level_data():
    for path in (FIXTURES / "valid.json", FIXTURES / "invalid.json"):
        payload = json.loads(path.read_text(encoding="utf-8"))
        assert set(payload) >= {"nodes", "edges", "traceability"}


def test_migration_nodes_require_traceable_migration_edges(tmp_path):
    payload = json.loads((FIXTURES / "valid.json").read_text(encoding="utf-8"))
    payload["edges"] = [edge for edge in payload["edges"] if edge["type"] != "migrates"]
    payload["traceability"]["resolvedEdgeCount"] -= 1
    payload["traceability"]["sourceSpanEdgeCount"] -= 1
    payload["traceability"]["confidenceEdgeCount"] -= 1
    graph_path = tmp_path / "missing-migration-edge.json"
    graph_path.write_text(json.dumps(payload), encoding="utf-8")

    result = validate_graph(graph_path)

    assert result["ok"] is False
    assert any("migration" in issue.lower() for issue in result["issues"])


@pytest.mark.parametrize("path", [FIXTURES / "valid.json"])
def test_domain_filter_keeps_stable_chain_validation(path):
    result = validate_graph(path, domain="h9-category-pdf")

    assert result["ok"] is True
    assert result["domain"] == "h9-category-pdf"
