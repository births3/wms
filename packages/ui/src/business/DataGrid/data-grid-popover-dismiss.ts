import * as React from "react";

interface DataGridPopoverDismissOptions {
  open: boolean;
  onDismiss: () => void;
}

export function useDataGridPopoverDismiss({ open, onDismiss }: DataGridPopoverDismissOptions) {
  const onDismissRef = React.useRef(onDismiss);

  React.useEffect(() => {
    onDismissRef.current = onDismiss;
  }, [onDismiss]);

  React.useEffect(() => {
    if (!open) return;

    function dismissOnOutsidePointer(event: PointerEvent) {
      const target = event.target instanceof Element ? event.target : null;
      if (target?.closest("[data-datagrid-popover]")) return;
      onDismissRef.current();
    }

    function dismissOnEscape(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      onDismissRef.current();
    }

    document.addEventListener("pointerdown", dismissOnOutsidePointer);
    document.addEventListener("keydown", dismissOnEscape);
    return () => {
      document.removeEventListener("pointerdown", dismissOnOutsidePointer);
      document.removeEventListener("keydown", dismissOnEscape);
    };
  }, [open]);
}
