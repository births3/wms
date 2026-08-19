"""Feature Flag 与 CHANGELOG 新鲜度治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
def test_feature_flags_empty_registry_passes():
    """空 Feature Flag 注册表允许存在，不引入默认业务开关。"""
    from datetime import date
    from check_feature_flags import check_flags
    assert check_flags({"flags": []}, today=date(2026, 6, 2)) == []


def test_feature_flags_expired_cleanup_fails():
    """cleanup_by 早于当前日期 → 必须清理。"""
    from datetime import date
    from check_feature_flags import check_flags
    issues = check_flags({
        "flags": [{
            "key": "h1_auth_login_v1",
            "owner": "platform",
            "created_at": "2026-01-01",
            "cleanup_by": "2026-03-01",
            "enabled": False,
        }],
    }, today=date(2026, 6, 2))
    assert any(i.kind == "expired" for i in issues)


def test_feature_flags_lifetime_over_90_days_fails():
    """Feature Flag 清理期不能超过 created_at + 90 天。"""
    from datetime import date
    from check_feature_flags import check_flags
    issues = check_flags({
        "flags": [{
            "key": "h1_auth_login_v1",
            "owner": "platform",
            "created_at": "2026-06-01",
            "cleanup_by": "2026-09-30",
            "enabled": False,
        }],
    }, today=date(2026, 6, 2))
    assert any(i.kind == "lifetime_too_long" for i in issues)


def test_changelog_freshness_allows_repos_without_tags():
    """仓库尚无 tag 时，CHANGELOG 只要存在即可通过。"""
    from check_changelog_freshness import check_changelog_text

    assert check_changelog_text("# Changelog\n\n## [Unreleased]\n", latest_tag=None) == []


def test_changelog_freshness_requires_latest_tag_entry():
    """有最新 tag 时，CHANGELOG 必须能追到对应版本条目。"""
    from check_changelog_freshness import check_changelog_text

    issues = check_changelog_text("# Changelog\n\n## [Unreleased]\n", latest_tag="v0.1.0")

    assert len(issues) == 1
    assert issues[0].kind == "missing_latest_tag"
    assert "v0.1.0" in issues[0].detail


def test_changelog_freshness_accepts_latest_tag_with_or_without_v_prefix():
    """CHANGELOG 可写 v0.1.0 或 0.1.0，两者都能匹配 latest tag。"""
    from check_changelog_freshness import check_changelog_text

    assert check_changelog_text("# Changelog\n\n## [v0.1.0]\n", latest_tag="v0.1.0") == []
    assert check_changelog_text("# Changelog\n\n## [0.1.0]\n", latest_tag="v0.1.0") == []
