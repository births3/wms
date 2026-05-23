import * as React from "react";
import { ArrowRight } from "lucide-react";
import { cn } from "../../lib/utils";

/**
 * DiffPanel — 旧值 vs 新值对比面板
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H2-002（审计追踪）/ M-BA-001（批号调整）/ M-VR-003（校验异常）/ M1-008（配置变更）
 * Wave：Wave 0.5 起步，Wave 4 重点（审计页）
 * 业务约束：变化字段必须高亮（业务方走查重点）
 *
 * @example
 *   <DiffPanel before={{ 状态: "验收中" }} after={{ 状态: "已验收" }} />
 */

export interface DiffPanelProps extends React.HTMLAttributes<HTMLDivElement> {
  before?: Record<string, string>;
  after?: Record<string, string>;
  /** 默认 true，关闭时不高亮变化字段 */
  highlightChanged?: boolean;
  layout?: "side-by-side" | "stacked";
}

export const DiffPanel = React.forwardRef<HTMLDivElement, DiffPanelProps>(
  ({ before, after, highlightChanged = true, layout = "side-by-side", className, ...rest }, ref) => {
    if (!before && !after) {
      return (
        <div
          ref={ref}
          className={cn("rounded-md bg-muted px-3 py-2 text-xs text-muted-foreground", className)}
          {...rest}
        >
          无字段级变更（纯流转事件）
        </div>
      );
    }

    const allKeys = Array.from(
      new Set([...(before ? Object.keys(before) : []), ...(after ? Object.keys(after) : [])])
    );
    const changed = (k: string) => before?.[k] !== after?.[k];

    if (layout === "stacked") {
      return (
        <div ref={ref} className={cn("space-y-1", className)} {...rest}>
          {allKeys.map((k) => {
            const isChanged = changed(k);
            return (
              <div
                key={k}
                className={cn(
                  "rounded-sm px-3 py-1.5 text-sm",
                  highlightChanged && isChanged
                    ? "bg-orange-50 border-l-[3px] border-wms-warning"
                    : "bg-muted border-l-[3px] border-transparent"
                )}
              >
                <div className="text-[11px] text-muted-foreground">{k}</div>
                <div className="flex items-center gap-2 font-mono text-xs">
                  <span className="text-destructive line-through">{before?.[k] ?? "—"}</span>
                  <ArrowRight aria-hidden className="size-3 text-muted-foreground/60" />
                  <span className="text-wms-success">{after?.[k] ?? "—"}</span>
                </div>
              </div>
            );
          })}
        </div>
      );
    }

    return (
      <div ref={ref} className={cn("grid grid-cols-2 gap-2", className)} {...rest}>
        <DiffPane title="旧值" data={before} keys={allKeys} mode="before" otherData={after} highlight={highlightChanged} />
        <DiffPane title="新值" data={after} keys={allKeys} mode="after" otherData={before} highlight={highlightChanged} />
      </div>
    );
  }
);
DiffPanel.displayName = "DiffPanel";

function DiffPane({
  title,
  data,
  keys,
  mode,
  otherData,
  highlight,
}: {
  title: string;
  data?: Record<string, string>;
  keys: string[];
  mode: "before" | "after";
  otherData?: Record<string, string>;
  highlight: boolean;
}) {
  const cellChanged = (k: string) => data?.[k] !== otherData?.[k];
  const isBefore = mode === "before";
  return (
    <div
      className={cn(
        "rounded-md border p-3 text-xs",
        isBefore ? "bg-red-50 border-destructive/40" : "bg-green-50 border-wms-success/40"
      )}
    >
      <div className="font-medium text-foreground/80 mb-1.5">{title}</div>
      {keys.length > 0 ? (
        <table className="w-full">
          <tbody>
            {keys.map((k) => {
              const isChanged = highlight && cellChanged(k);
              return (
                <tr key={k}>
                  <td className="py-0.5 pr-2 whitespace-nowrap text-muted-foreground">{k}</td>
                  <td
                    className={cn(
                      "py-0.5 font-mono",
                      isChanged && (isBefore ? "text-destructive font-semibold" : "text-wms-success font-semibold"),
                      !isChanged && "text-foreground"
                    )}
                  >
                    {data?.[k] ?? "—"}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      ) : (
        <div className="text-muted-foreground italic">—</div>
      )}
    </div>
  );
}
