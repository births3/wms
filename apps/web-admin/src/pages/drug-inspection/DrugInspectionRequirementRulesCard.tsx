import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Checkbox,
  StatusBadge,
} from "@wms/ui";

import {
  useDrugInspectionRequirementRulesQuery,
  useUpsertDrugInspectionRequirementRuleMutation,
} from "@/features/drug-inspection/document-queries";
import { useSpecialDrugCategoriesQuery } from "@/features/master-data/master-data-queries";

export function DrugInspectionRequirementRulesCard() {
  const rules = useDrugInspectionRequirementRulesQuery();
  const categories = useSpecialDrugCategoriesQuery();
  const save = useUpsertDrugInspectionRequirementRuleMutation();
  const [category, setCategory] = React.useState("*");
  const [behavior, setBehavior] = React.useState<"warning" | "block">("block");
  const [enabled, setEnabled] = React.useState(true);
  const [notice, setNotice] = React.useState("");

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    setNotice("");
    try {
      const saved = await save.mutateAsync({
        specialDrugCategory: category,
        missingBehavior: behavior,
        enabled,
      });
      setNotice(`规则已保存为 v${saved.version}`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "保存规则失败");
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>入库验收药检要求规则</CardTitle>
        <p className="text-sm text-muted-foreground">
          按商品特殊药品类别决定缺少当前已确认药检单时警告或阻塞；每次保存形成新规则版本。
        </p>
      </CardHeader>
      <CardContent className="grid gap-5">
        <form className="grid gap-3 md:grid-cols-[1fr_180px_auto_auto]" onSubmit={submit}>
          <label className="grid gap-1 text-sm">
            <span>商品类别</span>
            <select
              className="h-9 rounded-md border border-input bg-background px-3 text-sm"
              aria-label="药检要求商品类别"
              value={category}
              onChange={(event) => setCategory(event.target.value)}
            >
              <option value="*">全部类别（默认规则）</option>
              {(categories.data ?? []).map((option) => (
                <option key={option.value} value={option.value}>{option.label}</option>
              ))}
            </select>
          </label>
          <label className="grid gap-1 text-sm">
            <span>缺失处理</span>
            <select
              className="h-9 rounded-md border border-input bg-background px-3 text-sm"
              aria-label="药检单缺失处理"
              value={behavior}
              onChange={(event) => setBehavior(event.target.value as "warning" | "block")}
            >
              <option value="block">阻塞验收</option>
              <option value="warning">警告后继续</option>
            </select>
          </label>
          <label className="flex items-end gap-2 pb-2 text-sm">
            <Checkbox
              checked={enabled}
              onCheckedChange={(value) => setEnabled(value === true)}
            />
            启用
          </label>
          <Button className="self-end" type="submit" disabled={save.isPending}>
            {save.isPending ? "保存中..." : "保存规则"}
          </Button>
        </form>
        {notice && (
          <p role={save.isError ? "alert" : "status"} className="text-sm">
            {notice}
          </p>
        )}
        <div className="overflow-x-auto">
          <table className="w-full min-w-[720px] text-sm">
            <thead>
              <tr className="border-b text-left text-muted-foreground">
                <th className="px-3 py-2">商品类别</th>
                <th className="px-3 py-2">缺失处理</th>
                <th className="px-3 py-2">状态</th>
                <th className="px-3 py-2">规则版本</th>
                <th className="px-3 py-2">更新时间</th>
              </tr>
            </thead>
            <tbody>
              {(rules.data ?? []).map((rule) => (
                <tr key={rule.id} className="border-b">
                  <td className="px-3 py-2">{rule.special_drug_category === "*" ? "全部类别" : rule.special_drug_category}</td>
                  <td className="px-3 py-2">{rule.missing_behavior === "block" ? "阻塞验收" : "警告后继续"}</td>
                  <td className="px-3 py-2">
                    <StatusBadge
                      status={rule.enabled ? "completed" : "isolated"}
                      label={rule.enabled ? "启用" : "停用"}
                      size="sm"
                    />
                  </td>
                  <td className="px-3 py-2">v{rule.version}</td>
                  <td className="px-3 py-2">{formatTime(rule.updated_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {!rules.isPending && !rules.data?.length && (
            <p className="py-5 text-center text-sm text-muted-foreground">
              尚未配置规则；没有命中规则的商品类别不强制药检单。
            </p>
          )}
          {rules.error && (
            <p role="alert" className="py-3 text-sm text-destructive">{rules.error.message}</p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

function formatTime(value: string) {
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "Asia/Shanghai",
  }).format(new Date(value));
}
