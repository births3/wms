import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import {
  PageHeader,
  AuditTimeline,
  StatusBadge,
  type AuditTimelineEvent,
} from "@/components/business";
import { Download, FileSpreadsheet, FileText, ChevronRight } from "lucide-react";

/**
 * M6Reports — M6-002 GSP 报表
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M6-002（GSP 法定报表 / 月报 / 年报 / 监管要求格式 / 审计追溯）
 * Wave：Wave 3.0（M6 报表合规）
 * 业务约束：报表生成有 H2 审计；导出含数据签名（防篡改）；保留 5 年
 *
 * @example
 *   <M6Reports />
 */

interface Report {
  id: string;
  category: string;
  name: string;
  period: string;
  /** GSP 条款 */
  clause: string;
  status: "ready" | "generating" | "failed";
  generatedAt?: string;
  size?: string;
  format: ("PDF" | "Excel" | "JSON")[];
}

const REPORTS: Report[] = [
  { id: "r1", category: "采购", name: "采购入库月报", period: "2026-04",
    clause: "GSP §83", status: "ready", generatedAt: "2026-05-01 02:15", size: "1.2 MB",
    format: ["PDF", "Excel"] },
  { id: "r2", category: "销售", name: "销售出库月报", period: "2026-04",
    clause: "GSP §85", status: "ready", generatedAt: "2026-05-01 02:18", size: "2.4 MB",
    format: ["PDF", "Excel"] },
  { id: "r3", category: "库存", name: "库存盘点月报", period: "2026-04",
    clause: "GSP §95", status: "ready", generatedAt: "2026-05-01 02:22", size: "3.8 MB",
    format: ["PDF", "Excel"] },
  { id: "r4", category: "冷链", name: "冷链温度月报", period: "2026-04",
    clause: "GSP §64", status: "ready", generatedAt: "2026-05-01 02:25", size: "5.1 MB",
    format: ["PDF", "Excel", "JSON"] },
  { id: "r5", category: "质量", name: "近效期/不合格品月报", period: "2026-04",
    clause: "GSP §50", status: "ready", generatedAt: "2026-05-01 02:28", size: "0.8 MB",
    format: ["PDF", "Excel"] },
  { id: "r6", category: "采购", name: "采购入库月报", period: "2026-05",
    clause: "GSP §83", status: "generating", format: ["PDF", "Excel"] },
];

// 报表生成 + 审计追踪事件
const AUDIT_EVENTS: AuditTimelineEvent[] = [
  {
    id: "e1", time: "2026-05-23 02:25:18", actor: "system",
    action: "生成报表", module: "M6", resource: "冷链温度月报 2026-04",
    status: "completed",
    detail: <div className="text-xs">
      数据范围：2026-04-01 ~ 2026-04-30 · 共 86,400 条温度记录 · MD5: a3f2…b9x7
    </div>,
  },
  {
    id: "e2", time: "2026-05-15 14:30:05", actor: "李四 (u002)",
    action: "下载报表（PDF）", module: "M6", resource: "采购入库月报 2026-04",
    status: "qualified",
    detail: <div className="text-xs">下载次数 +1（累计 3 次）· IP 10.2.0.22</div>,
  },
  {
    id: "e3", time: "2026-05-15 10:08:42", actor: "张三 (u001)",
    action: "下载报表（Excel）", module: "M6", resource: "销售出库月报 2026-04",
    status: "qualified",
    detail: <div className="text-xs">下载次数 +1（累计 5 次）· IP 10.2.0.18</div>,
  },
  {
    id: "e4", time: "2026-05-08 09:12:33", actor: "王五 (u003)",
    action: "导出监管报送（药监 EDI）", module: "M6", resource: "近效期月报 2026-04",
    status: "qualified",
    detail: <div className="text-xs">推送至 ERP（码上放心由 ERP 上报）· 含数据签名</div>,
  },
  {
    id: "e5", time: "2026-05-01 02:15:00", actor: "system",
    action: "批量生成月报（5 份）", module: "M6", resource: "2026-04 月报",
    status: "completed",
    detail: <div className="text-xs">触发：每月 1 日 02:00 cron · 用时 13 分 · 全部成功</div>,
  },
];

