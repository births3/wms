import * as React from "react";

export interface TransientEventState {
  lastEvent: string | null;
  setLastEvent: React.Dispatch<React.SetStateAction<string | null>>;
  clearLastEvent: () => void;
}

/**
 * 页面右上角「XX 已完成」轻提示：设置后自动在 timeoutMs 毫秒后消失。
 */
export function useTransientEvent(timeoutMs = 3000): TransientEventState {
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!lastEvent) return;
    const timer = window.setTimeout(() => setLastEvent(null), timeoutMs);
    return () => window.clearTimeout(timer);
  }, [lastEvent, timeoutMs]);

  const clearLastEvent = React.useCallback(() => setLastEvent(null), []);

  return { lastEvent, setLastEvent, clearLastEvent };
}
