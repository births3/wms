"""Rust 层依赖、unsafe/unwrap 与 handler 覆盖治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _write_api_lib(tmp_path: Path, lines: list[str]) -> tuple[Path, Path]:
    api_crate = tmp_path / "api"
    src = api_crate / "src"
    src.mkdir(parents=True)
    (src / "lib.rs").write_text("\n".join(lines), encoding="utf-8")
    return api_crate, src / "lib.rs"


def test_layer_dependency_detects_forbidden_refs():
    """domain 层禁止引用 api / infra / axum / sqlx。"""
    from check_layer_dependency import find_domain_dependency_issues

    issues = find_domain_dependency_issues(
        "\n".join([
            "use wms_api::ApiDoc;",
            "use axum::Router;",
            "use sqlx::PgPool;",
            "use crate::infra::repo::Repo;",
        ]),
        path="backend/crates/domain/src/lib.rs",
    )

    assert [issue.kind for issue in issues] == ["api", "axum", "sqlx", "infra"]


def test_unsafe_and_unwrap_ignores_comments_and_test_shortcuts():
    """注释中的关键字不误报，测试代码允许 unwrap/expect/panic。"""
    from check_unsafe_and_unwrap import find_unsafe_unwrap_issues

    issues = find_unsafe_unwrap_issues(
        "\n".join([
            "// unsafe { ptr.read() }",
            "/* another unwrap() mention */",
            "#[cfg(test)]",
            "mod tests {",
            "  #[test]",
            "  fn allows_test_shortcuts() {",
            "    let value = result.expect(\"test setup\");",
            "    let other = option.unwrap();",
            "    panic!(\"expected test failure\");",
            "  }",
            "}",
        ]),
        path="backend/crates/api/src/lib.rs",
    )

    assert issues == []


def test_unsafe_and_unwrap_treats_cfg_test_module_file_as_test_code():
    from check_unsafe_and_unwrap import find_unsafe_unwrap_issues

    issues = find_unsafe_unwrap_issues(
        'let value = result.expect("test fixture should exist");',
        path="backend/crates/api/src/audit/tests.rs",
    )

    assert issues == []


def test_unsafe_and_unwrap_treats_cfg_test_include_as_test_code():
    from check_unsafe_and_unwrap import find_unsafe_unwrap_issues

    issues = find_unsafe_unwrap_issues(
        'let value = result.expect("included fixture should exist");',
        path="backend/crates/api/src/wave3_handlers_part1.rs",
        test_only=True,
    )

    assert issues == []


def test_unsafe_and_unwrap_treats_external_cfg_test_module_as_test_code(tmp_path):
    from check_unsafe_and_unwrap import _test_only_include_files

    src = tmp_path / "src"
    src.mkdir()
    (src / "mod.rs").write_text(
        "#[cfg(test)]\nmod partition_tests;\n",
        encoding="utf-8",
    )
    test_module = src / "partition_tests.rs"
    test_module.write_text(
        'fn fixture() { result.expect("test fixture should exist"); }\n',
        encoding="utf-8",
    )

    assert _test_only_include_files(tmp_path) == {test_module.resolve()}


def test_unsafe_and_unwrap_detects_real_production_usage():
    """生产路径 unsafe / unwrap / expect / panic 必须报错。"""
    from check_unsafe_and_unwrap import find_unsafe_unwrap_issues

    issues = find_unsafe_unwrap_issues(
        "\n".join([
            "unsafe { core::ptr::read(p) };",
            "let value = option.unwrap();",
            "let value = result.expect(\"must exist\");",
            "panic!(\"unreachable\");",
        ]),
        path="backend/crates/api/src/lib.rs",
    )

    assert [issue.kind for issue in issues] == ["unsafe", "unwrap", "expect", "panic"]


def test_handler_test_coverage_extracts_unique_paths():
    """utoipa path 抽取应去重。"""
    from check_handler_test_coverage import extract_utoipa_paths

    paths = extract_utoipa_paths(
        '\n'.join([
            '#[utoipa::path(path = "/api/v1/healthz")]',
            '#[utoipa::path(path = "/api/v1/auth/login")]',
            '#[utoipa::path(path = "/api/v1/healthz")]',
        ])
    )

    assert paths == ["/api/v1/healthz", "/api/v1/auth/login"]


def test_handler_test_coverage_extracts_helper_format_paths():
    from check_handler_test_coverage import extract_test_path_literals

    assert extract_test_path_literals(
        'request_json("PATCH", &format!("/api/v1/master-data/warehouse-zones/{}", id));'
    ) == ["/api/v1/master-data/warehouse-zones/{}"]


def test_handler_test_coverage_requires_path_literals(tmp_path):
    """有 utoipa path 但测试未覆盖 path 字面量时应失败。"""
    from check_handler_test_coverage import check_handler_test_coverage

    api_crate, lib_rs = _write_api_lib(
        tmp_path,
        [
            '#[utoipa::path(path = "/api/v1/auth/login")]',
            '#[cfg(test)]',
            'mod tests {',
            '    #[test]',
            '    fn smoke() { assert!(true); }',
            '}',
        ],
    )

    issues, stats = check_handler_test_coverage(lib_rs, api_crate)
    assert [issue.kind for issue in issues] == ["missing_path_coverage"]
    assert stats["path_count"] == 1


def test_handler_test_coverage_requires_every_path(tmp_path):
    """新增 handler 时，不能只靠已覆盖的旧 path 通过 T2。"""
    from check_handler_test_coverage import check_handler_test_coverage

    api_crate, lib_rs = _write_api_lib(
        tmp_path,
        [
            '#[utoipa::path(path = "/api/v1/auth/login")]',
            'fn login() {}',
            '#[utoipa::path(path = "/api/v1/auth/me")]',
            'fn me() {}',
            '#[cfg(test)]',
            'mod tests {',
            '    #[test]',
            '    fn covers_login() { assert_eq!("/api/v1/auth/login", "/api/v1/auth/login"); }',
            '}',
        ],
    )

    issues, stats = check_handler_test_coverage(lib_rs, api_crate)
    assert [issue.kind for issue in issues] == ["partial_path_coverage"]
    assert stats["covered_paths"] == ["/api/v1/auth/login"]
    assert stats["missing_paths"] == ["/api/v1/auth/me"]


def test_handler_test_coverage_scans_split_openapi_path_modules(tmp_path):
    """OpenAPI path 拆出 lib.rs 后仍必须被统计，禁止 path_count=0 假绿。"""
    from check_handler_test_coverage import check_handler_test_coverage

    api_crate, lib_rs = _write_api_lib(tmp_path, ["mod openapi_paths;"])
    paths_dir = api_crate / "src" / "openapi_paths"
    paths_dir.mkdir()
    (paths_dir / "core.rs").write_text(
        '#[utoipa::path(get, path = "/api/v1/healthz")]\nfn healthz() {}\n',
        encoding="utf-8",
    )
    tests_dir = api_crate / "tests"
    tests_dir.mkdir()
    (tests_dir / "openapi.rs").write_text(
        '#[test]\nfn covers_healthz() {\n'
        '    let response = Request::builder().uri("/api/v1/healthz");\n'
        '    assert_eq!(response.status(), StatusCode::OK);\n'
        '}\n',
        encoding="utf-8",
    )

    issues, stats = check_handler_test_coverage(lib_rs, api_crate, strict=True)

    assert issues == []
    assert stats["path_count"] == 1
    assert stats["covered_paths"] == ["/api/v1/healthz"]


def test_handler_test_coverage_strict_rejects_path_string_only(tmp_path):
    from check_handler_test_coverage import check_handler_test_coverage

    api_crate, lib_rs = _write_api_lib(tmp_path, [
        '#[utoipa::path(get, path = "/api/v1/healthz")]\nfn healthz() {}',
    ])
    tests_dir = api_crate / "tests"
    tests_dir.mkdir()
    (tests_dir / "openapi.rs").write_text(
        '#[test]\nfn mentions_only() { assert_eq!("/api/v1/healthz", "/api/v1/healthz"); }\n',
        encoding="utf-8",
    )

    issues, stats = check_handler_test_coverage(lib_rs, api_crate, strict=True)

    assert [issue.kind for issue in issues] == ["missing_http_exercise"]
    assert stats["exercised_paths"] == []


def test_handler_test_coverage_strict_distinguishes_http_methods(tmp_path):
    from check_handler_test_coverage import check_handler_test_coverage

    api_crate, lib_rs = _write_api_lib(tmp_path, [
        '#[utoipa::path(get, path = "/api/v1/items")]\nfn list() {}\n'
        '#[utoipa::path(post, path = "/api/v1/items")]\nfn create() {}',
    ])
    tests_dir = api_crate / "tests"
    tests_dir.mkdir()
    (tests_dir / "items.rs").write_text(
        '#[test]\nfn lists() {\n'
        '    let response = Request::builder().uri("/api/v1/items");\n'
        '    assert_eq!(response.status(), StatusCode::OK);\n'
        '}\n',
        encoding="utf-8",
    )

    issues, stats = check_handler_test_coverage(lib_rs, api_crate, strict=True)

    assert [issue.kind for issue in issues] == ["missing_http_exercise"]
    assert stats["exercised_operations"] == ["GET /api/v1/items"]
    assert stats["unexercised_operations"] == ["POST /api/v1/items"]


def test_handler_test_coverage_strict_accepts_quoted_post_method(tmp_path):
    from check_handler_test_coverage import check_handler_test_coverage

    api_crate, lib_rs = _write_api_lib(tmp_path, [
        '#[utoipa::path(post, path = "/api/v1/items")]\nfn create() {}',
    ])
    tests_dir = api_crate / "tests"
    tests_dir.mkdir()
    (tests_dir / "items.rs").write_text(
        '#[test]\nfn creates() {\n'
        '    let response = Request::builder().method("POST").uri("/api/v1/items");\n'
        '    assert_eq!(response.status(), StatusCode::CREATED);\n'
        '}\n',
        encoding="utf-8",
    )

    issues, stats = check_handler_test_coverage(lib_rs, api_crate, strict=True)

    assert issues == []
    assert stats["exercised_operations"] == ["POST /api/v1/items"]


def test_handler_test_coverage_strict_accepts_method_after_uri(tmp_path):
    from check_handler_test_coverage import check_handler_test_coverage

    api_crate, lib_rs = _write_api_lib(tmp_path, [
        '#[utoipa::path(post, path = "/api/v1/items")]\nfn create() {}',
    ])
    tests_dir = api_crate / "tests"
    tests_dir.mkdir()
    (tests_dir / "items.rs").write_text(
        '#[test]\nfn creates() {\n'
        '    let response = Request::builder().uri("/api/v1/items").method("POST");\n'
        '    assert_eq!(response.status(), StatusCode::OK);\n'
        '}\n',
        encoding="utf-8",
    )

    issues, stats = check_handler_test_coverage(lib_rs, api_crate, strict=True)

    assert issues == []
    assert stats["exercised_operations"] == ["POST /api/v1/items"]


def test_handler_test_coverage_strict_rejects_unrelated_or_404_assertion(tmp_path):
    from check_handler_test_coverage import check_handler_test_coverage

    api_crate, lib_rs = _write_api_lib(tmp_path, [
        '#[utoipa::path(get, path = "/api/v1/items")]\nfn list() {}',
    ])
    tests_dir = api_crate / "tests"
    tests_dir.mkdir()
    (tests_dir / "items.rs").write_text(
        '#[test]\nfn missing_route() {\n'
        '    let response = Request::builder().uri("/api/v1/items");\n'
        '    assert_eq!(response.status(), StatusCode::NOT_FOUND);\n'
        '    let other = fake_response();\n'
        '    assert_eq!(other.status(), StatusCode::OK);\n'
        '}\n',
        encoding="utf-8",
    )

    issues, stats = check_handler_test_coverage(lib_rs, api_crate, strict=True)

    assert {issue.kind for issue in issues} == {"missing_http_exercise"}
    assert stats["exercised_operations"] == []


def test_handler_test_coverage_rejects_route_inventory_as_behavior_coverage(tmp_path):
    """只排除 404/405 的路由盘点不能替代逐接口行为测试。"""
    from check_handler_test_coverage import check_handler_test_coverage

    api_crate, lib_rs = _write_api_lib(tmp_path, [
        '#[utoipa::path(get, path = "/api/v1/items")]\nfn list() {}\n'
        '#[utoipa::path(post, path = "/api/v1/items")]\nfn create() {}',
    ])
    tests_dir = api_crate / "tests"
    tests_dir.mkdir()
    (tests_dir / "runtime_inventory.rs").write_text(
        '#[test]\nfn every_openapi_operation_is_mounted_by_the_runtime_router() {\n'
        '    let openapi = ApiDoc::openapi();\n'
        '    let request = Request::builder();\n'
        '    let rejected = StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED;\n'
        '    assert!(missing.is_empty());\n'
        '}\n',
        encoding="utf-8",
    )

    issues, stats = check_handler_test_coverage(lib_rs, api_crate, strict=True)

    assert {issue.kind for issue in issues} == {
        "missing_path_coverage",
        "missing_http_exercise",
    }
    assert stats["exercised_operations"] == []
