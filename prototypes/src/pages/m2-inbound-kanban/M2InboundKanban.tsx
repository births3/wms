import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { KanbanBoard, PageHeader, type KanbanColumn } from "@/components/business";
import { RefreshCw, Maximize2 } from "lucide-react";

const COLUMNS: KanbanColumn[] = [
  {
    title: "已下单 (待到货)",
    variant: "default",
    items: [
      { id: "asn1", title: "PO-2026-0042", subtitle: "国药控股 · 葡萄糖注射液 ×3 SKU", priority: "normal", meta: [{ label: "预计到货", value: "今日 14:00" }, { label: "件数", value: "120" }] },
      { id: "asn2", title: "PO-2026-0043", subtitle: "九州通 · 头孢类 ×8 SKU", priority: "normal", meta: [{ label: "预计", value: "明日 09:00" }, { label: "件数", value: "240" }] },
    ],
  },
  {
    title: "已到货 (待收货)",
    variant: "warning",
    items: [
      { id: "asn3", title: "PO-2026-0040", subtitle: "上海医药 · 麻醉类（特管） ×2 SKU", priority: "urgent", status: "pending", meta: [{ label: "到货", value: "今日 10:30" }, { label: "等待", value: "1h 25m" }] },
      { id: "asn4", title: "PO-2026-0041", subtitle: "华润 · 注射液 ×5 SKU", priority: "high", status: "pending", meta: [{ label: "到货", value: "今日 11:15" }, { label: "等待", value: "40m" }] },
    ],
  },
  {
    title: "PDA 验收中",
    variant: "default",
    count: 3,
    items: [
      { id: "asn5", title: "PO-2026-0038", subtitle: "国药控股 · 维生素", priority: "normal", status: "in_progress", meta: [{ label: "操作员", value: "张三" }, { label: "进度", value: "8/12" }] },
      { id: "asn6", title: "PO-2026-0039", subtitle: "九州通 · 抗生素", priority: "normal", status: "in_progress", meta: [{ label: "操作员", value: "李四" }, { label: "进度", value: "3/8" }] },
      { id: "asn7", title: "PO-2026-0037", subtitle: "上海医药 · 中成药", priority: "low", status: "offline_cached", meta: [{ label: "操作员", value: "王五" }, { label: "状态", value: "离线暂存" }] },
    ],
  },
  {
    title: "待双人复核",
    variant: "warning",
    items: [
      { id: "asn8", title: "PO-2026-0036", subtitle: "上海医药 · 麻醉类", priority: "urgent", status: "pending", meta: [{ label: "首签", value: "张三 09:14" }, { label: "等待", value: "12m" }] },
    ],
  },
  {
    title: "上架中 / 已完成",
    variant: "success",
    items: [
      { id: "asn9", title: "PO-2026-0035", subtitle: "九州通 · 心血管类", priority: "normal", status: "in_progress", meta: [{ label: "上架", value: "进行中" }, { label: "保管员", value: "赵六" }] },
      { id: "asn10", title: "PO-2026-0034", subtitle: "华润 · 维生素", priority: "low", status: "completed", meta: [{ label: "完成", value: "今日 08:50" }] },
      { id: "asn11", title: "PO-2026-0033", subtitle: "国药 · 注射液", priority: "low", status: "completed", meta: [{ label: "完成", value: "昨日 17:20" }] },
    ],
  },
];

/**
 * M2InboundKanban — M2-008 收货进度看板
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M2-008（收货全流程实时看板）
 * Wave：Wave 3（M2 业务页）
 * 业务约束：实时刷新 ≤ 3s；urgent 优先级（特管药品）红色边条；离线状态特殊标记
 *
 * @example
 *   <M2InboundKanban />
 */
export function M2InboundKanban() {
  return (
    <div className="w-full max-w-[1400px] min-h-[800px] bg-muted/30 border rounded-xl p-6 font-sans">
      <PageHeader
        title="收货进度看板"
        subtitle={`M2-008 · 实时刷新 · 共 ${COLUMNS.reduce((a, c) => a + c.items.length, 0)} 个 ASN · 最近刷新 2 秒前 · GSP §73 收货全流程`}
        actions={
          <>
            <Button variant="outline" size="sm"><RefreshCw className="size-3.5" />手动刷新</Button>
            <Button variant="outline" size="sm"><Maximize2 className="size-3.5" />全屏大屏</Button>
          </>
        }
      />

      {/* 筛选栏 */}
      <Card className="p-3 mb-4">
        <div className="grid grid-cols-[200px_200px_200px_1fr_auto] gap-3 items-end">
          <div className="space-y-1">
            <Label className="text-xs">货主</Label>
            <Select defaultValue="all">
              <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部</SelectItem>
                <SelectItem value="sino">国药控股</SelectItem>
                <SelectItem value="jiuzhou">九州通</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-1">
            <Label className="text-xs">月台</Label>
            <Select defaultValue="all">
              <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部</SelectItem>
                <SelectItem value="d1">月台 1</SelectItem>
                <SelectItem value="d2">月台 2</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-1">
            <Label className="text-xs">优先级</Label>
            <Select defaultValue="all">
              <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部</SelectItem>
                <SelectItem value="urgent">紧急（特管）</SelectItem>
                <SelectItem value="high">高</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <Input className="h-8 text-xs" placeholder="搜索 ASN 号 / 商品 / 操作员" />
          <Button size="sm">查询</Button>
        </div>
      </Card>

      {/* 看板 */}
      <KanbanBoard columns={COLUMNS} />
    </div>
  );
}
