import * as React from "react";
import { CalendarRange, ChevronDown, ListChecks, Warehouse } from "lucide-react";

import { EmptyState, StatusBadge, cn, type StatusKey } from "@wms/ui";
import type { Dock, DockAppointment } from "@/features/dock/dock-queries";

/**
 * DockOccupancyBoard — 月台预约占用时间线与 24 小时队列
 *
 * 层级：Layer 3 页面私有组件
 * 关联故事：US-DOCK-006（月台占用看板）
 * Wave：延期故事前端基础看板切片
 * 业务约束：展示页面传入的仓库预约查询结果；实时刷新、告警、导出和打印留待后续切片
 *
 * @example
 *   <DockOccupancyBoard warehouseSelected docks={[]} appointments={[]} />
 */

const DAY_MS = 24 * 60 * 60 * 1000;
const TIME_MARKS = [
  ["现在", "left-0"], ["+6 小时", "left-1/4"], ["+12 小时", "left-1/2"],
  ["+18 小时", "left-3/4"], ["+24 小时", "right-0"],
] as const;

export type DockBoardStatus = "idle" | "scheduled" | "occupied" | "arrived" | "cancelled" | "timeout" | "completed" | "invalid";
export type DockOccupancyBoardProps = React.HTMLAttributes<HTMLDetailsElement> & {
  warehouseSelected: boolean;
  warehouseLabel?: string;
  docks: Dock[];
  appointments: DockAppointment[];
  loading?: boolean;
  error?: string;
  /** 供纯函数测试和未来刷新边界使用；未传时取渲染时刻。 */
  now?: Date;
};

type TimelineItem = { appointment: DockAppointment; status: DockBoardStatus; startMs: number; endMs: number; left: number; width: number; lane: number };
type OccupancyRow = { dock: Dock; status: DockBoardStatus; timeline: TimelineItem[]; laneCount: number };
type QueueItem = { appointment: DockAppointment; status: DockBoardStatus; dockLabel: string; startMs: number; endMs: number };
export type DockOccupancyBoardModel = {
  totalAppointments: number;
  currentOccupiedCount: number;
  futureQueueCount: number;
  invalidAppointmentCount: number;
  rows: OccupancyRow[];
  queue: QueueItem[];
};

const STATUS_META: Record<DockBoardStatus, { label: string; badge: StatusKey; bar: string }> = {
  idle: { label: "空闲", badge: "completed", bar: "border-wms-success/60 bg-wms-success/70" },
  scheduled: { label: "预约中", badge: "pending", bar: "border-primary/60 bg-primary/70" },
  occupied: { label: "预约中", badge: "pending", bar: "border-wms-warning bg-wms-warning/80" },
  arrived: { label: "已到达", badge: "completed", bar: "border-wms-success bg-wms-success/80" },
  cancelled: { label: "已取消", badge: "isolated", bar: "border-muted-foreground/50 bg-muted-foreground/40" },
  timeout: { label: "超时", badge: "expired", bar: "border-destructive bg-destructive/80" },
  completed: { label: "已完成", badge: "completed", bar: "border-wms-success/60 bg-wms-success/60" },
  invalid: { label: "日期异常", badge: "unqualified", bar: "border-destructive bg-destructive/15" },
};

