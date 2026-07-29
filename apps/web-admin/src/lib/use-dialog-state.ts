import * as React from "react";

export interface DialogState<T> {
  open: boolean;
  target: T | null;
  /** 设置目标并打开弹窗。 */
  openWith: (target: T) => void;
  /** 关闭弹窗；保留 target 以便关闭动画期间内容不闪烁。 */
  close: () => void;
  /** 兼容 Dialog 的 onOpenChange：false 时等价于 close()。 */
  setOpen: (open: boolean) => void;
  setTarget: React.Dispatch<React.SetStateAction<T | null>>;
}

/**
 * 弹窗「开关 + 目标」合并状态，替代页面里成对出现的
 * `const [xxOpen, setXxOpen] = useState(false)` 与 `const [xxTarget, setXxTarget] = useState(null)`。
 */
export function useDialogState<T>(): DialogState<T> {
  const [open, setOpenState] = React.useState(false);
  const [target, setTarget] = React.useState<T | null>(null);

  const openWith = React.useCallback((next: T) => {
    setTarget(next);
    setOpenState(true);
  }, []);

  const close = React.useCallback(() => {
    setOpenState(false);
  }, []);

  const setOpen = React.useCallback((next: boolean) => {
    setOpenState(next);
  }, []);

  return { open, target, openWith, close, setOpen, setTarget };
}
