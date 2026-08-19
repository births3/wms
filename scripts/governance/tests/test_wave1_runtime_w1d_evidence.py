"""Wave 1 W1.D runtime evidence 出口聚合治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


W1D_RUNTIME_COMMANDS = [
    "wave-1-runtime-evidence-validate",
    "wave-1-runtime-prereq-rollback-k8s",
    "wave-1-runtime-prereq-rollback-compose",
    "wave-1-rollback-runtime-readiness-k8s",
    "wave-1-rollback-runtime-readiness-compose",
    "wave-1-rollback-runtime-evidence-k8s",
    "wave-1-rollback-runtime-evidence-compose",
]


def _write_w1d_runtime_collection_assets(tmp_path: Path) -> None:
    probe = tmp_path / "deploy" / "scripts" / "wave1_auto_rollback_probe.sh"
    probe.parent.mkdir(parents=True)
    probe.write_text("#!/usr/bin/env bash\n# --check-only\n", encoding="utf-8")

    prereq = tmp_path / "scripts" / "governance" / "check_wave1_runtime_evidence_prereqs.py"
    prereq.parent.mkdir(parents=True)
    prereq.write_text("# prereq\n", encoding="utf-8")
    validator = tmp_path / "scripts" / "governance" / "validate_wave1_runtime_evidence.py"
    validator.write_text("# validator\n", encoding="utf-8")

    (tmp_path / "justfile").write_text(
        "\n".join(f"{command}:\n    @true" for command in W1D_RUNTIME_COMMANDS),
        encoding="utf-8",
    )
    runbook = tmp_path / "docs" / "runbooks" / "wave-1-runtime-evidence.md"
    runbook.parent.mkdir(parents=True)
    runbook.write_text(
        "\n".join(f"just {command}" for command in W1D_RUNTIME_COMMANDS),
        encoding="utf-8",
    )


def test_wave1_completion_report_w1d_collection_assets_are_checked(tmp_path, monkeypatch):
    """W1.D runtime probe、just 入口和 runbook 都要在出口报告中可见。"""
    import report_wave1_completion as report

    _write_w1d_runtime_collection_assets(tmp_path)
    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)

    ok, message = report.valid_w1d_runtime_collection_assets()

    assert ok is True
    assert "W1.D runtime" in message


def test_wave1_completion_report_w1d_backend_evidence_is_not_keyword_only(monkeypatch):
    """W1.D 后端证据不能只因注释里出现 feature flag 字样而通过。"""
    import report_wave1_completion as report

    def fake_any_file_contains(root, pattern):
        if root == "backend" and "FeatureFlagRegistry" in pattern:
            return False
        if root == "deploy":
            return True
        return False

    monkeypatch.setattr(report, "any_file_contains", fake_any_file_contains)
    monkeypatch.setattr(report, "file_exists", lambda path: path == "docs/retros/wave-1-retro.md")
    monkeypatch.setattr(report, "file_contains", lambda path, needle: needle == "dev/staging")
    monkeypatch.setattr(report, "accepted_adr", lambda path: True)

    items = {item.item_id: item for item in report.evaluate_wave1()}

    assert items["W1.D-runtime"].status == report.MISSING_OR_NEEDS_CONFIRMATION
    assert any("FeatureFlagRegistry" in gap for gap in items["W1.D-runtime"].gaps)


def test_wave1_completion_report_w1d_signal_entry_without_runtime_record_is_pre_release_gap(monkeypatch):
    """W1.D 真实信号入口可完成开发门禁，真实运行记录仍进入预发布 gate。"""
    import report_wave1_completion as report

    def fake_any_file_contains(root, pattern):
        return root == "backend" and "FeatureFlagRegistry" in pattern

    def fake_file_exists(path):
        return path in {
            "deploy/scripts/wave1_rollback.sh",
            "deploy/scripts/wave1_auto_rollback_probe.sh",
            "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
            "scripts/governance/validate_wave1_runtime_evidence.py",
            "docs/retros/wave-1-retro.md",
        }

    def fake_file_contains(path, needle):
        if path == "justfile":
            return needle in set(W1D_RUNTIME_COMMANDS)
        if path == "docs/runbooks/wave-1-runtime-evidence.md":
            return needle in {f"just {command}" for command in W1D_RUNTIME_COMMANDS}
        if path == "deploy/scripts/wave1_rollback.sh":
            return needle in {
                "kubectl rollout undo",
                "docker compose",
                "--execute",
                "validate_environment_boundary",
                'validate_environment_boundary "--context" "$context"',
                'validate_environment_boundary "--namespace" "$namespace"',
                'validate_environment_boundary "--compose-file" "$compose_file_abs"',
                "must include the selected environment token",
                "must not point to a production boundary",
            }
        if path == "deploy/scripts/wave1_auto_rollback_probe.sh":
            return needle in {
                "missing runtime evidence",
                "--smoke-url",
                "PROMETHEUS_URL",
                "wave1_rollback.sh",
                "--execute",
                "--check-only",
            }
        if path == "docs/retros/wave-1-retro.md":
            return needle == "dev/staging"
        return False

    monkeypatch.setattr(report, "any_file_contains", fake_any_file_contains)
    monkeypatch.setattr(report, "file_exists", fake_file_exists)
    monkeypatch.setattr(report, "file_contains", fake_file_contains)
    monkeypatch.setattr(report, "accepted_adr", lambda path: True)
    monkeypatch.setattr(
        report,
        "valid_w1d_runtime_evidence",
        lambda: (False, "缺少真实 dev/staging 自动回滚证据"),
    )

    items = {item.item_id: item for item in report.evaluate_wave1()}

    assert items["W1.D-runtime"].status == report.PROVED_BY_STATIC_FILES
    assert not items["W1.D-runtime"].blocks_strict
    assert items["W1.D-pre-release-runtime"].status == report.MISSING_OR_NEEDS_CONFIRMATION
    assert not items["W1.D-pre-release-runtime"].blocks_strict
    assert any("真实 dev/staging" in gap for gap in items["W1.D-pre-release-runtime"].gaps)


def test_wave1_completion_report_w1d_deploy_keyword_is_not_enough(monkeypatch):
    """deploy 文件里只有 rollback 字样不能证明 W1.D 回滚链路。"""
    import report_wave1_completion as report

    def fake_any_file_contains(root, pattern):
        return root == "backend" and "FeatureFlagRegistry" in pattern

    def fake_file_exists(path):
        return path in {
            "deploy/scripts/wave1_rollback.sh",
            "docs/retros/wave-1-retro.md",
        }

    def fake_file_contains(path, needle):
        if path == "deploy/scripts/wave1_rollback.sh":
            return needle == "rollback"
        if path == "docs/retros/wave-1-retro.md":
            return needle == "dev/staging"
        return False

    monkeypatch.setattr(report, "any_file_contains", fake_any_file_contains)
    monkeypatch.setattr(report, "file_exists", fake_file_exists)
    monkeypatch.setattr(report, "file_contains", fake_file_contains)
    monkeypatch.setattr(report, "accepted_adr", lambda path: True)

    items = {item.item_id: item for item in report.evaluate_wave1()}

    assert items["W1.D-runtime"].status == report.MISSING_OR_NEEDS_CONFIRMATION
    assert any("回滚执行资产" in gap for gap in items["W1.D-runtime"].gaps)
