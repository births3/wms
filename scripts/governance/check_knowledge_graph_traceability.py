#!/usr/bin/env python3
"""校验 Understand-Anything 知识图谱的关系可追溯性（T2）。"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any

from generate_table_catalog import ALTER_TABLE_RE, collect_catalog, collect_schema_events


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_GRAPH = REPO_ROOT / ".ua" / "knowledge-graph.json"
TRACEABILITY_SCHEMA_VERSION = "1.0"
CANONICAL_ID_SCHEME = "<type>:<relative-path>[:symbol]"
KNOWN_TYPES = {
    "file", "function", "class", "module", "concept", "config", "document",
    "service", "table", "endpoint", "pipeline", "schema", "resource", "flow",
    "domain", "step",
}

DOMAIN_SPECS: dict[str, dict[str, Any]] = {
    "outbound-query": {
        "implementation": ("outbound", "outbound_query"),
        "test": ("outbound",),
        "table": ("outbound_order", "outbound_orders"),
        "edge_types": ("routes", "reads_from", "writes_to"),
    },
    "h9-category-pdf": {
        "implementation": ("category_pdf", "print_orchestration"),
        "test": ("category_pdf", "h9_print"),
        "table": ("category_pdf", "h9_category_pdf"),
        "edge_types": ("tested_by",),
    },
    "drug-inspection-download": {
        "implementation": ("drug_inspection", "drug-inspection"),
        "test": ("drug_inspection", "drug-inspection"),
        "table": ("drug_inspection", "drug_inspection"),
        "edge_types": ("tested_by", "calls", "routes"),
        "download": ("download", "file_attachment"),
    },
    "h2-audit": {
        "implementation": ("audit_query", "audit"),
        "test": ("audit", "wms_api_part"),
        "table": ("audit_event", "audit_chain_seal"),
        "edge_types": ("reads_from", "writes_to", "tested_by"),
    },
}


def _text(node: dict[str, Any]) -> str:
    return " ".join(
        str(node.get(key, "")) for key in ("id", "filePath", "name", "summary")
    ).lower()


def _matches(node: dict[str, Any], tokens: tuple[str, ...]) -> bool:
    haystack = _text(node)
    return any(token.lower() in haystack for token in tokens)


def _is_test(node: dict[str, Any]) -> bool:
    haystack = _text(node)
    return bool(re.search(r"(^|[/_.-])(test|tests|spec|specs|fixture)([/_.-]|$)", haystack))


def _valid_span(span: Any) -> bool:
    if not isinstance(span, dict) or not isinstance(span.get("filePath"), str):
        return False
    path = span["filePath"]
    if not path or Path(path).is_absolute() or ".." in Path(path).parts:
        return False
    start, end = span.get("lineStart"), span.get("lineEnd")
    if start is None and end is None:
        return True
    return (
        isinstance(start, int)
        and not isinstance(start, bool)
        and isinstance(end, int)
        and not isinstance(end, bool)
        and start > 0
        and end >= start
    )


def _valid_confidence(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
        and 0 <= float(value) <= 1
    )


def _is_migration_path(value: Any) -> bool:
    if not isinstance(value, str):
        return False
    normalized = value.replace("\\", "/")
    return "/migrations/" in f"/{normalized}" and normalized.endswith(".sql")


def _graph_source_root(graph_path: Path) -> Path:
    graph_parent = graph_path.resolve().parent
    if graph_parent.name in {".ua", ".understand-anything"}:
        return graph_parent.parent
    return graph_parent


def _resolve_graph_file(graph_path: Path, relative_path: str) -> Path | None:
    """Resolve a graph-relative regular file without allowing root escape."""
    normalized_path = relative_path.replace("\\", "/")
    normalized = Path(normalized_path)
    if (
        normalized.is_absolute()
        or re.match(r"^[A-Za-z]:/", normalized_path)
        or ".." in normalized.parts
    ):
        return None
    source_root = _graph_source_root(graph_path).resolve()
    candidate = (source_root / normalized).resolve()
    try:
        candidate.relative_to(source_root)
    except ValueError:
        return None
    if candidate.is_symlink() or not candidate.is_file():
        return None
    return candidate


def _migration_alter_statements(sql: str) -> list[dict[str, Any]]:
    statements: list[dict[str, Any]] = []
    # Keep event discovery identical to generate_table_catalog.collect_schema_events;
    # deduplication happens later per (migration path, table) mapping.
    for match in ALTER_TABLE_RE.finditer(sql):
        line_start = sql.count("\n", 0, match.start()) + 1
        semicolon = sql.find(";", match.end())
        line_end = (
            sql.count("\n", 0, semicolon) + 1
            if semicolon >= 0
            else line_start
        )
        table = match.group(1).strip('"').lower()
        statements.append(
            {
                "table": table,
                "lineStart": line_start,
                "lineEnd": line_end,
            }
        )
    return statements


def _validate_migration_alter_edges(
    graph_path: Path,
    nodes: list[Any],
    edges: list[Any],
) -> tuple[list[str], int, int, int]:
    """Require one exact pseudo-migration -> canonical-table edge per ALTER mapping."""
    issues: list[str] = []
    node_ids = {
        node.get("id")
        for node in nodes
        if isinstance(node, dict) and isinstance(node.get("id"), str)
    }
    pseudo_nodes: dict[str, str] = {}
    for node in nodes:
        if not isinstance(node, dict):
            continue
        node_id = node.get("id")
        file_path = node.get("filePath")
        if not isinstance(node_id, str) or not isinstance(file_path, str):
            continue
        if (
            node.get("type") == "table"
            and node_id.endswith(":migration")
            and _is_migration_path(file_path)
        ):
            pseudo_nodes[file_path.replace("\\", "/")] = node_id

    migration_edges = [
        edge
        for edge in edges
        if isinstance(edge, dict) and edge.get("type") == "migrates"
    ]
    catalog_root = _graph_source_root(graph_path)
    canonical_ids = {
        table.name.lower(): f"table:{table.migration}:{table.name}"
        for table in collect_catalog(catalog_root)
    }
    expected_mappings = {
        (event.migration.replace("\\", "/"), event.table.lower())
        for event in collect_schema_events(catalog_root)
        if event.kind == "alter"
    }
    statements_by_mapping: dict[tuple[str, str], list[dict[str, int]]] = {}
    alter_statement_count = 0
    missing_source_paths: set[str] = set()
    for migration_path in sorted({path for path, _ in expected_mappings}):
        source_path = _resolve_graph_file(graph_path, migration_path)
        if source_path is None:
            missing_source_paths.add(migration_path)
            continue
        try:
            sql = source_path.read_text(encoding="utf-8")
        except OSError as exc:
            issues.append(f"migration source file cannot be read: {migration_path}: {exc}")
            missing_source_paths.add(migration_path)
            continue
        statements = _migration_alter_statements(sql)
        alter_statement_count += len(statements)
        for statement in statements:
            statements_by_mapping.setdefault(
                (migration_path, statement["table"]), []
            ).append(
                {
                    "lineStart": statement["lineStart"],
                    "lineEnd": statement["lineEnd"],
                }
            )

    alter_mapping_count = len(expected_mappings)
    matched_edge_count = 0
    for migration_path, table_name in sorted(expected_mappings):
        if migration_path in missing_source_paths:
            issues.append(f"migration source file not found: {migration_path}")
            continue
        spans = statements_by_mapping.get((migration_path, table_name), [])
        if not spans:
            issues.append(
                f"ALTER TABLE {table_name} at {migration_path} has no parsed source statement"
            )
            continue
        canonical_id = canonical_ids.get(table_name)
        representative = min(
            spans, key=lambda span: (span["lineStart"], span["lineEnd"])
        )
        if canonical_id is None:
            issues.append(
                f"ALTER TABLE {table_name} at {migration_path}:"
                f" line {representative['lineStart']} has no table-catalog creation mapping"
            )
            continue
        if canonical_id not in node_ids:
            issues.append(
                f"ALTER TABLE {table_name} at {migration_path}:"
                f" line {representative['lineStart']} missing canonical table node {canonical_id}"
            )
            continue
        pseudo_id = pseudo_nodes.get(migration_path)
        if pseudo_id is None:
            issues.append(
                f"ALTER TABLE {table_name} at {migration_path}:"
                f" line {representative['lineStart']} missing migration pseudo node"
            )
            continue
        matching_edges = [
            edge
            for edge in migration_edges
            if edge.get("source") == pseudo_id
            and edge.get("target") == canonical_id
        ]
        expected_span = {
            "filePath": migration_path,
            "lineStart": representative["lineStart"],
            "lineEnd": representative["lineEnd"],
        }
        if (
            len(matching_edges) == 1
            and matching_edges[0].get("sourceSpan") == expected_span
        ):
            matched_edge_count += 1
            continue
        existing_spans = [edge.get("sourceSpan") for edge in matching_edges]
        suffix = f"; existing sourceSpans={existing_spans!r}" if existing_spans else ""
        multiplicity = (
            f"; expected exactly one edge, found {len(matching_edges)}"
            if len(matching_edges) > 1
            else ""
        )
        issues.append(
            f"ALTER TABLE {table_name} at {migration_path}:"
            f" line {representative['lineStart']}-{representative['lineEnd']} missing "
            f"migrates edge from {pseudo_id} with exact sourceSpan"
            f"{suffix}{multiplicity or '; expected exactly one edge'}"
        )
    return issues, alter_statement_count, alter_mapping_count, matched_edge_count


def _stable_chain(graph: dict[str, Any], domain: str) -> dict[str, Any]:
    spec = DOMAIN_SPECS[domain]
    nodes = graph.get("nodes", [])
    edges = graph.get("edges", [])
    implementations = [
        node for node in nodes
        if isinstance(node, dict)
        and _matches(node, spec["implementation"])
        and not _is_test(node)
    ]
    tests = [
        node for node in nodes
        if isinstance(node, dict) and _matches(node, spec["test"]) and _is_test(node)
    ]
    tables = [
        node for node in nodes
        if isinstance(node, dict)
        and node.get("type") == "table"
        and _matches(node, spec["table"])
    ]
    implementation_ids = {node.get("id") for node in implementations}
    test_ids = {node.get("id") for node in tests}
    table_ids = {node.get("id") for node in tables}
    tested = any(
        edge.get("type") == "tested_by"
        and edge.get("source") in implementation_ids
        and edge.get("target") in test_ids
        for edge in edges
    )
    data_link = any(
        edge.get("source") in implementation_ids
        and edge.get("target") in table_ids
        and edge.get("type") in spec["edge_types"]
        for edge in edges
    )
    download = True
    if spec.get("download"):
        download = any(
            _matches(node, spec["download"])
            and ("download" in _text(node) or "file_attachment" in _text(node))
            for node in nodes
            if isinstance(node, dict)
        )
    route = any(
        edge.get("type") == "routes"
        and edge.get("target") in implementation_ids
        for edge in edges
    )
    issues: list[str] = []
    if not implementations:
        issues.append("missing implementation node")
    if not tests and domain != "h2-audit":
        issues.append("missing test/evidence node")
    if domain not in {"h9-category-pdf", "drug-inspection-download"} and not tables:
        issues.append("missing table/repository node")
    if domain in {"h9-category-pdf", "drug-inspection-download"} and not tested:
        issues.append("missing implementation -> tested_by -> test edge")
    if domain in {"outbound-query", "h2-audit"} and not data_link:
        issues.append("missing implementation -> table data-flow edge")
    if domain == "outbound-query" and not route:
        issues.append("missing operation -> handler route edge")
    if not download:
        issues.append("missing download/file-attachment node")
    return {
        "ok": not issues,
        "implementationCount": len(implementation_ids - {None}),
        "testCount": len(test_ids - {None}),
        "tableCount": len(table_ids - {None}),
        "sampleNodes": {
            "implementation": sorted(implementation_ids - {None})[:5],
            "test": sorted(test_ids - {None})[:5],
            "table": sorted(table_ids - {None})[:5],
        },
        "issues": issues,
    }


def validate_graph(path: Path | str, domain: str | None = None) -> dict[str, Any]:
    """返回稳定 JSON 结构；不修改图谱。"""
    graph_path = Path(path)
    issues: list[str] = []
    warnings: list[str] = []
    graph: dict[str, Any] = {}
    try:
        graph = json.loads(graph_path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        issues.append(f"graph file not found: {graph_path}")
    except (OSError, json.JSONDecodeError) as exc:
        issues.append(f"graph file cannot be read: {exc}")
    if not isinstance(graph, dict):
        issues.append("graph root must be an object")
        graph = {}

    nodes = graph.get("nodes")
    edges = graph.get("edges")
    if not isinstance(nodes, list):
        issues.append("graph.nodes must be an array")
        nodes = []
    if not isinstance(edges, list):
        issues.append("graph.edges must be an array")
        edges = []

    node_ids: set[str] = set()
    duplicate_ids: set[str] = set()
    nodes_by_id: dict[str, dict[str, Any]] = {}
    for index, node in enumerate(nodes):
        if not isinstance(node, dict):
            issues.append(f"node[{index}] must be an object")
            continue
        node_id, node_type = node.get("id"), node.get("type")
        if not isinstance(node_id, str) or not node_id:
            issues.append(f"node[{index}] missing canonical id")
            continue
        if node_id in node_ids:
            duplicate_ids.add(node_id)
        node_ids.add(node_id)
        nodes_by_id.setdefault(node_id, node)
        if node_type not in KNOWN_TYPES or not node_id.startswith(f"{node_type}:"):
            issues.append(f"node[{index}] non-canonical id: {node_id}")
    for node_id in sorted(duplicate_ids):
        issues.append(f"duplicate node id: {node_id}")

    valid_edges = 0
    source_span_count = 0
    confidence_count = 0
    relation_types: Counter[str] = Counter()
    for index, edge in enumerate(edges):
        if not isinstance(edge, dict):
            issues.append(f"edge[{index}] must be an object")
            continue
        relation_types[str(edge.get("type", "unknown"))] += 1
        if edge.get("source") not in node_ids or edge.get("target") not in node_ids:
            issues.append(f"dangling edge[{index}]: {edge.get('source')} -> {edge.get('target')}")
        span_ok = _valid_span(edge.get("sourceSpan"))
        confidence_ok = _valid_confidence(edge.get("confidence"))
        source_node = nodes_by_id.get(edge.get("source"))
        if (
            span_ok
            and isinstance(source_node, dict)
            and isinstance(source_node.get("filePath"), str)
            and edge["sourceSpan"]["filePath"] != source_node["filePath"]
        ):
            span_ok = False
            issues.append(f"edge[{index}] sourceSpan does not locate source node")
        if not span_ok:
            issues.append(f"edge[{index}] invalid sourceSpan")
        else:
            source_span_count += 1
        if not confidence_ok:
            issues.append(f"edge[{index}] invalid confidence")
        else:
            confidence_count += 1
        if edge.get("source") in node_ids and edge.get("target") in node_ids and span_ok and confidence_ok:
            valid_edges += 1

    traceability = graph.get("traceability")
    if not isinstance(traceability, dict):
        issues.append("missing graph.traceability")
        traceability = {}
    if traceability.get("schemaVersion") != TRACEABILITY_SCHEMA_VERSION:
        issues.append(
            "traceability.schemaVersion must be "
            f"{TRACEABILITY_SCHEMA_VERSION!r}"
        )
    if traceability.get("canonicalIdScheme") != CANONICAL_ID_SCHEME:
        issues.append(
            "traceability.canonicalIdScheme must be "
            f"{CANONICAL_ID_SCHEME!r}"
        )
    expected_counts = {
        "resolvedEdgeCount": valid_edges,
        "sourceSpanEdgeCount": source_span_count,
        "confidenceEdgeCount": confidence_count,
    }
    for field, expected in expected_counts.items():
        if traceability.get(field) != expected:
            issues.append(
                f"traceability.{field}={traceability.get(field)!r} does not match {expected}"
            )
    unresolved = traceability.get("unresolvedEdgeCount")
    if not isinstance(unresolved, int) or isinstance(unresolved, bool) or unresolved < 0:
        issues.append("traceability.unresolvedEdgeCount must be a non-negative integer")

    for relation in ("documents", "deploys", "migrates", "tested_by"):
        if not relation_types.get(relation):
            warnings.append(f"relation family absent: {relation}")

    migration_node_ids = {
        node.get("id")
        for node in nodes
        if isinstance(node, dict)
        and node.get("type") == "table"
        and _is_migration_path(node.get("filePath"))
    }
    migration_edges = [
        edge
        for edge in edges
        if isinstance(edge, dict)
        and edge.get("type") == "migrates"
        and _is_migration_path(
            edge.get("sourceSpan", {}).get("filePath")
            if isinstance(edge.get("sourceSpan"), dict)
            else None
        )
        and (
            edge.get("source") in migration_node_ids
            or edge.get("target") in migration_node_ids
        )
    ]
    if migration_node_ids and not migration_edges:
        issues.append(
            "migration table nodes require traceable migrates edges sourced from SQL migrations"
        )
    (
        alter_issues,
        alter_statement_count,
        alter_mapping_count,
        alter_edge_count,
    ) = _validate_migration_alter_edges(
        graph_path, nodes, edges
    )
    issues.extend(alter_issues)

    selected_domains = [domain] if domain else list(DOMAIN_SPECS)
    stable_chains: dict[str, dict[str, Any]] = {}
    for selected in selected_domains:
        if selected not in DOMAIN_SPECS:
            issues.append(f"unknown domain: {selected}")
            continue
        chain = _stable_chain({"nodes": nodes, "edges": edges}, selected)
        stable_chains[selected] = chain
        issues.extend(f"{selected}: {message}" for message in chain["issues"])

    return {
        "check": "check_knowledge_graph_traceability",
        "tier": "T2",
        "category": "知识图谱治理",
        "graphPath": str(graph_path),
        "domain": domain or "all",
        "counts": {"nodes": len(nodes), "edges": len(edges), "validEdges": valid_edges},
        "relationTypes": dict(sorted(relation_types.items())),
        "traceability": {
            "resolvedEdgeCount": traceability.get("resolvedEdgeCount"),
            "unresolvedEdgeCount": unresolved,
            "sourceSpanEdgeCount": traceability.get("sourceSpanEdgeCount"),
            "confidenceEdgeCount": traceability.get("confidenceEdgeCount"),
        },
        "migrationTraceability": {
            "migrationNodeCount": len(migration_node_ids - {None}),
            "migrationEdgeCount": len(migration_edges),
            "alterStatementCount": alter_statement_count,
            "alterMappingCount": alter_mapping_count,
            "alterEdgeCount": alter_edge_count,
        },
        "stableChains": stable_chains,
        "issues": issues,
        "warnings": warnings,
        "ok": not issues,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--graph", type=Path, default=DEFAULT_GRAPH)
    parser.add_argument("--domain", choices=sorted(DOMAIN_SPECS))
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    result = validate_graph(args.graph, args.domain)
    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        print(f"{result['check']} ({result['tier']}, {result['category']})")
        print(f"  · graph: {result['graphPath']}")
        print(f"  · nodes={result['counts']['nodes']} edges={result['counts']['edges']}")
        for issue in result["issues"]:
            print(f"  ✘ {issue}")
        for warning in result["warnings"]:
            print(f"  ⚠ {warning}")
        if result["ok"]:
            print("  ✓ knowledge-graph.traceability 通过")
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # noqa: BLE001
        print(f"script error: {exc}", file=sys.stderr)
        sys.exit(2)
