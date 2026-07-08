import sys
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))

from check_dialog_overlay_pointer_events import dialog_overlay_is_pointer_safe  # noqa: E402


def test_dialog_overlay_pointer_events_none_is_required():
    source = '''
export const DialogOverlay = React.forwardRef((props, ref) => (
  <DialogPrimitive.Overlay className="pointer-events-none fixed inset-0" />
));
'''
    assert dialog_overlay_is_pointer_safe(source)


def test_dialog_overlay_close_wrapper_is_rejected():
    source = '''
export const DialogOverlay = React.forwardRef((props, ref) => (
  <DialogPrimitive.Close asChild>
    <DialogPrimitive.Overlay className="pointer-events-none fixed inset-0" />
  </DialogPrimitive.Close>
));
'''
    assert not dialog_overlay_is_pointer_safe(source)


def test_dialog_overlay_without_pointer_events_none_is_rejected():
    source = '''
export const DialogOverlay = React.forwardRef((props, ref) => (
  <DialogPrimitive.Overlay className="fixed inset-0" />
));
'''
    assert not dialog_overlay_is_pointer_safe(source)
