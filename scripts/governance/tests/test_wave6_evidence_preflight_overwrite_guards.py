"""Wave 6 evidence preflight overwrite guard tests."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import collect_overwrite_guard_errors


def test_wave6_evidence_preflight_requires_overwrite_guards_for_evidence_writers(
    tmp_path,
    monkeypatch,
):
    """所有 evidence 写入器都必须默认防覆盖，并要求显式 --force。"""
    guarded = "scripts/governance/record_wave_x_evidence.py"
    unguarded = "scripts/governance/record_wave_y_evidence.py"
    validator = "scripts/governance/validate_wave_y_evidence.py"
    errors = collect_overwrite_guard_errors(
        tmp_path,
        monkeypatch,
        {
            guarded: "parser.add_argument('--force')\n'already exists; pass --force to overwrite'\n",
            unguarded: "path.write_text('{}')\n",
            validator: "validate only\n",
        },
    )

    assert len(errors) == 1
    assert unguarded in errors[0]
    assert "--force" in errors[0]
    assert "already exists" in errors[0]
