import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

/**
 * FieldTable — 字段核对表（标签 + 值）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M2-003 PDA 验收（14 项核对）/ M4-004 出库复核 / M3-005 库存盘点
 * Wave：Wave 1.5 起步（M2 业务页）
 * 业务约束：autoFilled=true 字段必须视觉醒目（扫码自动填充反馈，业务方走查重点）
 *
 * @example
 *   <FieldTable size="lg" rows={[{ label: "批号", value: "20260301A", autoFilled: true, required: true }]} />
 */

const tableVariants = cva(
  "rounded-md border bg-background overflow-hidden font-sans",
  {
    variants: {
      size: {
        sm: "[--ft-px:12px] [--ft-py:8px] [--ft-fs:14px] [--ft-elw:35%]",
        default: "[--ft-px:14px] [--ft-py:10px] [--ft-fs:15px] [--ft-elw:40%]",
        lg: "[--ft-px:16px] [--ft-py:12px] [--ft-fs:18px] [--ft-elw:46%]",
      },
    },
    defaultVariants: { size: "default" },
  }
);

export interface FieldRow {
  label: string;
  value: React.ReactNode;
  autoFilled?: boolean;
  required?: boolean;
  error?: string;
}

export interface FieldTableProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children">,
    VariantProps<typeof tableVariants> {
  rows: FieldRow[];
  /** 标签列宽度；不传时按 size 自适应 */
  labelWidth?: string;
}

export const FieldTable = React.forwardRef<HTMLDivElement, FieldTableProps>(
  ({ rows, size, labelWidth, className, ...rest }, ref) => {
    return (
      <div ref={ref} className={cn(tableVariants({ size }), className)} {...rest}>
        {rows.map((row, i) => (
          <div
            key={i}
            className={cn(
              "grid border-b last:border-b-0 transition-colors",
              row.autoFilled
                ? "bg-primary/5 border-l-[4px] border-l-primary"
                : "border-l-[4px] border-l-transparent"
            )}
            style={{
              gridTemplateColumns: `${labelWidth ?? "var(--ft-elw)"} 1fr`,
              fontSize: "var(--ft-fs)",
            }}
          >
            <div
              className="bg-muted/50 border-r flex items-center font-medium text-foreground/80"
              style={{ padding: "var(--ft-py) var(--ft-px)" }}
            >
              <span>
                {row.label}
                {row.required && (
                  <span aria-label="必填" className="text-destructive ml-1">*</span>
                )}
              </span>
            </div>
            <div
              className="flex flex-col justify-center break-all"
              style={{ padding: "var(--ft-py) var(--ft-px)" }}
            >
              <div>{row.value}</div>
              {row.error && (
                <div role="alert" className="text-destructive mt-1" style={{ fontSize: "calc(var(--ft-fs) - 2px)" }}>
                  {row.error}
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    );
  }
);
FieldTable.displayName = "FieldTable";
