"""G0-E：check_audit_trail_coverage / check_idempotency_test 真实规则单测。"""

from __future__ import annotations

import json
import sys
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS))

from check_audit_trail_coverage import (  # noqa: E402
    collect_write_success_tests,
    file_covers_operation,
    find_missing_audit_assertions,
    find_missing_openapi_audit_tests,
    path_to_regex,
)
from check_idempotency_test import (  # noqa: E402
    collect_idempotency_required_ops,
    find_missing_idempotency_tests,
)


def test_audit_http_success_requires_audit_event_in_same_test(tmp_path: Path):
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "ok.rs").write_text(
        """
#[sqlx::test]
async fn receive_writes_audit(pool: PgPool) {
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/inbound/receiving-orders/1/receive")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_event WHERE resource_id = $1")
        .bind("1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(count >= 1);
}
""",
        encoding="utf-8",
    )
    (tests / "missing.rs").write_text(
        """
#[sqlx::test]
async fn receive_without_audit_check(pool: PgPool) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/inbound/receiving-orders/2/receive")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
""",
        encoding="utf-8",
    )

    samples = collect_write_success_tests(tests)
    missing = find_missing_audit_assertions(samples)
    ids = {item["id"] for item in missing}
    assert "POST /api/v1/inbound/receiving-orders/2/receive :: receive_without_audit_check" in ids
    assert "POST /api/v1/inbound/receiving-orders/1/receive :: receive_writes_audit" not in ids


def test_file_covers_repository_style_receive_action():
    text = """
async fn receiving_receipt_is_single_closure_and_idempotent(pool: PgPool) {
    repo.receive_receiving_order(&ctx, order.id, req, now, "idem-receive-1").await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_event").fetch_one(&pool).await.unwrap();
}
"""
    path = "/api/v1/inbound/receiving-orders/{id}/receive"
    assert file_covers_operation(text, path, path_to_regex(path), "")


def test_openapi_audit_requires_path_and_audit_in_same_file(tmp_path: Path):
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "covered.rs").write_text(
        """
async fn receive_ok() {
    receive_receiving_order(...);
    SELECT COUNT(*) FROM audit_event;
}
""",
        encoding="utf-8",
    )
    ops = [
        {
            "id": "POST /api/v1/inbound/receiving-orders/{id}/receive",
            "method": "POST",
            "path": "/api/v1/inbound/receiving-orders/{id}/receive",
            "operation_id": "",
            "path_re": path_to_regex("/api/v1/inbound/receiving-orders/{id}/receive"),
        },
        {
            "id": "POST /api/v1/tms/dispatches",
            "method": "POST",
            "path": "/api/v1/tms/dispatches",
            "operation_id": "",
            "path_re": path_to_regex("/api/v1/tms/dispatches"),
        },
    ]
    missing = find_missing_openapi_audit_tests(ops, tests)
    ids = {m["id"] for m in missing}
    assert "POST /api/v1/inbound/receiving-orders/{id}/receive" not in ids
    assert "POST /api/v1/tms/dispatches" in ids


def test_idempotency_gate_matches_repo_tests_and_flags_uncovered(tmp_path: Path):
    openapi = {
        "paths": {
            "/api/v1/inbound/receiving-orders/{id}/receive": {
                "post": {
                    "operationId": "receive_order",
                    "parameters": [{"name": "Idempotency-Key", "in": "header", "required": True}],
                }
            },
            "/api/v1/demo/exempt": {
                "post": {
                    "operationId": "demo_exempt",
                    "x-idempotency-exempt-reason": "read-like side effect free",
                }
            },
            "/api/v1/inbound/receiving-orders/{id}/reject": {
                "post": {
                    "operationId": "reject_order",
                    "parameters": [{"name": "Idempotency-Key", "in": "header"}],
                }
            },
            "/api/v1/tms/dispatches": {
                "post": {
                    "parameters": [{"name": "Idempotency-Key", "in": "header"}],
                }
            },
        }
    }
    openapi_path = tmp_path / "openapi.json"
    openapi_path.write_text(json.dumps(openapi), encoding="utf-8")

    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "receive.rs").write_text(
        """
async fn receiving_receipt_is_idempotent(pool: PgPool) {
    repo.receive_receiving_order(&ctx, id, req, now, "k1").await.unwrap();
    let _: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM idempotency_request WHERE idempotency_key = $1"
    ).bind("k1").fetch_one(&pool).await.unwrap();
}
async fn receiving_order_reject_closes_order_and_replays_idempotently(pool: PgPool) {
    repo.reject_receiving_order(&ctx, id, req, now, "idem-reject-1").await.unwrap();
    SELECT COUNT(*) FROM idempotency_request;
}
""",
        encoding="utf-8",
    )

    required = collect_idempotency_required_ops(openapi_path)
    assert any(op["id"] == "POST /api/v1/inbound/receiving-orders/{id}/receive" for op in required)
    assert all(op["id"] != "POST /api/v1/demo/exempt" for op in required)

    missing = find_missing_idempotency_tests(required, tests)
    ids = {item["id"] for item in missing}
    assert "POST /api/v1/inbound/receiving-orders/{id}/receive" not in ids
    assert "POST /api/v1/inbound/receiving-orders/{id}/reject" not in ids
    assert "POST /api/v1/tms/dispatches" in ids
