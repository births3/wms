import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import {
  PageHeader,
  RuleEditor,
  StatusBadge,
  type RuleGroup,
  type RuleAction,
} from "@/components/business";
import { Plus, PlayCircle, History, FileCheck } from "lucide-react";

/**
 * M8StorageFeeRules — M8-001 仓储费规则配置
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M8-001（3PL 仓储计费规则 / 多档阶梯 / 测试运行）
 * Wave：Wave 4.0（M8 计费）
 * 业务约束：规则修改有版本号 + 审批 + 生效日期；冷链费率独立；测试运行不写库
 *
 * @example
 *   <M8StorageFeeRules />
 */

const RULE_GROUPS: RuleGroup[] = [
  {
    connector: "AND",
    conditions: [
      { field: "storage_zone", op: "in", value: "RT,CL" },
      { field: "duration_days", op: "lte", value: "30" },
    ],
  },
];

const RULE_ACTIONS: RuleAction[] = [
  { type: "set_rate", label: "费率 = 0.15 元/件/天", params: { unit: "元/件/天", value: "0.15" } },
  { type: "set_min_charge", label: "最低收费 5 元", params: { 单位: "元", value: "5.00" } },
];

const COLD_GROUPS: RuleGroup[] = [
  {
    connector: "AND",
    conditions: [
      { field: "storage_zone", op: "in", value: "CR,FR" },
      { field: "duration_days", op: "lte", value: "30" },
    ],
  },
];

const COLD_ACTIONS: RuleAction[] = [
  { type: "set_rate", label: "费率 = 0.45 元/件/天（冷链溢价）", params: { unit: "元/件/天", value: "0.45" } },
  { type: "extra", label: "+ 温度监控费 50 元/月/库位" },
];

const HISTORY = [
  { ver: "v3", date: "2026-04-01", actor: "张三 (u001)", desc: "冷链费率 0.40 → 0.45", status: "active" },
  { ver: "v2", date: "2026-01-01", actor: "李四 (u002)", desc: "RT/CL 费率统一", status: "archived" },
  { ver: "v1", date: "2025-09-01", actor: "王五 (u003)", desc: "初版", status: "archived" },
];

