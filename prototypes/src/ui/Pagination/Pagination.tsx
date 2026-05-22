import { colors, fontStack } from "../../tokens";
import { Button } from "../Button/Button";

export interface PaginationProps {
  total: number;
  pageSize: number;
  current: number;
  onChange: (page: number) => void;
}

/**
 * UI 复合 · Pagination（最简版）
 * Wave 1 替换为 shadcn/ui Pagination
 */
export function Pagination({ total, pageSize, current, onChange }: PaginationProps) {
  const totalPages = Math.max(1, Math.ceil(total / pageSize));
  return (
    <div
      style={{
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        padding: "12px 24px",
        fontSize: 12,
        color: colors.neutral[500],
        fontFamily: fontStack.sans,
      }}
    >
      <span>
        每页 {pageSize} 条 · 共 {total} 条 · 第 {current}/{totalPages} 页
      </span>
      <span style={{ display: "flex", gap: 8 }}>
        <Button
          size="sm"
          variant="secondary"
          disabled={current <= 1}
          onClick={() => onChange(current - 1)}
        >
          上一页
        </Button>
        <Button
          size="sm"
          variant="secondary"
          disabled={current >= totalPages}
          onClick={() => onChange(current + 1)}
        >
          下一页
        </Button>
      </span>
    </div>
  );
}