export function buildDockOccupancyModel(docks: Dock[], appointments: DockAppointment[], now: Date): DockOccupancyBoardModel {
  const safeDocks: Dock[] = Array.isArray(docks) ? docks : [];
  const safeAppointments: DockAppointment[] = Array.isArray(appointments) ? appointments : [];
  const nowMs = safeNow(now);
  const endOfWindow = nowMs + DAY_MS;
  const byDock = new Map<string, DockAppointment[]>();
  for (const appointment of safeAppointments) byDock.set(appointment.dock_id, [...(byDock.get(appointment.dock_id) ?? []), appointment]);

  const rows = safeDocks.map((dock) => {
    const dockAppointments = byDock.get(dock.id) ?? [];
    const timeline = addLanes(dockAppointments.flatMap((appointment) => {
      const startMs = parseDate(appointment.window_start_at);
      const endMs = parseDate(appointment.window_end_at);
      if (startMs === null || endMs === null || endMs <= startMs || endMs <= nowMs || startMs >= endOfWindow) return [];
      const left = ((Math.max(startMs, nowMs) - nowMs) / DAY_MS) * 100;
      const right = ((Math.min(endMs, endOfWindow) - nowMs) / DAY_MS) * 100;
      return [{ appointment, status: getDockBoardStatus(appointment, now), startMs, endMs, left, width: Math.max(0.5, Math.min(100 - left, right - left)), lane: 0 }];
    }));
    return { dock, status: currentDockStatus(dockAppointments, now), timeline, laneCount: Math.max(1, timeline.reduce((max, item) => Math.max(max, item.lane + 1), 0)) };
  });

  const labels = new Map(safeDocks.map((dock) => [dock.id, dockLabel(dock)]));
  const queue = safeAppointments.flatMap((appointment) => {
    const startMs = parseDate(appointment.window_start_at);
    const endMs = parseDate(appointment.window_end_at);
    const status = getDockBoardStatus(appointment, now);
    if (startMs === null || endMs === null || endMs <= startMs || startMs < nowMs || startMs >= endOfWindow || status !== "scheduled") return [];
    return [{ appointment, status, dockLabel: labels.get(appointment.dock_id) ?? "未匹配月台", startMs, endMs }];
  }).sort((left, right) => left.startMs - right.startMs);

  return {
    totalAppointments: safeAppointments.length,
    currentOccupiedCount: rows.filter((row) => row.status === "occupied" || row.status === "arrived").length,
    futureQueueCount: queue.length,
    invalidAppointmentCount: safeAppointments.filter((appointment) => getDockBoardStatus(appointment, now) === "invalid").length,
    rows,
    queue,
  };
}

export function getDockBoardStatus(appointment: DockAppointment, now: Date): DockBoardStatus {
  const startMs = parseDate(appointment.window_start_at);
  const endMs = parseDate(appointment.window_end_at);
  if (startMs === null || endMs === null || endMs <= startMs) return "invalid";
  const status = String(appointment.status ?? "").toLowerCase();
  if (status === "cancelled") return "cancelled";
  if (["completed", "done", "finished"].includes(status)) return "completed";
  if (endMs <= safeNow(now)) return "timeout";
  if (status === "arrived" || Boolean(appointment.arrived_at)) return "arrived";
  return startMs <= safeNow(now) ? "occupied" : "scheduled";
}

