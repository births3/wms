import sys
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))

from check_datagrid_popover_portal import popover_tag_uses_absolute  # noqa: E402


def test_popover_absolute_is_rejected():
    tag = '<div className="absolute top-full z-30" data-datagrid-popover>'
    assert popover_tag_uses_absolute(tag)


def test_popover_fixed_is_allowed():
    tag = '<div className="fixed z-50 w-56" data-datagrid-popover>'
    assert not popover_tag_uses_absolute(tag)
