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


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_GRAPH = REPO_ROOT / ".ua" / "knowledge-graph.json"
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
