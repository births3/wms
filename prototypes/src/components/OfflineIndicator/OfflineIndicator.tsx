import type { CSSProperties } from "react";
import { colors, fontStack } from "../../tokens";

export type OfflineState = "online" | "offline" | "syncing";

export interface OfflineIndicatorProps {
  state: OfflineState;
  /** 离线暂存的待同步条数 */
  pendingCount?: number;
  /** 同步进度 0-100（state=syncing 时使用） */
  syncProgress?: number;
  className?: string;
  testId?: string;
}

const STATE_META = {
  online: { bg: colors.success, text: "在线" },
  offline: { bg: colors.warning, text: "离线模式" },
  syncing: { bg: colors.primary, text: "同步中" },
} as const;

export function OfflineIndicator({
  state,
  pendingCount,
  syncProgress,
  className,
  testId,
}: OfflineIndicatorProps) {
  const meta = STATE_META[state];

  // online 状态默认不展示（避免占用 PDA 屏幕空间）
  if (state === "online" && !pendingCount) return null;
  // syncing 100% 完成且无暂存 = 同步刚完成，自动隐藏
  if (state === "syncing" && syncProgress === 100 && !pendingCount) return null;

  const containerStyle: CSSProperties = {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    background: meta.bg,
    color: "#fff",
    padding: "8px 16px",
    fontSize: 16, // PDA 端 ≥ 16pt
    fontWeight: 500,
    width: "100%",
    boxSizing: "border-box",
    fontFamily: fontStack.sans,
  };

  return (
    <div
      data-testid={testId}
      data-state={state}
      role="status"
      aria-live="polite"
      className={className}
      style={containerStyle}
    >
      <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span aria-hidden>{state === "syncing" ? "⟳" : state === "offline" ? "⚠" : "●"}</span>
        <span>{meta.text}</span>
        {pendingCount !== undefined && pendingCount > 0 && (
          <span
            style={{
              background: "rgba(255,255,255,0.25)",
              borderRadius: 999,
              padding: "2px 8px",
              fontSize: 14,
            }}
          >
            待同步 {pendingCount}
          </span>
        )}
      </span>
      {state === "syncing" && syncProgress !== undefined && (
        <span style={{ fontSize: 14 }}>{Math.round(syncProgress)}%</span>
      )}
    </div>
  );
}
