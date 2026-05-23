import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import {
  PageHeader,
  DataTable,
  StatusBadge,
  type DataTableColumn,
} from "@/components/business";
import { Plus, Mail, Clock, Pause, Play, Edit, Trash2, History } from "lucide-react";

interface Subscription {
  id: string;
  reportName: string;
  schedule: string;
  recipients: string[];
  format: "PDF" | "Excel" | "Both";
  lastRun: string;
  lastStatus: "success" | "failed" | "pending";
  enabled: boolean;
  totalRuns: number;
}

const MOCK: Subscription[] = [
  { id: "s1", reportName: "采购入库月报", schedule: "每月 1 号 02:00", recipients: ["zhangsan@wms.com", "lisi@wms.com"],
    format: "Both", lastRun: "2026-05-01 02:15", lastStatus: "success", enabled: true, totalRuns: 12 },
  { id: "s2", reportName: "冷链温度月报", schedule: "每月 1 号 02:30", recipients: ["wangwu@wms.com", "compliance@wms.com"],
    format: "PDF", lastRun: "2026-05-01 02:32", lastStatus: "success", enabled: true, totalRuns: 12 },
  { id: "s3", reportName: "供应商月度采购排行（自定义）", schedule: "每周一 09:00",
    recipients: ["procurement@wms.com"], format: "Excel", lastRun: "2026-05-19 09:00",
    lastStatus: "success", enabled: true, totalRuns: 26 },
  { id: "s4", reportName: "近效期/不合格月报", schedule: "每月 25 号 09:00",
    recipients: ["quality@wms.com", "warehouse@wms.com"], format: "Both",
    lastRun: "2026-04-25 09:01", lastStatus: "failed", enabled: false, totalRuns: 4 },
  { id: "s5", reportName: "特殊药品台账（麻精）", schedule: "每月 1 号 03:00",
    recipients: ["compliance@wms.com", "manager@wms.com"], format: "PDF",
    lastRun: "2026-05-01 03:02", lastStatus: "success", enabled: true, totalRuns: 12 },
];

interface RunHistory {
  time: string;
  subscription: string;
  recipientCount: number;
  fileSize: string;
  status: "success" | "failed";
  error?: string;
}

const HISTORY: RunHistory[] = [
  { time: "2026-05-19 09:00:12", subscription: "供应商月度采购排行", recipientCount: 1, fileSize: "245 KB", status: "success" },
  { time: "2026-05-12 09:00:08", subscription: "供应商月度采购排行", recipientCount: 1, fileSize: "238 KB", status: "success" },
  { time: "2026-05-05 09:00:15", subscription: "供应商月度采购排行", recipientCount: 1, fileSize: "242 KB", status: "success" },
  { time: "2026-05-01 03:02:24", subscription: "特殊药品台账", recipientCount: 2, fileSize: "1.2 MB", status: "success" },
  { time: "2026-05-01 02:32:18", subscription: "冷链温度月报", recipientCount: 2, fileSize: "5.1 MB", status: "success" },
  { time: "2026-05-01 02:15:42", subscription: "采购入库月报", recipientCount: 2, fileSize: "1.2 MB", status: "success" },
  { time: "2026-04-25 09:01:33", subscription: "近效期/不合格月报", recipientCount: 2, fileSize: "—",
    status: "failed", error: "SMTP 连接超时（quality@wms.com）" },
];

/**
 * M6Subscriptions — M6-005 报表订阅
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M6-005（报表定时订阅 + 邮件分发 + 历史记录）
 * Wave：Wave 2.5（订阅引擎正式上线）
 * 业务约束：订阅触发写 H2 审计；邮件含数据签名 + 留 30 天历史；权限按订阅人而非创建人
 *
 * 参考 ADR-0023 D：订阅 WMS 自实现（不依赖 Metabase）
 *
 * @example
 *   <M6Subscriptions />
 */
