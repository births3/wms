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
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  StatusBadge,
  PageHeader,
  DataTable,
  type DataTableColumn,
} from "@/components/business";
import { Database, Archive, Plus } from "lucide-react";
import { useState } from "react";

interface ArchiveRule {
  id: string;
  scope: string;
  retentionYears: number;
  archiveAfter: string;
  destination: string;
  encryption: "aes256" | "none";
  status: "enabled" | "disabled";
}

const MOCK_RULES: ArchiveRule[] = [
  { id: "r1", scope: "audit_trail（H2 审计追踪）", retentionYears: 30, archiveAfter: "180 天后归档", destination: "S3 冷存储 (cn-northwest-1)", encryption: "aes256", status: "enabled" },
  { id: "r2", scope: "inventory_movement（M3 库存流水）", retentionYears: 5, archiveAfter: "365 天后归档", destination: "S3 冷存储 (cn-northwest-1)", encryption: "aes256", status: "enabled" },
  { id: "r3", scope: "inbound_record（M2 入库记录）", retentionYears: 5, archiveAfter: "365 天后归档", destination: "PostgreSQL 归档分区", encryption: "aes256", status: "enabled" },
  { id: "r4", scope: "cold_chain_temp（M5 冷链温控）", retentionYears: 5, archiveAfter: "90 天后归档", destination: "TimescaleDB 压缩块", encryption: "aes256", status: "enabled" },
  { id: "r5", scope: "trace_code_event（M-TC 追溯码原始事件）", retentionYears: 5, archiveAfter: "180 天后归档", destination: "S3 冷存储 (cn-northwest-1)", encryption: "aes256", status: "disabled" },
];

interface ArchiveJob {
  id: string;
  scope: string;
  startedAt: string;
  duration: string;
  recordCount: number;
  size: string;
  status: "completed" | "running" | "failed";
}

const MOCK_JOBS: ArchiveJob[] = [
  { id: "j1", scope: "audit_trail", startedAt: "今日 02:00", duration: "12m 34s", recordCount: 245890, size: "1.2 GB", status: "completed" },
  { id: "j2", scope: "inventory_movement", startedAt: "今日 02:30", duration: "8m 12s", recordCount: 89234, size: "420 MB", status: "completed" },
  { id: "j3", scope: "cold_chain_temp", startedAt: "今日 03:00", duration: "进行中 (已 4m)", recordCount: 1280450, size: "—", status: "running" },
  { id: "j4", scope: "inbound_record", startedAt: "昨日 02:00", duration: "—", recordCount: 0, size: "—", status: "failed" },
];

const STATUS_MAP = {
  enabled: { status: "qualified" as const, label: "已启用" },
  disabled: { status: "isolated" as const, label: "已停用" },
  completed: { status: "completed" as const, label: "成功" },
  running: { status: "in_progress" as const, label: "进行中" },
  failed: { status: "unqualified" as const, label: "失败" },
};

/**
 * H2Archive — 审计/业务数据归档配置 + 生命周期
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-H2-004（审计归档）/ US-H2-006（业务数据生命周期）
 * Wave：Wave 0.5（P0 必交付）
 * 业务约束：审计 ≥ 30 年（GSP 永久）/ 业务 ≥ 5 年；归档前必须 AES256 加密 + 异地副本
 *
 * @example
 *   <H2Archive />
 */
