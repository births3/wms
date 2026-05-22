import { useState } from "react";
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
import {
  StatusBadge,
  DiffPanel,
  PageHeader,
  DataTable,
  type DataTableColumn,
} from "@/components/business";

interface AuditEvent {
  id: string;
  time: string;
  actor: string;
  action: string;
  module: string;
  resource: string;
  result: "completed" | "unqualified";
  detail?: { before?: Record<string, string>; after?: Record<string, string> };
}

const MOCK_EVENTS: AuditEvent[] = [
  {
    id: "AE-2026-0001",
    time: "2026-05-22 09:14:23",
    actor: "张三 (u001)",
    action: "验收提交",
    module: "M2 入库",
    resource: "ASN PO-2026-0001",
    result: "completed",
    detail: {
      before: { 状态: "验收中", 验收员: "—", 数量: "未确认" },
      after: { 状态: "已验收", 验收员: "u001", 数量: "240 瓶" },
    },
  },
  { id: "AE-2026-0002", time: "2026-05-22 09:18:11", actor: "李四 (u002)", action: "双人复核签字", module: "M2 入库", resource: "ASN PO-2026-0001", result: "completed" },
  { id: "AE-2026-0003", time: "2026-05-22 10:02:55", actor: "王五 (u003)", action: "库位移动", module: "M3 库存", resource: "Pallet LPN-001234", result: "completed" },
  {
    id: "AE-2026-0004",
    time: "2026-05-22 10:15:42",
    actor: "赵六 (u004)",
    action: "批号调整",
    module: "M-BA",
    resource: "批号 20260301A → 20260301B",
    result: "unqualified",
    detail: { before: { 状态: "待审批" }, after: { 状态: "驳回（理由：未提供调整原因）" } },
  },
  { id: "AE-2026-0005", time: "2026-05-22 11:30:08", actor: "张三 (u001)", action: "出库复核", module: "M4 出库", resource: "SO SO-2026-0042", result: "completed" },
];

const COLUMNS: DataTableColumn<AuditEvent>[] = [
  { key: "time", header: "时间" },
  { key: "actor", header: "actor" },
  { key: "module", header: "模块" },
  { key: "action", header: "操作" },
  { key: "resource", header: "对象", mono: true },
  {
    key: "result",
    header: "结果",
    render: (e) => <StatusBadge status={e.result} size="sm" label={e.result === "completed" ? "成功" : "失败"} />,
  },
];

/**
 * H2AuditQuery — 审计追踪查询页（重构：PageHeader + DataTable）
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-H2-002
 * Wave：Wave 0.5（P0 必交付）
 * 业务约束：append-only（GSP 合规）；P99 ≤ 2s
 *
 * @example
 *   <H2AuditQuery />
 */
export function H2AuditQuery() {
  const [selectedId, setSelectedId] = useState<string | null>(MOCK_EVENTS[0].id);
  const selected = MOCK_EVENTS.find((e) => e.id === selectedId);

  return (
    <div data-device="pc" className="w-full max-w-[1280px] min-h-[800px] bg-muted/40 rounded-xl border overflow-hidden flex flex-col font-sans">
      <div className="bg-background border-b px-6 py-4">
        <PageHeader
          title="审计追踪查询"
          subtitle="H2 / append-only · GSP 法定台账"
          actions={
            <>
              <Button size="sm" variant="outline">导出 CSV</Button>
              <Button size="sm" variant="outline">导出 PDF</Button>
              <Button size="sm">追溯码反查</Button>
            </>
          }
          className="mb-0"
        />
      </div>

      {/* Filters */}
      <div className="bg-background px-6 py-4 border-b grid grid-cols-[200px_200px_200px_200px_200px_1fr] gap-3 items-end">
        <FilterField label="时间范围"><Input className="h-8 text-xs" defaultValue="2026-05-22 ~ 2026-05-22" /></FilterField>
        <FilterField label="操作人 actor"><Input className="h-8 text-xs" placeholder="工号 / 姓名" /></FilterField>
        <FilterField label="模块">
          <Select defaultValue="all">
            <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="M2">M2 入库</SelectItem>
              <SelectItem value="M3">M3 库存</SelectItem>
              <SelectItem value="M4">M4 出库</SelectItem>
              <SelectItem value="M-BA">M-BA 批号调整</SelectItem>
            </SelectContent>
          </Select>
        </FilterField>
        <FilterField label="action 类型">
          <Select defaultValue="all">
            <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="crud">create / update / delete</SelectItem>
              <SelectItem value="state">状态变更</SelectItem>
              <SelectItem value="approval">审批 / 签字</SelectItem>
            </SelectContent>
          </Select>
        </FilterField>
        <FilterField label="结果">
          <Select defaultValue="all">
            <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="ok">成功</SelectItem>
              <SelectItem value="fail">失败 / 驳回</SelectItem>
            </SelectContent>
          </Select>
        </FilterField>
        <div className="flex gap-2 justify-end">
          <Button size="sm" variant="outline">重置</Button>
          <Button size="sm">查询</Button>
        </div>
      </div>

      {/* Body */}
      <div className="flex-1 grid grid-cols-[1fr_480px] min-h-0">
        <div className="bg-background border-r overflow-auto">
          <DataTable
            className="rounded-none border-0"
            columns={COLUMNS}
            data={MOCK_EVENTS}
            rowKey={(e) => e.id}
            selectedKey={selectedId ?? undefined}
            onRowClick={(e) => setSelectedId(e.id)}
            caption={`共 ${MOCK_EVENTS.length} 条 · 按时间倒序 · P99 0.42s`}
            footer={
              <div className="px-4 py-2 text-xs text-muted-foreground flex justify-between items-center">
                <span>每页 100 条 · 共 {MOCK_EVENTS.length} 条 · 第 1/1 页</span>
                <span className="flex gap-2">
                  <Button size="sm" variant="outline" disabled>上一页</Button>
                  <Button size="sm" variant="outline" disabled>下一页</Button>
                </span>
              </div>
            }
          />
        </div>

        <div className="bg-background overflow-auto">
          {selected && (
            <div className="p-6">
              <p className="text-xs text-muted-foreground">{selected.id}</p>
              <h3 className="text-lg font-semibold mt-1 mb-4">{selected.action}</h3>
              <div className="grid grid-cols-[80px_1fr] gap-2 text-sm mb-6">
                <DetailRow k="时间" v={selected.time} />
                <DetailRow k="actor" v={selected.actor} />
                <DetailRow k="模块" v={selected.module} />
                <DetailRow k="对象" v={<span className="font-mono text-xs">{selected.resource}</span>} />
                <DetailRow k="结果" v={<StatusBadge status={selected.result} size="sm" label={selected.result === "completed" ? "成功" : "失败"} />} />
                <DetailRow k="IP" v={<span className="font-mono text-xs">10.2.0.18</span>} />
                <DetailRow k="设备" v={<span className="font-mono text-xs">PDA-A1B2C3</span>} />
              </div>
              <p className="text-sm font-medium mb-2">变更对比</p>
              <DiffPanel before={selected.detail?.before} after={selected.detail?.after} />
              <p className="mt-6 pt-4 border-t text-xs text-muted-foreground">
                ⓘ append-only：此记录不可修改，不可删除（GSP 合规）
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function FilterField({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1">
      <Label className="text-xs text-muted-foreground">{label}</Label>
      {children}
    </div>
  );
}

function DetailRow({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <>
      <div className="text-xs text-muted-foreground">{k}</div>
      <div>{v}</div>
    </>
  );
}
