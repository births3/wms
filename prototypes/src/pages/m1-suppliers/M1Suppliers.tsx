import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import { Input } from "@wms/ui";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@wms/ui";
import {
  StatusBadge,
  PageHeader,
  DataTable,
  type DataTableColumn,
} from "@wms/ui";
import { Plus, Search, AlertTriangle, FileText, Calendar, Star } from "lucide-react";

interface Supplier {
  id: string;
  code: string;
  name: string;
  legalRep: string;
  contact: string;
  /** 经营范围（药品类别） */
  scope: string[];
  /** GSP 证有效期 */
  gspExpiresAt: string;
  /** 营业执照有效期 */
  bizExpiresAt: string;
  /** 质量评分 0-100 */
  score: number;
  /** 资质状态 */
  status: "valid" | "warning" | "expired" | "blacklist";
  recentIssues: number;
}

const MOCK: Supplier[] = [
  { id: "s1", code: "SUP-0001", name: "国药控股北京有限公司", legalRep: "李建国", contact: "010-8888 0001",
    scope: ["处方药", "OTC", "冷链", "麻醉药"], gspExpiresAt: "2028-06-15", bizExpiresAt: "2030-12-31",
    score: 96, status: "valid", recentIssues: 0 },
  { id: "s2", code: "SUP-0002", name: "上海医药华东分公司", legalRep: "陈晓东", contact: "021-6666 8888",
    scope: ["处方药", "OTC", "冷链"], gspExpiresAt: "2026-08-30", bizExpiresAt: "2029-05-20",
    score: 88, status: "warning", recentIssues: 2 },
  { id: "s3", code: "SUP-0003", name: "九州通医药集团", legalRep: "王翔", contact: "027-5555 0099",
    scope: ["处方药", "OTC"], gspExpiresAt: "2027-03-22", bizExpiresAt: "2028-11-08",
    score: 92, status: "valid", recentIssues: 1 },
  { id: "s4", code: "SUP-0004", name: "甘李药业股份有限公司", legalRep: "刘世高", contact: "010-3399 8800",
    scope: ["冷链", "处方药"], gspExpiresAt: "2026-06-01", bizExpiresAt: "2030-08-15",
    score: 95, status: "warning", recentIssues: 0 },
  { id: "s5", code: "SUP-0005", name: "已注销 - XX 医药贸易", legalRep: "（已注销）", contact: "—",
    scope: ["OTC"], gspExpiresAt: "2025-12-30", bizExpiresAt: "2025-12-30",
    score: 65, status: "expired", recentIssues: 5 },
  { id: "s6", code: "SUP-0006", name: "黑名单 - YY 药品代理", legalRep: "张某", contact: "—",
    scope: ["—"], gspExpiresAt: "—", bizExpiresAt: "—",
    score: 30, status: "blacklist", recentIssues: 12 },
];

/**
 * M1Suppliers — M1-002 供应商资质档案
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M1-002（GSP 证有效期 / 营业执照 / 经营范围 / 质量评分 / 黑名单）
 * Wave：Wave 2.0（M1 基础数据）
 * 业务约束：GSP 证 < 60 天预警；过期/黑名单不可选；冷链供应商需冷链经营许可证
 *
 * @example
 *   <M1Suppliers />
 */
