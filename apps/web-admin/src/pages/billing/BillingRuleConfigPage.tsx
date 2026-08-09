import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  DataGrid,
  Input,
  PageHeader,
  type DataGridColumn,
} from "@wms/ui";

import {
  useCreateBillingRuleMutation,
  type BillingRule,
  type CreateBillingRuleRequest,
} from "@/features/billing/billing-rule-queries";

/**
 * BillingRuleConfigPage — 配置计费规则并展示本次创建结果
 *
 * 层级：Layer 3 页面
 * 关联故事：US-M9-001
 * Wave：Wave 5 M9
 * 页面设计契约：配置型；主信息载体为规则表单与本次创建列表；标准动作入口为保存；无私有动作；详情通过 DataGrid 展示；无禁止常驻区域。
 * @example
 *   <BillingRuleConfigPage />
 */

type FormState = Omit<CreateBillingRuleRequest, "unit_price_cents"> & { unit_price_cents: string };
type Notice = { kind: "success" | "error"; text: string } | null;

const chargeItemOptions = [
  ["storage", "仓储费"],
  ["inbound_operation", "入库作业费"],
  ["outbound_operation", "出库作业费"],
  ["consumable", "耗材费"],
  ["loading_unloading", "装卸费"],
] as const;
const chargeItemLabels: Record<string, string> = Object.fromEntries(chargeItemOptions);
const unitOptions = [
  ["square_meter_day", "平方米·日"],
  ["pallet_day", "托盘位·日"],
  ["order", "单"],
  ["line", "行"],
  ["piece", "件"],
  ["box", "箱"],
  ["job", "作业次"],
  ["hour", "小时"],
] as const;
const unitLabels: Record<string, string> = Object.fromEntries(unitOptions);
const billingCycleOptions = [
  ["daily", "按日"],
  ["weekly", "按周"],
  ["monthly", "按月"],
  ["quarterly", "按季"],
  ["one_off", "一次性"],
] as const;
const billingCycleLabels: Record<string, string> = Object.fromEntries(billingCycleOptions);
const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

const columns: DataGridColumn<BillingRule>[] = [
  { key: "charge_item", header: "计费项", width: 150, render: (row) => chargeItemLabels[row.charge_item] ?? row.charge_item },
  { key: "unit", header: "计量单位", width: 140, render: (row) => unitLabels[row.unit] ?? row.unit },
  { key: "billing_cycle", header: "计费周期", width: 110, render: (row) => billingCycleLabels[row.billing_cycle] ?? row.billing_cycle },
  { key: "unit_price_cents", header: "单价（分）", width: 110, render: (row) => String(row.unit_price_cents) },
  { key: "effective_from", header: "生效起日", width: 120 },
  { key: "effective_to", header: "生效止日", width: 120 },
  { key: "contract_id", header: "合同 ID", width: 290, mono: true, copyValue: (row) => row.contract_id },
  { key: "created_at", header: "创建时间", width: 175 },
];

