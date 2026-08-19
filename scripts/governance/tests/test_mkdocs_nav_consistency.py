"""MkDocs 导航一致性治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_parse_mkdocs_nav_paths_normalizes_docs_prefix():
    """mkdocs nav 中的相对 docs 路径应统一归一化为 docs/...。"""
    from check_mkdocs_nav_consistency import parse_mkdocs_nav_paths

    paths = parse_mkdocs_nav_paths(
        """
        nav:
          - 治理:
            - 编码规范: coding-standards.md
            - ADR: adr/README.md
            - 外部: https://example.com/readme.md
        """
    )

    assert "docs/coding-standards.md" in paths
    assert "docs/adr/README.md" in paths
    assert not any(path.startswith("docs/https://") for path in paths)


def test_parse_mkdocs_nav_paths_ignores_exclude_docs():
    """只解析 nav 段，不能把 exclude_docs 中的路径误判为已导航。"""
    from check_mkdocs_nav_consistency import parse_mkdocs_nav_paths

    paths = parse_mkdocs_nav_paths(
        """
        nav:
          - 首页: index.md

        exclude_docs: |
          retros/README.md
        """
    )

    assert "docs/index.md" in paths
    assert "docs/retros/README.md" not in paths


def test_parse_agents_required_docs_reads_required_section_only():
    """只解析 AGENTS 必读文档段，避免把全量业务索引当强制 nav。"""
    from check_mkdocs_nav_consistency import parse_agents_required_docs

    docs = parse_agents_required_docs(
        """
        ## 必读文档（按优先级）
        1. [编码](docs/coding-standards.md)
        2. [分层](docs/layered-design.md#section)

        ## 业务文档索引
        | [业务](docs/domain/user-stories-x.md) | 用途 |
        """
    )

    assert docs == {"docs/coding-standards.md", "docs/layered-design.md"}


def test_required_agents_docs_missing_from_mkdocs_nav_fails():
    """AGENTS 必读文档不在 mkdocs nav 中必须失败。"""
    from check_mkdocs_nav_consistency import check_consistency

    issues, warnings = check_consistency(
        mkdocs_paths={"docs/coding-standards.md"},
        agents_required_docs={"docs/coding-standards.md", "docs/layered-design.md"},
        all_docs={"docs/coding-standards.md", "docs/layered-design.md"},
    )

    assert any(issue.target == "docs/layered-design.md" for issue in issues)
    assert warnings == []


def test_any_docs_missing_from_nav_fails():
    """任一 docs 文档未进 mkdocs nav 都必须失败，避免文档导航漂移。"""
    from check_mkdocs_nav_consistency import CORE_DOCS, check_consistency

    mkdocs_paths = set(CORE_DOCS)
    issues, warnings = check_consistency(
        mkdocs_paths=mkdocs_paths,
        agents_required_docs=set(),
        all_docs=mkdocs_paths | {"docs/reviews/ad-hoc-review.md"},
    )

    assert any(issue.target == "docs/reviews/ad-hoc-review.md" for issue in issues)
    assert warnings == []