export const DockOccupancyBoard = React.forwardRef<HTMLDetailsElement, DockOccupancyBoardProps>(
  ({ warehouseSelected, warehouseLabel, docks, appointments, loading = false, error, now = new Date(), className, ...rest }, ref) => {
    const model = buildDockOccupancyModel(docks, appointments, now);
    const selectedWarehouse = warehouseLabel?.trim() || "当前仓库";
    const legend = ["idle", "scheduled", "arrived", "cancelled", "timeout"] as DockBoardStatus[];
    return (
      <details ref={ref} className={cn("rounded-md border bg-card", className)} {...rest}>
        <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 [&::-webkit-details-marker]:hidden">
          <div className="min-w-0"><h2 className="truncate text-base font-semibold">月台占用看板</h2><p className="mt-1 truncate text-xs text-muted-foreground">{warehouseSelected ? `当前选定仓库：${selectedWarehouse}` : "当前未选择仓库"} · 未来 24 小时接口预约</p></div>
          <ChevronDown className="size-4 shrink-0 text-muted-foreground" aria-hidden />
        </summary>
        <div className="space-y-4 border-t p-4">
          {!warehouseSelected ? <EmptyState icon={<Warehouse className="size-10" aria-hidden />} title="请选择仓库" description="选择仓库后查看月台占用时间线和未来 24 小时预约队列。" /> : error ? <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-8 text-center text-sm text-destructive" role="alert">月台占用看板加载失败：{error}</div> : loading ? <div className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground" role="status">正在加载月台占用看板...</div> : (
            <>
              <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground"><span>数据范围：当前仓库未来 24 小时接口预约</span>{model.invalidAppointmentCount > 0 && <span className="text-destructive" role="status">{model.invalidAppointmentCount} 条预约日期异常，未绘制时间条</span>}</div>
              <dl className="grid grid-cols-1 divide-y rounded-md border sm:grid-cols-3 sm:divide-x sm:divide-y-0"><Metric icon={<CalendarRange className="size-4" aria-hidden />} label="预约总数" value={model.totalAppointments} /><Metric icon={<Warehouse className="size-4" aria-hidden />} label="当前占用 / 已到达" value={model.currentOccupiedCount} /><Metric icon={<ListChecks className="size-4" aria-hidden />} label="未来 24 小时排队" value={model.futureQueueCount} /></dl>
              {model.totalAppointments === 0 && <EmptyState title="当前仓库未来 24 小时暂无预约" description="创建月台预约后将显示时间线和未来 24 小时队列。" />}
              {model.rows.length === 0 ? <EmptyState icon={<Warehouse className="size-10" aria-hidden />} title="当前仓库暂无月台档案" description="先维护月台档案，再查看预约占用状态。" /> : <>
                <section aria-labelledby="dock-occupancy-timeline-title" className="space-y-3"><div className="flex flex-wrap items-center justify-between gap-2"><h3 id="dock-occupancy-timeline-title" className="text-sm font-semibold">月台时间线</h3><div className="flex flex-wrap gap-2" aria-label="看板状态图例">{legend.map((status) => <StatusBadge key={status} status={STATUS_META[status].badge} label={STATUS_META[status].label} size="sm" />)}</div></div><div className="overflow-x-auto rounded-md border"><div className="min-w-[760px] p-3"><div className="flex gap-3 pb-2 text-xs text-muted-foreground"><span className="w-28 shrink-0">月台 / 当前状态</span><div className="relative h-5 flex-1">{TIME_MARKS.map(([label, position]) => <span key={label} className={cn("absolute -translate-x-1/2 whitespace-nowrap", position, position === "right-0" && "translate-x-0")}>{label}</span>)}</div></div><div className="space-y-2">{model.rows.map((row) => <TimelineRow key={row.dock.id} row={row} />)}</div></div></div></section>
                <section aria-labelledby="dock-occupancy-queue-title" className="space-y-3"><div className="flex items-center justify-between gap-2"><h3 id="dock-occupancy-queue-title" className="text-sm font-semibold">未来 24 小时预约队列</h3><span className="text-xs text-muted-foreground">{model.futureQueueCount} 条</span></div>{model.queue.length === 0 ? <div className="rounded-md border border-dashed px-4 py-6 text-center text-sm text-muted-foreground" role="status">未来 24 小时暂无排队预约</div> : <ol className="divide-y rounded-md border">{model.queue.map((item) => <QueueRow key={`${item.appointment.id}-${item.appointment.version}`} item={item} />)}</ol>}</section>
              </>}
            </>
          )}
        </div>
      </details>
    );
  },
);

DockOccupancyBoard.displayName = "DockOccupancyBoard";

function Metric({ icon, label, value }: { icon: React.ReactNode; label: string; value: number }) { return <div className="flex items-center gap-3 px-4 py-3"><div className="text-muted-foreground">{icon}</div><div><dt className="text-xs text-muted-foreground">{label}</dt><dd className="mt-1 text-lg font-semibold tabular-nums">{value}</dd></div></div>; }

