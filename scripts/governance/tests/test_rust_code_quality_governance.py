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