export function M6Reports() {
  const [expanded, setExpanded] = useState<string | undefined>("e1");

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="GSP 法定报表"
        subtitle="M6-002 · 月报 + 年报 · 监管报送格式 · 含审计追溯"
        actions={
          <>
            <Button variant="outline" size="sm">报表配置</Button>
            <Button size="sm">手动生成</Button>
          </>
        }
      />

      <div className="px-6 py-4 grid grid-cols-[1fr_360px] gap-6">
        {/* 左侧：报表列表 */}
        <div>
          <div className="flex items-center justify-between mb-3">
            <div className="text-sm font-semibold">2026 年 4 月报表</div>
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <span>下次生成：6/1 02:00</span>
            </div>
          </div>
          <div className="space-y-2">
            {REPORTS.map((r) => (
              <Card key={r.id} className="p-4 hover:bg-muted/30 cursor-pointer">
                <div className="flex items-start gap-3">
                  <div className="flex-shrink-0 w-12 h-12 rounded bg-primary/10 flex items-center justify-center">
                    <FileText className="h-5 w-5 text-primary" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="text-sm font-semibold">{r.name}</span>
                      <span className="text-[11px] px-1.5 py-0.5 bg-muted rounded font-mono text-muted-foreground">
                        {r.period}
                      </span>
                      <span className="text-[11px] text-muted-foreground">{r.clause}</span>
                    </div>
                    <div className="flex items-center gap-3 text-xs text-muted-foreground">
                      <span>分类：{r.category}</span>
                      {r.generatedAt && <span>· 生成于 {r.generatedAt}</span>}
                      {r.size && <span>· {r.size}</span>}
                    </div>
                  </div>
                  <div className="flex flex-col items-end gap-2">
                    {r.status === "ready" ? (
                      <StatusBadge status="completed" size="sm" label="就绪" />
                    ) : r.status === "generating" ? (
                      <StatusBadge status="in_progress" size="sm" label="生成中" />
                    ) : (
                      <StatusBadge status="unqualified" size="sm" label="失败" />
                    )}
                    {r.status === "ready" && (
                      <div className="flex gap-1">
                        {r.format.map((f) => (
                          <Button key={f} variant="ghost" size="sm" className="h-7 px-2 text-xs">
                            {f === "Excel" ? <FileSpreadsheet className="h-3 w-3 mr-1" /> :
                             f === "PDF" ? <FileText className="h-3 w-3 mr-1" /> :
                             <Download className="h-3 w-3 mr-1" />}
                            {f}
                          </Button>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              </Card>
            ))}
          </div>
        </div>

        {/* 右侧：审计追踪 */}
        <div>
          <div className="flex items-center justify-between mb-3">
            <div className="text-sm font-semibold">审计追踪</div>
            <Button variant="ghost" size="sm" className="text-xs h-7">
              查看全部 <ChevronRight className="h-3 w-3 ml-0.5" />
            </Button>
          </div>
          <Card className="p-4">
            <AuditTimeline
              events={AUDIT_EVENTS}
              expandedId={expanded}
              onExpand={(id) => setExpanded(id === expanded ? undefined : id)}
            />
          </Card>

          <Card className="p-4 mt-4 bg-muted/30">
            <div className="text-xs space-y-1">
              <div className="font-semibold text-foreground">合规说明</div>
              <ul className="text-muted-foreground space-y-0.5 mt-1">
                <li>· 报表生成、下载、导出均写 H2 审计</li>
                <li>· 含数据签名（MD5+时间戳）防篡改</li>
                <li>· 留存 5 年（GSP §95）</li>
                <li>· 监管报送通过 ERP 走药监 EDI</li>
              </ul>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