function TimelineRow({ row }: { row: OccupancyRow }) {
  const rowHeight = Math.max(56, row.laneCount * 28 + 8);
  return <div className="flex items-center gap-3"><div className="flex w-28 shrink-0 flex-col gap-1 overflow-hidden"><span className="truncate font-mono text-sm" title={dockLabel(row.dock)}>{dockLabel(row.dock)}</span><StatusBadge status={STATUS_META[row.status].badge} label={STATUS_META[row.status].label} size="sm" className="w-fit" /></div>
    {/* 动态：根据重叠预约数量撑开同一月台的时间条轨道。 */}
    <div className="relative flex-1 rounded-md bg-muted/40" style={{ minHeight: `${rowHeight}px` }}>{["left-0", "left-1/4", "left-1/2", "left-3/4", "right-0"].map((position) => <span key={position} className={cn("absolute inset-y-0 border-l border-border/70", position)} aria-hidden />)}{row.timeline.length === 0 && <span className="absolute inset-0 flex items-center px-3 text-xs text-muted-foreground">无未来预约条带</span>}
      {/* 动态：预约时间在 24 小时轨道中的位置、宽度和重叠层级。 */}
      {row.timeline.map((item) => <div key={`${item.appointment.id}-${item.appointment.version}`} className={cn("absolute z-10 h-6 overflow-hidden rounded border px-2 text-xs leading-6 text-white shadow-sm", STATUS_META[item.status].bar)} style={{ left: `${item.left}%`, top: `${4 + item.lane * 28}px`, width: `${item.width}%` }} title={`${appointmentLabel(item.appointment)} · ${STATUS_META[item.status].label}`} aria-label={`${appointmentLabel(item.appointment)}，${STATUS_META[item.status].label}`} role="img"><span className="block truncate">{appointmentLabel(item.appointment)}</span></div>)}</div></div>;
}

function QueueRow({ item }: { item: QueueItem }) { return <li className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 px-3 py-3 text-sm"><div className="min-w-0"><div className="flex flex-wrap items-center gap-2"><span className="break-all font-mono font-medium">{appointmentLabel(item.appointment)}</span><StatusBadge status={STATUS_META[item.status].badge} label={STATUS_META[item.status].label} size="sm" /></div><div className="mt-1 text-xs text-muted-foreground">{item.dockLabel} · {item.appointment.driver_name || "未填司机"} · {item.appointment.vehicle_plate_no || "未填车牌"}</div></div><time className="shrink-0 text-xs tabular-nums text-muted-foreground" dateTime={item.appointment.window_start_at}>{formatDateTime(item.startMs)} - {formatDateTime(item.endMs)}</time></li>; }

function currentDockStatus(appointments: DockAppointment[], now: Date): DockBoardStatus {
  const statuses = appointments.map((appointment) => getDockBoardStatus(appointment, now));
  if (statuses.includes("arrived")) return "arrived";
  if (statuses.includes("occupied")) return "occupied";
  if (statuses.includes("timeout")) return "timeout";
  return "idle";
}

function addLanes(items: TimelineItem[]): TimelineItem[] {
  const laneEnds: number[] = [];
  return [...items].sort((left, right) => left.startMs - right.startMs).map((item) => { const available = laneEnds.findIndex((endMs) => endMs <= item.startMs); const lane = available === -1 ? laneEnds.length : available; laneEnds[lane] = item.endMs; return { ...item, lane }; });
}

function safeNow(now: Date): number { return Number.isFinite(now.getTime()) ? now.getTime() : 0; }
function parseDate(value: string | null | undefined): number | null { const parsed = Date.parse(value ?? ""); return Number.isFinite(parsed) ? parsed : null; }
function formatDateTime(timestamp: number): string { const date = new Date(timestamp); return Number.isNaN(date.getTime()) ? "日期异常" : date.toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", hour12: false }); }
function dockLabel(dock: Dock): string { return dock.dock_code?.trim() || "未命名月台"; }
function appointmentLabel(appointment: DockAppointment): string { return appointment.appointment_no?.trim() || "未编号预约"; }