export function M1Suppliers() {
  const cols: DataTableColumn<Supplier>[] = [
    {
      key: "code",
      header: "供应商编码",
      render: (r) => <span className="font-mono text-xs text-primary">{r.code}</span>,
    },
    {
      key: "name",
      header: "公司名称 / 法人",
      render: (r) => (
        <div>
          <div className="font-medium text-sm">{r.name}</div>
          <div className="text-xs text-muted-foreground mt-0.5">法人：{r.legalRep}</div>
        </div>
      ),
    },
    {
      key: "scope",
      header: "经营范围",
      render: (r) => (
        <div className="flex flex-wrap gap-1">
          {r.scope.map((s) => (
            <span key={s} className="text-[11px] px-1.5 py-0.5 bg-muted rounded">{s}</span>
          ))}
        </div>
      ),
    },
    {
      key: "gspExpiresAt",
      header: "GSP 证 / 营业执照",
      render: (r) => {
        if (r.gspExpiresAt === "—") return <span className="text-muted-foreground">—</span>;
        const gspDate = new Date(r.gspExpiresAt);
        const today = new Date("2026-05-22");
        const daysLeft = Math.floor((gspDate.getTime() - today.getTime()) / (1000 * 60 * 60 * 24));
        const isExpiring = daysLeft < 90 && daysLeft > 0;
        const isExpired = daysLeft <= 0;
        return (
          <div className="text-xs flex flex-col gap-0.5">
            <div className="flex items-center gap-1">
              <Calendar className="size-3 text-muted-foreground" />
              <span className={isExpired ? "text-destructive font-medium" : isExpiring ? "text-wms-warning font-medium" : ""}>
                GSP {r.gspExpiresAt} ({isExpired ? "已过期" : `剩 ${daysLeft} 天`})
              </span>
            </div>
            <div className="flex items-center gap-1 text-muted-foreground">
              <FileText className="size-3" />
              <span>营 {r.bizExpiresAt}</span>
            </div>
          </div>
        );
      },
    },
    {
      key: "score",
      header: "质量评分",
      render: (r) => {
        const color = r.score >= 90 ? "text-wms-success" : r.score >= 80 ? "text-wms-warning" : "text-destructive";
        return (
          <div className="flex items-center gap-1">
            <Star className={`size-3.5 ${color} fill-current`} />
            <span className={`text-sm font-semibold ${color}`}>{r.score}</span>
            {r.recentIssues > 0 && (
              <span className="ml-1 text-[11px] text-destructive">异常 {r.recentIssues}</span>
            )}
          </div>
        );
      },
    },
    {
      key: "status",
      header: "状态",
      render: (r) =>
        r.status === "valid" ? (
          <StatusBadge status="qualified" size="sm" label="正常" />
        ) : r.status === "warning" ? (
          <StatusBadge status="pending" size="sm" label="证照预警" />
        ) : r.status === "expired" ? (
          <StatusBadge status="expired" size="sm" label="已注销" />
        ) : (
          <StatusBadge status="unqualified" size="sm" label="黑名单" />
        ),
    },
    {
      key: "actions",
      header: "操作",
      render: () => (
        <div className="flex gap-1">
          <Button variant="ghost" size="sm" className="h-7 px-2">详情</Button>
          <Button variant="ghost" size="sm" className="h-7 px-2">证照</Button>
        </div>
      ),
    },
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="供应商资质档案"
        subtitle="M1-002 · GSP 证（如 GSP-BJ-2026-0001）+ 营业执照 + 经营范围 + 质量评分 · 维护 u001"
        actions={
          <>
            <Button size="sm" variant="ghost" onClick={() => (window.location.hash = "#m6-custom")} title="另存为自定义报表">
              ⇲ 另存为报表
            </Button>
            <Button variant="outline" size="sm">导出名册</Button>
            <Button size="sm">
              <Plus data-icon="inline-start" /> 新增供应商
            </Button>
          </>
        }
      />

      {/* 资质告警栏 */}
      <div className="mx-6 mt-4 p-3 bg-wms-warning/10 border border-wms-warning/30 rounded-md flex items-start gap-2">
        <AlertTriangle className="size-4 text-wms-warning flex-shrink-0 mt-0.5" />
        <div className="text-xs flex-1">
          <span className="font-medium text-wms-warning">证照预警</span>
          <span className="text-muted-foreground ml-2">
            8 家供应商 GSP 证将在 90 天内过期 · 2 家营业执照已过期 · 1 家在黑名单
          </span>
        </div>
        <Button variant="link" size="sm" className="text-xs h-auto py-0">查看 →</Button>
      </div>

      {/* 筛选栏 */}
      <div className="px-6 py-4 border-b grid grid-cols-5 gap-3 items-end">
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">搜索</label>
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
            <Input className="pl-9" placeholder="编码 / 公司名 / 法人" />
          </div>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">经营范围</label>
          <Select>
            <SelectTrigger>
              <SelectValue placeholder="全部" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="rx">处方药</SelectItem>
              <SelectItem value="cold">冷链</SelectItem>
              <SelectItem value="narcotic">麻醉药</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">证照状态</label>
          <Select>
            <SelectTrigger>
              <SelectValue placeholder="全部" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="valid">正常</SelectItem>
              <SelectItem value="expiring">即将过期</SelectItem>
              <SelectItem value="expired">已过期</SelectItem>
              <SelectItem value="blacklist">黑名单</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">评分范围</label>
          <Select>
            <SelectTrigger>
              <SelectValue placeholder="全部" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="excellent">≥ 90</SelectItem>
              <SelectItem value="good">80-89</SelectItem>
              <SelectItem value="poor">&lt; 80</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">重置</Button>
          <Button size="sm">查询</Button>
        </div>
      </div>

      <DataTable columns={cols} data={MOCK} rowKey={(r) => r.id} />

      <Card className="mx-6 my-4 p-3 bg-muted/30">
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground">每页 50 条 · 共 156 条 · 第 1/4 页</span>
          <div className="flex gap-1.5">
            <Button variant="outline" size="sm" disabled>上一页</Button>
            <Button variant="outline" size="sm">下一页</Button>
          </div>
        </div>
      </Card>
    </div>
  );
}
