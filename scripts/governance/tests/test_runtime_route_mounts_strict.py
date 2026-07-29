"""正式 OpenAPI 路由族必须进入 T4 strict 挂载检查。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import check_runtime_route_mounts as check


def test_strict_route_specs_cover_reports_and_parameter_mapping():
    paths = {spec.openapi_path for spec in check.STRICT_ROUTE_MOUNT_SPECS}

    assert paths == {
        "/api/v1/reports/query",
        "/api/v1/parameter-mapping/map",
    }


def test_current_strict_scan_tracks_remaining_unimplemented_route_families():
    issues = check.scan(strict=True)
    messages = {issue.message for issue in issues}

    assert any("DELETE /api/v1/inbound/receiving-orders/{id}" in message for message in messages)
    assert any("POST /api/v1/reports/query" in message for message in messages)
    assert not any("POST /api/v1/parameter-mapping/map" in message for message in messages)


def test_runtime_operation_parser_tracks_method_and_normalizes_path_parameter():
    source = '''
        Router::new()
            .route(
                "/api/v1/items/:id",
                get(read_item).patch(update_item),
            )
    '''

    assert check.operations_from_sources([source]) == {
        "GET /api/v1/items/{id}",
        "PATCH /api/v1/items/{id}",
    }


def test_axum_07_runtime_parser_rejects_openapi_brace_parameter_syntax():
    source = '''
        Router::new()
            .route("/api/v1/items/{id}", get(read_item))
            .route("/api/v1/items/:id/confirm", post(confirm_item))
    '''

    assert check.invalid_axum_route_paths(source) == {"/api/v1/items/{id}"}
