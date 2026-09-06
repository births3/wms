import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import { Search, ArrowRight, Package } from "lucide-react";
import { StatusBadge, OfflineIndicator } from "@wms/ui";

/**
 * M2InboundTasks — M2-002 PDA 待收货任务列表
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M2-002（PDA 待收货 ASN 任务列表 / 按优先级 + 时间排序）
 * Wave：Wave 1.5（M2 PDA 业务流）
 * 业务约束：只显示当前作业人有权操作的 ASN；冷链优先；超期红色高亮
 *
 * @example
 *   <M2InboundTasks />
 */

type Priority = "urgent" | "cold" | "normal";

interface Task {
  asn: string;
  supplier: string;
  itemCount: number;
  totalQty: number;
  priority: Priority;
  arrivalAt: string;
  status: "pending" | "in_progress" | "completed";
  isCold: boolean;
  isOverdue?: boolean;
}

const TASKS: Task[] = [
  {
    asn: "PO-2026-0001",
    supplier: "国药控股北京",
    itemCount: 3,
    totalQty: 240,
    priority: "cold",
    arrivalAt: "今日 14:30",
    status: "pending",
    isCold: true,
  },
  {
    asn: "PO-2026-0002",
    supplier: "上海医药华东",
    itemCount: 8,
    totalQty: 1820,
    priority: "urgent",
    arrivalAt: "今日 09:15（已到）",
    status: "in_progress",
    isCold: false,
    isOverdue: true,
  },
  {
    asn: "PO-2026-0003",
    supplier: "九州通医药",
    itemCount: 12,
    totalQty: 3640,
    priority: "normal",
    arrivalAt: "今日 16:00",
    status: "pending",
    isCold: false,
  },
  {
    asn: "PO-2026-0004",
    supplier: "华润医药",
    itemCount: 5,
    totalQty: 480,
    priority: "cold",
    arrivalAt: "今日 17:45",
    status: "pending",
    isCold: true,
  },
];

const PRIORITY_LABEL: Record<Priority, string> = {
  urgent: "🔥 加急",
  cold: "❄️ 冷链",
  normal: "普通",
};

export function M2InboundTasks() {
  return (
    <div data-device="pda" className="w-[480px] min-h-[800px] flex flex-col rounded-xl border overflow-hidden shadow-md font-sans bg-muted/30">
      <OfflineIndicator state="online" />

      {/* 顶栏 */}
      <div className="bg-background px-4 py-3 border-b">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-xs text-muted-foreground">M2-002 待收货任务</div>
            <div className="text-base font-semibold mt-0.5">收货员 张三 · u001</div>
          </div>
          <div className="text-right">
            <div className="text-2xl font-bold text-primary">{TASKS.length}</div>
            <div className="text-xs text-muted-foreground">待处理</div>
          </div>
        </div>
      </div>

      {/* 搜索栏 */}
      <div className="bg-background px-4 py-2 border-b">
        <div className="relative">
          <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
          <input
            placeholder="搜索 ASN / 供应商"
            className="w-full pl-9 pr-3 py-2 text-sm rounded-md border bg-background"
          />
        </div>
      </div>

      {/* 任务列表 */}
      <div className="flex-1 overflow-y-auto p-3 flex flex-col gap-2">
        {TASKS.map((task) => (
          <Card
            key={task.asn}
            className={`p-3 active:bg-muted ${
              task.isOverdue ? "border-destructive border-2" : ""
            }`}
          >
            <div className="flex items-start justify-between mb-2">
              <div className="flex-1">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-mono text-sm font-semibold">{task.asn}</span>
                  {task.priority !== "normal" && (
                    <span className={`text-[11px] px-1.5 py-0.5 rounded ${
                      task.priority === "urgent"
                        ? "bg-destructive/10 text-destructive font-medium"
                        : "bg-wms-cold/10 text-wms-cold font-medium"
                    }`}>
                      {PRIORITY_LABEL[task.priority]}
                    </span>
                  )}
                </div>
                <div className="text-xs text-muted-foreground">{task.supplier}</div>
              </div>
              {task.status === "in_progress" ? (
                <StatusBadge status="in_progress" size="sm" label="收货中" />
              ) : (
                <StatusBadge status="pending" size="sm" />
              )}
            </div>

            <div className="flex items-center gap-3 text-xs text-muted-foreground mt-2">
              <div className="flex items-center gap-1">
                <Package className="size-3" />
                <span>{task.itemCount} 项 · {task.totalQty} 件</span>
              </div>
              <div className={task.isOverdue ? "text-destructive font-medium" : ""}>
                {task.arrivalAt}
              </div>
            </div>

            <Button
              variant={task.status === "in_progress" ? "default" : "outline"}
              size="sm"
              className="w-full mt-3 h-9"
            >
              {task.status === "in_progress" ? "继续收货" : "开始收货"}
              <ArrowRight className="size-4 ml-1" />
            </Button>
          </Card>
        ))}
      </div>

      {/* 底部统计 */}
      <div className="bg-background px-4 py-2 border-t text-xs text-muted-foreground flex items-center justify-between">
        <span>今日已完成 5 单</span>
        <span>同步 30s 前</span>
      </div>
    </div>
  );
}
