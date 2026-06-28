/**
 * M2InboundFilterBar — 入库列表筛选区
 *
 * 层级：Layer 3 页面局部组件
 * 关联故事：US-M2-008
 * Wave：Wave 6
 * 业务约束：筛选和重置在页面内直接操作，不使用弹窗。
 *
 * @example
 *   <M2InboundFilterBar statusFilter="receiving" ... />
 */

import { Button, Card, CardContent, Input } from "@wms/ui";
import { Search } from "lucide-react";

export type StatusFilter = "all" | "receiving" | "inspecting" | "putaway" | "completed" | "closed_rejected";

interface M2InboundFilterBarProps {
  keyword: string;
  statusFilter: StatusFilter;
  arrivalDate: string;
  onKeywordChange: (value: string) => void;
  onStatusFilterChange: (value: StatusFilter) => void;
  onArrivalDateChange: (value: string) => void;
  onReset: () => void;
}

export function M2InboundFilterBar({
  keyword,
  statusFilter,
  arrivalDate,
  onKeywordChange,
  onStatusFilterChange,
  onArrivalDateChange,
  onReset,
}: M2InboundFilterBarProps) {
  return (
    <Card className="rounded-lg shadow-sm">
      <CardContent className="grid gap-3 p-4 md:grid-cols-[minmax(18rem,1fr)_10rem_9rem_auto] md:items-end">
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">关键字</label>
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" aria-hidden />
            <Input
              className="pl-9"
              value={keyword}
              onChange={(event) => onKeywordChange(event.target.value)}
              placeholder="ASN / 商品 / 批号"
            />
          </div>
        </div>
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">状态</label>
          <select
            className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm"
            value={statusFilter}
            onChange={(event) => onStatusFilterChange(event.target.value as StatusFilter)}
          >
            <option value="all">全部</option>
            <option value="receiving">待收货/收货中</option>
            <option value="inspecting">验收中</option>
            <option value="putaway">上架中</option>
            <option value="completed">已完成</option>
            <option value="closed_rejected">已关闭(拒收)</option>
          </select>
        </div>
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">预计到货</label>
          <Input type="date" value={arrivalDate} onChange={(event) => onArrivalDateChange(event.target.value)} />
        </div>
        <Button type="button" variant="outline" onClick={onReset}>
          重置
        </Button>
      </CardContent>
    </Card>
  );
}
