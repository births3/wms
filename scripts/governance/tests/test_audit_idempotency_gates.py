"""G0-E：check_audit_trail_coverage / check_idempotency_test 真实规则单测。"""

from __future__ import annotations

import json
import sys
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS))

from check_audit_trail_coverage import (  # noqa: E402
    collect_openapi_audit_exempt_ops,
    collect_openapi_write_ops,
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


def test_audit_http_success_ignores_helpers_and_skipped_paths(tmp_path: Path):
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "helpers.rs").write_text(
        """
async fn login(app: Router) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

async fn submit_change(app: Router) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alert-definitions/change-requests")
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

    assert collect_write_success_tests(tests) == []


def test_audit_openapi_coverage_ignores_helper_function(tmp_path: Path):
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "helper.rs").write_text(
        """
async fn submit_change(app: Router, pool: PgPool) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alert-definitions/change-requests")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let audit_event: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_event")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audit_event, 1);
}
""",
        encoding="utf-8",
    )
    path = "/api/v1/alert-definitions/change-requests"
    operation = {
        "id": f"POST {path}",
        "method": "POST",
        "path": path,
        "operation_id": "create_alert_definition_change_request",
        "path_re": path_to_regex(path),
    }

    missing = find_missing_openapi_audit_tests([operation], tests)
    assert [item["id"] for item in missing] == [operation["id"]]


def test_audit_gate_honors_documented_read_only_post_exemption(tmp_path: Path):
    openapi_path = tmp_path / "openapi.json"
    openapi_path.write_text(
        json.dumps(
            {
                "paths": {
                    "/api/v1/demo/preview": {
                        "post": {
                            "operationId": "preview_demo",
                            "x-audit-exempt-reason": "read-only derived preview",
                        }
                    },
                    "/api/v1/demo/write": {
                        "post": {"operationId": "write_demo"}
                    },
                }
            }
        ),
        encoding="utf-8",
    )
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "preview.rs").write_text(
        """
#[sqlx::test]
async fn preview_demo() {
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/demo/preview")
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

    exempt_ops = collect_openapi_audit_exempt_ops(openapi_path)
    assert collect_write_success_tests(tests, exempt_ops) == []
    assert [operation["id"] for operation in collect_openapi_write_ops(openapi_path)] == [
        "POST /api/v1/demo/write"
    ]


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
#[sqlx::test]
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


def test_openapi_audit_evidence_must_be_in_same_test_function(tmp_path: Path):
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "mixed.rs").write_text(
        """
async fn receive_without_audit() {
    receive_receiving_order(...);
}
async fn unrelated_audit_test() {
    SELECT COUNT(*) FROM audit_event;
}
""",
        encoding="utf-8",
    )
    path = "/api/v1/inbound/receiving-orders/{id}/receive"
    ops = [{"id": f"POST {path}", "method": "POST", "path": path, "operation_id": "", "path_re": path_to_regex(path)}]

    assert find_missing_openapi_audit_tests(ops, tests)[0]["id"] == f"POST {path}"


def test_audit_request_construction_is_not_persistence_evidence(tmp_path: Path):
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "request_only.rs").write_text(
        """
async fn receive_with_audit_input_only() {
    let audit = AuditWriteRequest::new(...);
    receive_receiving_order(..., audit);
}
""",
        encoding="utf-8",
    )
    path = "/api/v1/inbound/receiving-orders/{id}/receive"
    ops = [{"id": f"POST {path}", "method": "POST", "path": path, "operation_id": "", "path_re": path_to_regex(path)}]

    assert find_missing_openapi_audit_tests(ops, tests)


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


def test_idempotency_evidence_must_be_in_same_test_function(tmp_path: Path):
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "mixed.rs").write_text(
        """
async fn receive_without_replay_check() {
    receive_receiving_order(...);
}
async fn unrelated_idempotency_test() {
    SELECT COUNT(*) FROM idempotency_request;
}
""",
        encoding="utf-8",
    )
    path = "/api/v1/inbound/receiving-orders/{id}/receive"
    required = [{"id": f"POST {path}", "method": "POST", "path": path, "operation_id": "", "path_re": path_to_regex(path)}]

    assert find_missing_idempotency_tests(required, tests)[0]["id"] == f"POST {path}"


def test_single_idempotency_header_is_not_replay_evidence(tmp_path: Path):
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "single.rs").write_text(
        """
async fn receive_once() {
    request.header("Idempotency-Key", "once-only");
    receive_receiving_order(...);
}
""",
        encoding="utf-8",
    )
    path = "/api/v1/inbound/receiving-orders/{id}/receive"
    required = [{"id": f"POST {path}", "method": "POST", "path": path, "operation_id": "", "path_re": path_to_regex(path)}]

    assert find_missing_idempotency_tests(required, tests)


def test_other_action_replay_does_not_cover_resend(tmp_path: Path):
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "mixed_actions.rs").write_text(
        """
async fn h4_send_replays_multiple_actions() {
    resend_record(..., "resend-1").await.expect("resend once");
    send_notification(..., "send-1").await.expect("send first");
    let replay = send_notification(..., "send-1").await.expect("send should replay");
    assert!(replay.replayed);
}
""",
        encoding="utf-8",
    )
    path = "/api/v1/wechat-notify/records/{record_id}/resend"
    required = [{"id": f"POST {path}", "method": "POST", "path": path, "operation_id": "resend_h4_notification_record", "path_re": path_to_regex(path)}]

    assert find_missing_idempotency_tests(required, tests)