export function M8StorageFeeRules() {
  const [activeTab, setActiveTab] = useState<"normal" | "cold">("normal");

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="仓储费规则配置"
        subtitle="M8-001 · 3PL 计费 · 当前版本 v3 · 生效 2026-04-01"
        actions={
          <>
            <Button variant="outline" size="sm">
              <PlayCircle className="h-4 w-4 mr-1" /> 测试运行
            </Button>
            <Button variant="outline" size="sm">
              <History className="h-4 w-4 mr-1" /> 历史版本
            </Button>
            <Button size="sm">
              <Plus className="h-4 w-4 mr-1" /> 新增规则
            </Button>
          </>
        }
      />

      {/* 规则切换 tab */}
      <div className="px-6 py-3 border-b flex items-center gap-2">
        <Button
          variant={activeTab === "normal" ? "default" : "outline"}
          size="sm"
          onClick={() => setActiveTab("normal")}
        >
          常温/阴凉规则
        </Button>
        <Button
          variant={activeTab === "cold" ? "default" : "outline"}
          size="sm"
          onClick={() => setActiveTab("cold")}
        >
          ❄️ 冷链规则
        </Button>
        <span className="ml-auto text-xs text-muted-foreground">
          影响 <span className="font-semibold">86 家货主</span> · 月计费总额 <span className="font-semibold">¥1.86M</span>
        </span>
      </div>

      <div className="px-6 py-4 grid grid-cols-[1fr_320px] gap-4">
        {/* 左侧：规则编辑器 */}
        <div>
          <Card className="p-4">
            <div className="flex items-center justify-between mb-3">
              <div className="text-sm font-semibold">
                {activeTab === "normal" ? "常温/阴凉计费规则" : "❄️ 冷链计费规则"}
              </div>
              <StatusBadge status="qualified" size="sm" label="生效中" />
            </div>
            <RuleEditor
              groups={activeTab === "normal" ? RULE_GROUPS : COLD_GROUPS}
              actions={activeTab === "normal" ? RULE_ACTIONS : COLD_ACTIONS}
              fields={["storage_zone", "duration_days", "owner_id", "item_category", "qty"]}
              readOnly
            />
          </Card>

          {/* 阶梯费率展示 */}
          <Card className="p-4 mt-4">
            <div className="text-sm font-semibold mb-3">阶梯费率（按时长）</div>
            <table className="w-full text-sm">
              <thead className="text-xs text-muted-foreground border-b">
                <tr>
                  <th className="text-left pb-2">时长档</th>
                  <th className="text-right pb-2">常温/阴凉</th>
                  <th className="text-right pb-2">❄️ 冷藏</th>
                  <th className="text-right pb-2">❄️ 冷冻</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b">
                  <td className="py-2">≤ 30 天</td>
                  <td className="text-right font-mono">¥0.15</td>
                  <td className="text-right font-mono text-wms-cold">¥0.45</td>
                  <td className="text-right font-mono text-wms-cold font-semibold">¥0.62</td>
                </tr>
                <tr className="border-b">
                  <td className="py-2">31-90 天</td>
                  <td className="text-right font-mono">¥0.18</td>
                  <td className="text-right font-mono text-wms-cold">¥0.52</td>
                  <td className="text-right font-mono text-wms-cold font-semibold">¥0.71</td>
                </tr>
                <tr className="border-b">
                  <td className="py-2">91-180 天</td>
                  <td className="text-right font-mono">¥0.22</td>
                  <td className="text-right font-mono text-wms-cold">¥0.62</td>
                  <td className="text-right font-mono text-wms-cold font-semibold">¥0.85</td>
                </tr>
                <tr>
                  <td className="py-2">&gt; 180 天</td>
                  <td className="text-right font-mono">¥0.28</td>
                  <td className="text-right font-mono text-wms-cold">¥0.78</td>
                  <td className="text-right font-mono text-wms-cold font-semibold">¥1.05</td>
                </tr>
              </tbody>
            </table>
            <div className="text-xs text-muted-foreground mt-3">单位：元/件/天 · 不足 1 天按 1 天计 · 冷链含温控费</div>
          </Card>
        </div>

        {/* 右侧：历史版本 + 测试 */}
        <div className="space-y-4">
          <Card className="p-4">
            <div className="text-sm font-semibold mb-3 flex items-center gap-2">
              <FileCheck className="h-4 w-4" /> 测试运行
            </div>
            <div className="text-xs text-muted-foreground mb-3">用 2026-04 数据预演</div>
            <div className="space-y-2 text-xs">
              <div className="flex justify-between"><span className="text-muted-foreground">总计费货主</span><span>86</span></div>
              <div className="flex justify-between"><span className="text-muted-foreground">常温计费</span><span>¥1.42M</span></div>
              <div className="flex justify-between"><span className="text-muted-foreground">冷链计费</span><span className="text-wms-cold">¥0.44M</span></div>
              <div className="flex justify-between border-t pt-2 mt-2 font-semibold">
                <span>合计</span><span>¥1.86M</span>
              </div>
            </div>
            <Button size="sm" className="w-full mt-3">运行测试</Button>
          </Card>

          <Card className="p-4">
            <div className="text-sm font-semibold mb-3">版本历史</div>
            <div className="space-y-2.5 text-xs">
              {HISTORY.map((h) => (
                <div key={h.ver} className={`p-2 rounded border ${
                  h.status === "active" ? "border-primary/40 bg-primary/5" : ""
                }`}>
                  <div className="flex items-center justify-between mb-1">
                    <span className="font-mono font-semibold">{h.ver}</span>
                    {h.status === "active" ? (
                      <span className="text-[10px] px-1 py-0.5 bg-primary text-primary-foreground rounded">当前</span>
                    ) : (
                      <span className="text-[10px] px-1 py-0.5 bg-muted text-muted-foreground rounded">归档</span>
                    )}
                  </div>
                  <div className="text-muted-foreground">{h.date} · {h.actor}</div>
                  <div className="mt-0.5">{h.desc}</div>
                </div>
              ))}
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
