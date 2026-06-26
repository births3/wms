"""Wave 6 evidence preflight Python writer overwrite guard tests."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import collect_overwrite_guard_errors


def test_wave6_evidence_preflight_rejects_marker_only_python_overwrite_guard(
    tmp_path,
    monkeypatch,
):
    """字符串标记不能替代真正的 exists() and not force 写入保护。"""
    writer = "scripts/governance/record_wave_x_evidence.py"
    errors = collect_overwrite_guard_errors(
        tmp_path,
        monkeypatch,
        {
            writer: "\n".join([
                "parser.add_argument('--force', action='store_true')",
                "message = 'already exists; pass --force to overwrite'",
                "path.write_text('{}')",
            ]),
        },
    )

    assert len(errors) == 1
    assert writer in errors[0]
    assert "exists() and not force" in errors[0]


def test_wave6_evidence_preflight_accepts_semantic_python_overwrite_guard(
    tmp_path,
    monkeypatch,
):
    """Python writer 必须在 write_text 前检查 path.exists() and not force。"""
    writer = "scripts/governance/record_wave_x_evidence.py"
    errors = collect_overwrite_guard_errors(
        tmp_path,
        monkeypatch,
        {
            writer: "\n".join([
                "parser.add_argument('--force', action='store_true')",
                "if path.exists() and not force:",
                "    return False, f'{path} already exists; pass --force to overwrite'",
                "path.write_text('{}')",
            ]),
        },
    )

    assert errors == []


def test_wave6_evidence_preflight_accepts_args_force_python_overwrite_guard(
    tmp_path,
    monkeypatch,
):
    """Python writer 使用 argparse namespace 时也必须被识别为防覆盖。"""
    writer = "scripts/governance/record_wave_x_evidence.py"
    errors = collect_overwrite_guard_errors(
        tmp_path,
        monkeypatch,
        {
            writer: "\n".join([
                "parser.add_argument('--force', action='store_true')",
                "if output_path.exists() and not args.force:",
                "    return False, f'{output_path} already exists; pass --force to overwrite'",
                "output_path.write_text('{}')",
            ]),
        },
    )

    assert errors == []