export function H2Archive() {
  const [tab, setTab] = useState<"rules" | "jobs">("rules");

  const ruleColumns: DataTableColumn<ArchiveRule>[] = [
    { key: "scope", header: "数据范围", className: "font-medium" },
    {
      key: "retentionYears",
      header: "保留",
      render: (r) => (
        <span className={r.retentionYears >= 30 ? "text-destructive font-semibold" : ""}>
          {r.retentionYears} 年{r.retentionYears >= 30 && "（GSP）"}
        </span>
      ),
    },
    { key: "archiveAfter", header: "归档触发", className: "text-muted-foreground" },
    { key: "destination", header: "归档目的地", mono: true },
    { key: "encryption", header: "加密", className: "uppercase text-xs" },
    {
      key: "status",
      header: "状态",
      render: (r) => {
        const m = STATUS_MAP[r.status];
        return <StatusBadge status={m.status} size="sm" label={m.label} />;
      },
    },
    { key: "action", header: "操作", render: () => <Button variant="outline" size="sm">编辑</Button> },
  ];

  const jobColumns: DataTableColumn<ArchiveJob>[] = [
    { key: "scope", header: "数据范围", mono: true },
    { key: "startedAt", header: "开始时间", className: "text-muted-foreground" },
    { key: "duration", header: "耗时", className: "text-muted-foreground" },
    { key: "recordCount", header: "归档记录数", render: (j) => <span className="font-mono">{j.recordCount.toLocaleString()}</span> },
    { key: "size", header: "归档大小" },
    {
      key: "status",
      header: "状态",
      render: (j) => {
        const m = STATUS_MAP[j.status];
        return <StatusBadge status={m.status} size="sm" label={m.label} />;
      },
    },
    {
      key: "action",
      header: "操作",
      render: (j) =>
        j.status === "failed" ? <Button variant="outline" size="sm">重试</Button> :
        j.status === "completed" ? <Button variant="link" size="sm" className="px-0">查看日志</Button> : null,
    },
  ];

  return (
    <div className="w-full max-w-[1280px] min-h-[800px] bg-muted/40 border rounded-xl p-6 font-sans">
      <PageHeader
        title="数据归档与生命周期"
        subtitle="H2-004 审计归档 / H2-006 业务数据生命周期 · GSP 法定保留"
        actions={
          <>
            <Button variant="outline" size="sm"><Archive className="size-4" />手动触发归档</Button>
            <Button size="sm"><Plus className="size-4" />新建规则</Button>
          </>
        }
      />

      {/* 总览卡片 */}
      <div className="grid grid-cols-4 gap-4 mb-6">
        <SummaryCard label="总归档规则" value="5" sub="启用 4 / 停用 1" />
        <SummaryCard label="今日已归档" value="335,124" sub="条记录 · 1.6 GB" />
        <SummaryCard label="冷存储总量" value="2.4 TB" sub="占容量 12%" />
        <SummaryCard label="最近一次失败" value="1" sub="inbound_record 昨日" critical />
      </div>

      <Tabs value={tab} onValueChange={(v) => setTab(v as "rules" | "jobs")}>
        <TabsList className="mb-4">
          <TabsTrigger value="rules">归档规则（5）</TabsTrigger>
          <TabsTrigger value="jobs">执行历史（最近 30 天）</TabsTrigger>
        </TabsList>
      </Tabs>

      {tab === "rules" && (
        <DataTable
          columns={ruleColumns}
          data={MOCK_RULES}
          rowKey={(r) => r.id}
          caption={
            <div className="grid grid-cols-4 gap-4 items-end">
              <div className="space-y-1">
                <Label className="text-xs">数据范围</Label>
                <Select defaultValue="all">
                  <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">全部</SelectItem>
                    <SelectItem value="audit">审计类</SelectItem>
                    <SelectItem value="business">业务类</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1">
                <Label className="text-xs">保留年限</Label>
                <Select defaultValue="all">
                  <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">全部</SelectItem>
                    <SelectItem value="5">5 年</SelectItem>
                    <SelectItem value="30">30 年</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1">
                <Label className="text-xs">状态</Label>
                <Select defaultValue="all">
                  <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">全部</SelectItem>
                    <SelectItem value="enabled">已启用</SelectItem>
                    <SelectItem value="disabled">已停用</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <Input className="h-8 text-xs" placeholder="搜索规则名 / 数据表" />
            </div>
          }
        />
      )}

      {tab === "jobs" && (
        <DataTable columns={jobColumns} data={MOCK_JOBS} rowKey={(j) => j.id} />
      )}
    </div>
  );
}

function SummaryCard({ label, value, sub, critical }: { label: string; value: string; sub: string; critical?: boolean }) {
  return (
    <Card className="p-4">
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">{label}</span>
        <Database className={`size-4 ${critical ? "text-destructive" : "text-muted-foreground"}`} />
      </div>
      <div className={`text-2xl font-semibold mt-1 ${critical ? "text-destructive" : ""}`}>{value}</div>
      <div className="text-xs text-muted-foreground mt-1">{sub}</div>
    </Card>
  );
}
