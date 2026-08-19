"""Wave 6 evidence preflight shell writer overwrite guard tests."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import collect_overwrite_guard_errors


def test_wave6_evidence_preflight_rejects_marker_only_shell_overwrite_guard(
    tmp_path,
    monkeypatch,
):
    """shell writer 不能只含提示语，必须在写入前检查 evidence_file 和 force。"""
    writer = "deploy/scripts/wave1_auto_rollback_probe.sh"
    errors = collect_overwrite_guard_errors(
        tmp_path,
        monkeypatch,
        {
            writer: "\n".join([
                "force=false",
                "echo 'already exists; pass --force to overwrite'",
                "write_evidence_file() { EVIDENCE_FILE=\"$evidence_file\" python3 - <<'PY'; }",
                "# --force",
            ]),
        },
    )

    assert len(errors) == 1
    assert writer in errors[0]
    assert '-e "$evidence_file"' in errors[0]


def test_wave6_evidence_preflight_accepts_semantic_shell_overwrite_guard(
    tmp_path,
    monkeypatch,
):
    """shell writer 必须在写入前检查 evidence 文件存在且 force 未开启。"""
    writer = "deploy/scripts/wave1_auto_rollback_probe.sh"
    errors = collect_overwrite_guard_errors(
        tmp_path,
        monkeypatch,
        {
            writer: "\n".join([
                "force=false",
                "if [ -e \"$evidence_file\" ] && [ \"$force\" != \"true\" ]; then",
                "  echo \"${evidence_file} already exists; pass --force to overwrite\" >&2",
                "fi",
                "write_evidence_file() { EVIDENCE_FILE=\"$evidence_file\" python3 - <<'PY'; }",
                "# --force",
            ]),
        },
    )

    assert errors == []
