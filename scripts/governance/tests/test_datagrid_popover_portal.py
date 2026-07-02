import sys
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))

from check_datagrid_popover_portal import (  # noqa: E402
    data_grid_dismiss_hook_is_complete,
    popover_tag_uses_absolute,
    source_uses_datagrid_dismiss_hook,
)


def test_popover_absolute_is_rejected():
    tag = '<div className="absolute top-full z-30" data-datagrid-popover>'
    assert popover_tag_uses_absolute(tag)


def test_popover_fixed_is_allowed():
    tag = '<div className="fixed z-50 w-56" data-datagrid-popover>'
    assert not popover_tag_uses_absolute(tag)


def test_data_grid_dismiss_hook_requires_pointer_and_escape():
    source = '''
document.addEventListener("pointerdown", dismissOnOutsidePointer);
target?.closest("[data-datagrid-popover]");
document.addEventListener("keydown", dismissOnEscape);
if (event.key !== "Escape") return;
'''
    assert data_grid_dismiss_hook_is_complete(source)


def test_data_grid_dismiss_hook_rejects_missing_escape():
    source = '''
document.addEventListener("pointerdown", dismissOnOutsidePointer);
target?.closest("[data-datagrid-popover]");
'''
    assert not data_grid_dismiss_hook_is_complete(source)


def test_data_grid_source_must_use_shared_dismiss_hook():
    assert source_uses_datagrid_dismiss_hook('useDataGridPopoverDismiss({ open, onDismiss: close })')
    assert not source_uses_datagrid_dismiss_hook('document.addEventListener("pointerdown", closePanels)')
