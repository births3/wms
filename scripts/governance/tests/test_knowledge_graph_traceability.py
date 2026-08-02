"""knowledge-graph.traceability 治理规则的正反例。"""

import json
import shutil
import sys
from pathlib import Path

import pytest

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))

from check_knowledge_graph_traceability import (  # noqa: E402
    _resolve_graph_file,
    validate_graph,
)


FIXTURES = Path(__file__).parent / "fixtures" / "knowledge_graph_traceability"
ALTER_FIXTURES = FIXTURES / "alter_case" / "backend" / "migrations"
ALTER_SQL_FIXTURE = ALTER_FIXTURES / "202601010003_alter.sql.txt"
CREATE_SQL_FIXTURE = ALTER_FIXTURES / "202601010001_outbound.sql.txt"


def _write_alter_graph(tmp_path: Path, source_spans: list[tuple[int, int]]) -> Path:
    payload = json.loads((FIXTURES / "valid.json").read_text(encoding="utf-8"))
    migration_path = "backend/migrations/202601010003_alter.sql"
    pseudo_id = f"table:{migration_path}:migration"
    table_id = "table:backend/migrations/202601010001_outbound.sql:outbound_order"
    payload["nodes"].append(
        {
            "id": table_id,
            "type": "table",
            "name": "outbound_order",
            "filePath": "backend/migrations/202601010001_outbound.sql",
            "tags": ["database", "migration"],
        }
    )
    payload["nodes"].append(
        {
            "id": pseudo_id,
            "type": "table",
            "name": "migration",
            "filePath": migration_path,
            "tags": ["database", "migration"],
        }
    )
    for line_start, line_end in source_spans:
        payload["edges"].append(
            {
                "source": pseudo_id,
                "target": table_id,
                "type": "migrates",
                "direction": "forward",
                "weight": 0.7,
                "sourceSpan": {
                    "filePath": migration_path,
                    "lineStart": line_start,
                    "lineEnd": line_end,
                },
                "confidence": 0.8,
            }
        )
    for field in ("resolvedEdgeCount", "sourceSpanEdgeCount", "confidenceEdgeCount"):
        payload["traceability"][field] += len(source_spans)

    (tmp_path / "backend/migrations").mkdir(parents=True)
    shutil.copyfile(
        CREATE_SQL_FIXTURE,
        tmp_path / "backend/migrations/202601010001_outbound.sql",
    )
    shutil.copyfile(ALTER_SQL_FIXTURE, tmp_path / migration_path)
    graph_path = tmp_path / "graph.json"
    graph_path.write_text(json.dumps(payload), encoding="utf-8")
    return graph_path


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


def test_alter_edges_require_the_event_source_span_line(tmp_path):
    result = validate_graph(_write_alter_graph(tmp_path, [(3, 3)]))

    assert result["ok"] is False
    assert any("ALTER TABLE" in issue and "line" in issue for issue in result["issues"])


def test_alter_edges_dedupe_repeated_events_and_report_counts(tmp_path):
    result = validate_graph(_write_alter_graph(tmp_path, [(2, 3)]))

    assert result["ok"] is True
    assert result["migrationTraceability"]["alterStatementCount"] == 2
    assert result["migrationTraceability"]["alterMappingCount"] == 1
    assert result["migrationTraceability"]["alterEdgeCount"] == 1


def test_alter_edges_reject_non_representative_duplicate_span(tmp_path):
    result = validate_graph(_write_alter_graph(tmp_path, [(5, 6)]))

    assert result["ok"] is False
    assert any("line 2-3" in issue for issue in result["issues"])


def test_alter_edges_require_catalog_creation_target(tmp_path):
    graph_path = _write_alter_graph(tmp_path, [(2, 3)])
    payload = json.loads(graph_path.read_text(encoding="utf-8"))
    wrong_id = "table:backend/migrations/202601010002_other.sql:outbound_order"
    payload["nodes"].append(
        {
            "id": wrong_id,
            "type": "table",
            "name": "outbound_order",
            "filePath": "backend/migrations/202601010002_other.sql",
            "tags": ["database", "migration"],
        }
    )
    for edge in payload["edges"]:
        if edge.get("type") == "migrates" and edge.get("source", "").endswith(
            "202601010003_alter.sql:migration"
        ):
            edge["target"] = wrong_id
    graph_path.write_text(json.dumps(payload), encoding="utf-8")

    result = validate_graph(graph_path)

    assert result["ok"] is False
    assert any("missing migrates edge" in issue for issue in result["issues"])


def test_alter_edges_require_migration_pseudo_node(tmp_path):
    graph_path = _write_alter_graph(tmp_path, [(2, 3)])
    payload = json.loads(graph_path.read_text(encoding="utf-8"))
    pseudo_id = "table:backend/migrations/202601010003_alter.sql:migration"
    payload["nodes"] = [node for node in payload["nodes"] if node.get("id") != pseudo_id]
    payload["edges"] = [edge for edge in payload["edges"] if edge.get("source") != pseudo_id]
    for field in ("resolvedEdgeCount", "sourceSpanEdgeCount", "confidenceEdgeCount"):
        payload["traceability"][field] -= 1
    graph_path.write_text(json.dumps(payload), encoding="utf-8")

    result = validate_graph(graph_path)

    assert result["ok"] is False
    assert result["migrationTraceability"]["alterStatementCount"] == 2
    assert result["migrationTraceability"]["alterMappingCount"] == 1
    assert any("missing migration pseudo node" in issue for issue in result["issues"])


def test_resolve_graph_file_rejects_absolute_and_parent_paths(tmp_path):
    graph_path = tmp_path / "graph.json"

    assert _resolve_graph_file(graph_path, "/tmp/escape.sql") is None
    assert _resolve_graph_file(graph_path, r"C:\tmp\escape.sql") is None
    assert _resolve_graph_file(graph_path, "../backend/migrations/escape.sql") is None


def test_resolve_graph_file_rejects_symlink_escape(tmp_path):
    graph_path = tmp_path / "graph.json"
    migration_dir = tmp_path / "backend" / "migrations"
    migration_dir.mkdir(parents=True)
    outside = tmp_path.parent / f"{tmp_path.name}-outside.sql"
    outside.write_text("ALTER TABLE outbound_order;", encoding="utf-8")
    (migration_dir / "escape.sql").symlink_to(outside)

    try:
        assert _resolve_graph_file(graph_path, "backend/migrations/escape.sql") is None
    finally:
        outside.unlink()


@pytest.mark.parametrize("path", [FIXTURES / "valid.json"])
def test_domain_filter_keeps_stable_chain_validation(path):
    result = validate_graph(path, domain="h9-category-pdf")

    assert result["ok"] is True
    assert result["domain"] == "h9-category-pdf"