export function M6Subscriptions() {
  const cols: DataTableColumn<Subscription>[] = [
    { key: "name", header: "报表",
      render: (r) => <div>
        <div className="text-sm font-medium">{r.reportName}</div>
        <div className="text-xs text-muted-foreground">已运行 {r.totalRuns} 次</div>
      </div>
    },
    { key: "schedule", header: "周期",
      render: (r) => <div className="flex items-center gap-1 text-xs">
        <Clock className="h-3 w-3 text-muted-foreground" />
        {r.schedule}
      </div>
    },
    { key: "recipients", header: "收件人",
      render: (r) => (
        <div className="text-xs space-y-0.5">
          {r.recipients.slice(0, 2).map((e) => (
            <div key={e} className="flex items-center gap-1">
              <Mail className="h-3 w-3 text-muted-foreground" />
              <span className="font-mono">{e}</span>
            </div>
          ))}
          {r.recipients.length > 2 && (
            <div className="text-muted-foreground italic">+{r.recipients.length - 2} 人</div>
          )}
        </div>
      ),
    },
    { key: "format", header: "格式",
      render: (r) => <span className="text-[11px] px-1.5 py-0.5 bg-muted rounded">{r.format}</span>
    },
    { key: "lastRun", header: "上次运行",
      render: (r) => <div className="text-xs">
        <div className="font-mono">{r.lastRun}</div>
        {r.lastStatus === "success" && <StatusBadge status="completed" size="sm" label="成功" />}
        {r.lastStatus === "failed" && <StatusBadge status="unqualified" size="sm" label="失败" />}
        {r.lastStatus === "pending" && <StatusBadge status="pending" size="sm" />}
      </div>
    },
    { key: "enabled", header: "状态",
      render: (r) => r.enabled ? (
        <StatusBadge status="qualified" size="sm" label="启用" />
      ) : (
        <StatusBadge status="isolated" size="sm" label="已暂停" />
      ),
    },
    { key: "actions", header: "操作",
      render: (r) => (
        <div className="flex gap-1">
          <Button variant="ghost" size="sm" className="h-7 px-2">
            <Edit className="h-3 w-3" />
          </Button>
          <Button variant="ghost" size="sm" className="h-7 px-2">
            {r.enabled ? <Pause className="h-3 w-3" /> : <Play className="h-3 w-3" />}
          </Button>
          <Button variant="ghost" size="sm" className="h-7 px-2 text-destructive">
            <Trash2 className="h-3 w-3" />
          </Button>
        </div>
      ),
    },
  ];

  const historyCols: DataTableColumn<RunHistory>[] = [
    { key: "time", header: "时间", render: (r) => <span className="font-mono text-xs">{r.time}</span> },
    { key: "sub", header: "报表订阅", render: (r) => <span className="text-sm">{r.subscription}</span> },
    { key: "rcpt", header: "收件人数", align: "right", render: (r) => <span className="text-sm">{r.recipientCount}</span> },
    { key: "size", header: "附件大小", align: "right", render: (r) => <span className="text-xs font-mono">{r.fileSize}</span> },
    { key: "status", header: "结果",
      render: (r) => r.status === "success" ? (
        <StatusBadge status="completed" size="sm" label="成功" />
      ) : (
        <div className="flex flex-col gap-0.5">
          <StatusBadge status="unqualified" size="sm" label="失败" />
          <span className="text-[10px] text-destructive">{r.error}</span>
        </div>
      ),
    },
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="报表订阅"
        subtitle="M6-005 · 定时触发 + 邮件分发 + 历史记录 · 含数据签名"
        actions={
          <Button size="sm">
            <Plus className="h-4 w-4 mr-1" /> 新建订阅
          </Button>
        }
      />

      {/* KPI */}
      <div className="px-6 py-4 border-b grid grid-cols-5 gap-3">
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">订阅总数</div>
          <div className="text-2xl font-bold mt-1">8</div>
        </Card>
        <Card className="p-3 border-wms-success/40 bg-wms-success/5">
          <div className="text-xs text-wms-success">启用</div>
          <div className="text-2xl font-bold mt-1 text-wms-success">7</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">本月运行</div>
          <div className="text-2xl font-bold mt-1">42</div>
        </Card>
        <Card className="p-3 border-wms-success/40 bg-wms-success/5">
          <div className="text-xs text-wms-success">成功率</div>
          <div className="text-2xl font-bold mt-1 text-wms-success">97.6%</div>
        </Card>
        <Card className="p-3 border-destructive/40 bg-destructive/5">
          <div className="text-xs text-destructive">失败</div>
          <div className="text-2xl font-bold mt-1 text-destructive">1</div>
          <div className="text-[11px] text-destructive">SMTP 超时</div>
        </Card>
      </div>

      {/* 订阅列表 + 新建表单 */}
      <div className="px-6 py-4 grid grid-cols-[1fr_320px] gap-4">
        <div>
          <div className="text-sm font-semibold mb-2">订阅列表（5 条）</div>
          <DataTable columns={cols} data={MOCK} rowKey={(r) => r.id} />
        </div>

        {/* 新建订阅表单（侧栏）*/}
        <Card className="p-4">
          <div className="text-sm font-semibold mb-3 flex items-center gap-2">
            <Plus className="h-4 w-4" /> 新建订阅
          </div>
          <div className="space-y-2.5">
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">报表 *</label>
              <Select defaultValue="purchase">
                <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="purchase">采购入库月报</SelectItem>
                  <SelectItem value="sales">销售出库月报</SelectItem>
                  <SelectItem value="cold">冷链温度月报</SelectItem>
                  <SelectItem value="custom1">自定义：供应商排行</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">触发周期 *</label>
              <Select defaultValue="monthly_1st">
                <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="daily">每日</SelectItem>
                  <SelectItem value="weekly_mon">每周一</SelectItem>
                  <SelectItem value="weekly_fri">每周五</SelectItem>
                  <SelectItem value="monthly_1st">每月 1 号</SelectItem>
                  <SelectItem value="monthly_25">每月 25 号</SelectItem>
                  <SelectItem value="quarterly">每季度</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">触发时间</label>
              <Input type="time" defaultValue="02:00" className="h-8 text-xs" />
            </div>
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">收件人 *</label>
              <Input placeholder="多人用逗号分隔" defaultValue="" className="h-8 text-xs" />
              <div className="text-[11px] text-muted-foreground mt-1">支持邮箱组：@team-procurement / @team-quality</div>
            </div>
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">附件格式</label>
              <Select defaultValue="both">
                <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="pdf">仅 PDF</SelectItem>
                  <SelectItem value="excel">仅 Excel</SelectItem>
                  <SelectItem value="both">PDF + Excel</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center gap-2 pt-2">
              <input type="checkbox" defaultChecked id="audit" className="accent-primary" />
              <label htmlFor="audit" className="text-xs">写入 H2 审计</label>
            </div>
            <Button size="sm" className="w-full mt-2">创建订阅</Button>
          </div>
        </Card>
      </div>

      {/* 历史记录 */}
      <div className="px-6 py-4 border-t">
        <div className="flex items-center justify-between mb-2">
          <div className="text-sm font-semibold flex items-center gap-2">
            <History className="h-4 w-4" /> 运行历史（最近 7 条）
          </div>
          <span className="text-xs text-muted-foreground">保留 30 天 · 含数据签名</span>
        </div>
        <DataTable columns={historyCols} data={HISTORY} rowKey={(r) => r.time} />
      </div>
    </div>
  );
}