export function BillingRuleConfigPage() {
  const createMutation = useCreateBillingRuleMutation();
  const [form, setForm] = React.useState<FormState>(emptyForm);
  const [rules, setRules] = React.useState<BillingRule[]>([]);
  const [notice, setNotice] = React.useState<Notice>(null);

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const validationError = validate(form);
    if (validationError) {
      setNotice({ kind: "error", text: validationError });
      return;
    }
    setNotice(null);
    const body: CreateBillingRuleRequest = {
      contract_id: form.contract_id.trim(),
      charge_item: form.charge_item,
      unit: form.unit,
      billing_cycle: form.billing_cycle,
      unit_price_cents: form.unit_price_cents,
      effective_from: form.effective_from,
      effective_to: form.effective_to,
    };
    try {
      const result = await createMutation.mutateAsync(body);
      setRules((current) => [result, ...current]);
      setForm(emptyForm());
      setNotice({ kind: "success", text: `计费规则 ${result.id} 已创建` });
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error, "保存计费规则失败") });
    }
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader title="M9 计费规则配置" subtitle="US-M9-001 · 按合同维护计费项、单价和生效窗口" />
      {notice && (
        <div
          className={notice.kind === "error" ? "rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" : "rounded-md border border-wms-success/30 bg-wms-success/10 px-3 py-2 text-sm text-wms-success"}
          role={notice.kind === "error" ? "alert" : "status"}
          aria-live={notice.kind === "error" ? "assertive" : "polite"}
        >
          {notice.text}
        </div>
      )}
      <div className="grid gap-5 xl:grid-cols-[minmax(22rem,28rem)_1fr]">
        <Card>
          <CardHeader>
            <CardTitle>新建计费规则</CardTitle>
            <CardDescription>保存后仅展示本次接口返回的规则；规则历史查询待真实 GET 契约提供后接入。</CardDescription>
          </CardHeader>
          <CardContent>
            <form className="grid gap-4" onSubmit={submit}>
              <fieldset className="grid gap-4" disabled={createMutation.isPending}>
                <label className="grid gap-1.5 text-sm" htmlFor="billing-charge-item">
                  <span className="font-medium">计费项</span>
                  <select id="billing-charge-item" name="charge_item" required className="h-9 rounded-md border border-input bg-background px-3 text-sm" value={form.charge_item} onChange={(event) => setForm((current) => ({ ...current, charge_item: event.target.value }))}>
                    {chargeItemOptions.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
                  </select>
                </label>
                <label className="grid gap-1.5 text-sm" htmlFor="billing-unit">
                  <span className="font-medium">计量单位</span>
                  <select id="billing-unit" name="unit" required className="h-9 rounded-md border border-input bg-background px-3 text-sm" value={form.unit} onChange={(event) => setForm((current) => ({ ...current, unit: event.target.value }))}>
                    {unitOptions.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
                  </select>
                </label>
                <label className="grid gap-1.5 text-sm" htmlFor="billing-cycle">
                  <span className="font-medium">计费周期</span>
                  <select id="billing-cycle" name="billing_cycle" required className="h-9 rounded-md border border-input bg-background px-3 text-sm" value={form.billing_cycle} onChange={(event) => setForm((current) => ({ ...current, billing_cycle: event.target.value }))}>
                    {billingCycleOptions.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
                  </select>
                </label>
                <label className="grid gap-1.5 text-sm" htmlFor="billing-price">
                  <span className="font-medium">单价（分）</span>
                  <Input id="billing-price" name="unit_price_cents" required type="number" min="0" step="1" inputMode="numeric" value={form.unit_price_cents} onChange={(event) => setForm((current) => ({ ...current, unit_price_cents: event.target.value }))} />
                </label>
                <div className="grid grid-cols-2 gap-3">
                  <label className="grid gap-1.5 text-sm" htmlFor="billing-effective-from">
                    <span className="font-medium">生效起日</span>
                    <Input id="billing-effective-from" name="effective_from" required type="date" value={form.effective_from} onChange={(event) => setForm((current) => ({ ...current, effective_from: event.target.value }))} />
                  </label>
                  <label className="grid gap-1.5 text-sm" htmlFor="billing-effective-to">
                    <span className="font-medium">生效止日</span>
                    <Input id="billing-effective-to" name="effective_to" required type="date" value={form.effective_to} onChange={(event) => setForm((current) => ({ ...current, effective_to: event.target.value }))} />
                  </label>
                </div>
                <label className="grid gap-1.5 text-sm" htmlFor="billing-contract-id">
                  <span className="font-medium">合同 ID</span>
                  <Input id="billing-contract-id" name="contract_id" required placeholder="UUID" autoComplete="off" value={form.contract_id} onChange={(event) => setForm((current) => ({ ...current, contract_id: event.target.value }))} />
                </label>
              </fieldset>
              <Button type="submit" disabled={createMutation.isPending}>{createMutation.isPending ? "保存中..." : "保存计费规则"}</Button>
            </form>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>本次创建结果</CardTitle>
            <CardDescription>{rules.length ? `已返回 ${rules.length} 条计费规则` : "暂无本次会话创建成功的规则"}</CardDescription>
          </CardHeader>
          <CardContent>
            <DataGrid storageKey="m9.billing-rules.created" columns={columns} data={rules} rowKey={(row) => row.id} emptyTitle="暂无计费规则" emptyDescription="提交成功后，返回规则会显示在这里" />
          </CardContent>
        </Card>
      </div>
    </section>
  );
}

function emptyForm(): FormState {
  return { contract_id: "", charge_item: "storage", unit: "pallet_day", billing_cycle: "monthly", unit_price_cents: "", effective_from: "", effective_to: "" };
}

function validate(form: FormState): string | null {
  if (!uuidPattern.test(form.contract_id.trim())) return "合同 ID 必须是有效 UUID";
  if (!form.unit_price_cents.trim() || !Number.isSafeInteger(Number(form.unit_price_cents)) || Number(form.unit_price_cents) < 0) return "单价必须是非负整数（分）";
  if (!form.effective_from || !form.effective_to) return "请选择生效起止日期";
  if (form.effective_to < form.effective_from) return "生效止日不能早于生效起日";
  return null;
}

function errorMessage(error: unknown, fallback: string) {
  return error instanceof Error && error.message ? error.message : fallback;
}
