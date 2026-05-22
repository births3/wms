import type { CSSProperties } from "react";
import { colors, radius, fontStack } from "../../tokens";

export interface DiffPanelProps {
  /** 旧值字段 → 文本 */
  before?: Record<string, string>;
  /** 新值字段 → 文本 */
  after?: Record<string, string>;
  /** 高亮变化字段 */
  highlightChanged?: boolean;
  /** 单列布局（紧凑空间） */
  layout?: "side-by-side" | "stacked";
}

/**
 * 业务组件 · DiffPanel
 * 用途：审计追踪 / 批号调整 / 校验异常 / 配置变更 等场景的"旧值-新值对比"
 * 出现频率：H2-002, M-BA-001, M-VR-003, M1-008, ...
 */
export function DiffPanel({
  before,
  after,
  highlightChanged = true,
  layout = "side-by-side",
}: DiffPanelProps) {
  if (!before && !after) {
    return (
      <div
        style={{
          fontSize: 12,
          color: colors.neutral[500],
          padding: 12,
          background: colors.neutral[50],
          borderRadius: radius.md,
          fontFamily: fontStack.sans,
        }}
      >
        无字段级变更（纯流转事件）
      </div>
    );
  }

  // 找出所有 key（合并 before / after），保持顺序
  const allKeys = Array.from(
    new Set([...(before ? Object.keys(before) : []), ...(after ? Object.keys(after) : [])])
  );

  const changed = (k: string) => before?.[k] !== after?.[k];

  if (layout === "stacked") {
    return (
      <div style={{ fontFamily: fontStack.sans, fontSize: 13 }}>
        {allKeys.map((k) => {
          const isChanged = changed(k);
          return (
            <div
              key={k}
              style={{
                padding: "6px 12px",
                background: highlightChanged && isChanged ? "#FFF7ED" : colors.neutral[50],
                borderLeft: highlightChanged && isChanged ? `3px solid ${colors.warning}` : `3px solid transparent`,
                borderRadius: radius.sm,
                marginBottom: 4,
              }}
            >
              <div style={{ color: colors.neutral[500], fontSize: 11 }}>{k}</div>
              <div style={{ display: "flex", alignItems: "center", gap: 8, fontFamily: "monospace", fontSize: 12 }}>
                <span style={{ color: colors.danger, textDecoration: "line-through" }}>{before?.[k] ?? "—"}</span>
                <span aria-hidden style={{ color: colors.neutral[400] }}>→</span>
                <span style={{ color: colors.success }}>{after?.[k] ?? "—"}</span>
              </div>
            </div>
          );
        })}
      </div>
    );
  }

  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, fontFamily: fontStack.sans }}>
      <Pane title="旧值" data={before} keys={allKeys} bg="#FEF2F2" border={colors.danger} highlightChanged={highlightChanged} mode="before" otherData={after} />
      <Pane title="新值" data={after} keys={allKeys} bg="#F0FDF4" border={colors.success} highlightChanged={highlightChanged} mode="after" otherData={before} />
    </div>
  );
}

function Pane({
  title,
  data,
  keys,
  bg,
  border,
  highlightChanged,
  mode,
  otherData,
}: {
  title: string;
  data?: Record<string, string>;
  keys: string[];
  bg: string;
  border: string;
  highlightChanged: boolean;
  mode: "before" | "after";
  otherData?: Record<string, string>;
}) {
  const cellChanged = (k: string) => data?.[k] !== otherData?.[k];

  return (
    <div style={{ background: bg, border: `1px solid ${border}`, borderRadius: radius.md, padding: 12, fontSize: 12 }}>
      <div style={{ color: colors.neutral[700], fontWeight: 500, marginBottom: 6 }}>{title}</div>
      {keys.length > 0 ? (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <tbody>
            {keys.map((k) => {
              const isChanged = highlightChanged && cellChanged(k);
              const cellStyle: CSSProperties = {
                color: isChanged ? (mode === "before" ? colors.danger : colors.success) : colors.neutral[800],
                fontWeight: isChanged ? 600 : 400,
                padding: "2px 0",
                fontFamily: "monospace",
              };
              return (
                <tr key={k}>
                  <td style={{ color: colors.neutral[500], padding: "2px 0", whiteSpace: "nowrap", paddingRight: 8 }}>{k}</td>
                  <td style={cellStyle}>{data?.[k] ?? "—"}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      ) : (
        <div style={{ color: colors.neutral[500], fontStyle: "italic" }}>—</div>
      )}
    </div>
  );
}
