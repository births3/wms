import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Input,
  Label,
  PageHeader,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@wms/ui";
import { AlertCircle, CheckCircle2, Loader2, Route, Send } from "lucide-react";

import {
  useReceiveTmsRoutePlanMutation,
  type ReceiveTmsRoutePlanRequest,
  type TmsRoutePlan,
} from "@/features/tms/tms-route-plan-queries";
import { COLUMN_STATUS, FIELD_PLATE_NO } from "@/lib/ui-strings";

type FormState = {
  deliveryDate: string;
  dispatchResultId: string;
  outboundOrderIds: string;
  driverUserId: string;
  vehicleNo: string;
  plateNo: string;
  version: string;
  stopsJson: string;
};

const emptyForm: FormState = {
  deliveryDate: "",
  dispatchResultId: "",
  outboundOrderIds: "",
  driverUserId: "",
  vehicleNo: "",
  plateNo: "",
  version: "1",
  stopsJson: "",
};

export function TmsRoutePlanPage() {
  const [form, setForm] = React.useState<FormState>(emptyForm);
  const [validationError, setValidationError] = React.useState<string | null>(null);
  const [receivedPlan, setReceivedPlan] = React.useState<TmsRoutePlan | null>(null);
  const mutation = useReceiveTmsRoutePlanMutation();

  function update(key: keyof FormState, value: string) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    mutation.reset();
    setValidationError(null);
    setReceivedPlan(null);
    try {
      const request = toRequest(form);
      mutation.mutate(request, { onSuccess: setReceivedPlan });
    } catch (error) {
      setValidationError(error instanceof Error ? error.message : "请检查输入内容");
    }
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader />
      {(validationError || mutation.error) && (
        <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
          <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
          <div>
            <p>{validationError ?? mutation.error?.message}</p>
            {mutation.error && <p className="mt-1 text-xs">HTTP {mutation.error.status} · {mutation.error.code}</p>}
          </div>
        </div>
      )}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2"><Route className="size-4" aria-hidden="true" />路线结果</CardTitle>
          <CardDescription>订单 ID 每行一个；站点 JSON 必须符合 TMS 接收契约。</CardDescription>
        </CardHeader>
        <CardContent>
          <form className="grid gap-5" onSubmit={handleSubmit} noValidate>
            <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
              <Field id="deliveryDate" label="配送日期" type="date" value={form.deliveryDate} onChange={(value) => update("deliveryDate", value)} />
              <Field id="dispatchResultId" label="TMS 调度结果 ID" value={form.dispatchResultId} onChange={(value) => update("dispatchResultId", value)} />
              <Field id="driverUserId" label="司机 user_id" value={form.driverUserId} onChange={(value) => update("driverUserId", value)} />
              <Field id="version" label="规划版本" type="number" value={form.version} onChange={(value) => update("version", value)} />
              <Field id="vehicleNo" label="车辆编号" value={form.vehicleNo} onChange={(value) => update("vehicleNo", value)} />
              <Field id="plateNo" label={FIELD_PLATE_NO} value={form.plateNo} onChange={(value) => update("plateNo", value)} />
            </div>
            <div className="grid gap-4 lg:grid-cols-2">
              <div className="grid gap-1.5">
                <Label htmlFor="outboundOrderIds">出库订单 ID</Label>
                <textarea id="outboundOrderIds" required value={form.outboundOrderIds} onChange={(event) => update("outboundOrderIds", event.target.value)} placeholder="每行一个订单 ID" className="min-h-28 rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" />
              </div>
              <div className="grid gap-1.5">
                <Label htmlFor="stopsJson">路线站点 JSON</Label>
                <textarea id="stopsJson" required value={form.stopsJson} onChange={(event) => update("stopsJson", event.target.value)} placeholder={'[{"sequence":1,"store_id":"...","estimated_arrival_at":"2026-07-14T09:00:00Z","outbound_order_ids":["..."]}]'} className="min-h-28 rounded-md border border-input bg-background px-3 py-2 font-mono text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring" />
              </div>
            </div>
            <div className="flex justify-end">
              <Button type="submit" disabled={mutation.isPending}>
                {mutation.isPending ? <Loader2 className="size-4 animate-spin" aria-hidden="true" /> : <Send className="size-4" aria-hidden="true" />}
                {mutation.isPending ? "接收中..." : "接收路线结果"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
      {receivedPlan && <ReceivedPlan plan={receivedPlan} />}
    </section>
  );
}

function Field({ id, label, value, onChange, type = "text" }: { id: string; label: string; value: string; onChange: (value: string) => void; type?: "text" | "date" | "number" }) {
  return <div className="grid gap-1.5"><Label htmlFor={id}>{label}</Label><Input id={id} required type={type} value={value} onChange={(event) => onChange(event.target.value)} /></div>;
}

function ReceivedPlan({ plan }: { plan: TmsRoutePlan }) {
  return (
    <Card className="border-emerald-600/30">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-emerald-700"><CheckCircle2 className="size-4" aria-hidden="true" />路线结果已接收</CardTitle>
        <CardDescription role="status">已保存返回的路线信息，可供后续页面接入展示。</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5">
        <dl className="grid gap-4 text-sm sm:grid-cols-2 lg:grid-cols-4">
          <div><dt className="text-muted-foreground">路线 ID</dt><dd className="mt-1 break-all font-mono">{plan.id}</dd></div>
          <div><dt className="text-muted-foreground">调度结果 ID</dt><dd className="mt-1 break-all font-mono">{plan.dispatch_result_id}</dd></div>
          <div><dt className="text-muted-foreground">规划版本</dt><dd className="mt-1">{plan.version}</dd></div>
          <div><dt className="text-muted-foreground">{COLUMN_STATUS}</dt><dd className="mt-1">{plan.status}</dd></div>
          <div><dt className="text-muted-foreground">配送日期</dt><dd className="mt-1">{plan.delivery_date}</dd></div>
          <div><dt className="text-muted-foreground">司机 user_id</dt><dd className="mt-1 break-all font-mono">{plan.driver_user_id}</dd></div>
          <div><dt className="text-muted-foreground">车辆 / 车牌</dt><dd className="mt-1">{plan.vehicle_no} / {plan.plate_no}</dd></div>
          <div><dt className="text-muted-foreground">出库订单</dt><dd className="mt-1 break-all">{plan.outbound_order_ids.join(", ")}</dd></div>
        </dl>
        <div>
          <h2 className="mb-2 text-sm font-semibold">路线站点（{plan.stops.length}）</h2>
          <Table>
            <TableHeader><TableRow><TableHead>顺序</TableHead><TableHead>门店 ID</TableHead><TableHead>预计到达</TableHead><TableHead>关联订单</TableHead></TableRow></TableHeader>
            <TableBody>{plan.stops.map((stop) => <TableRow key={stop.id}><TableCell>{stop.sequence}</TableCell><TableCell className="font-mono">{stop.store_id}</TableCell><TableCell>{stop.estimated_arrival_at}</TableCell><TableCell>{stop.outbound_order_ids.join(", ")}</TableCell></TableRow>)}</TableBody>
          </Table>
        </div>
      </CardContent>
    </Card>
  );
}

function toRequest(form: FormState): ReceiveTmsRoutePlanRequest {
  const outbound_order_ids = splitList(form.outboundOrderIds);
  if (!outbound_order_ids.length) throw new Error("至少填写一个出库订单 ID");
  const version = Number(form.version);
  if (!Number.isInteger(version) || version < 1) throw new Error("规划版本必须是正整数");
  let parsed: unknown;
  try {
    parsed = JSON.parse(form.stopsJson) as unknown;
  } catch {
    throw new Error("路线站点 JSON 格式错误");
  }
  if (!Array.isArray(parsed) || !parsed.length || !parsed.every(isRouteStop)) throw new Error("路线站点 JSON 必须是非空站点数组");
  return { delivery_date: form.deliveryDate, dispatch_result_id: form.dispatchResultId.trim(), driver_user_id: form.driverUserId.trim(), outbound_order_ids, plate_no: form.plateNo.trim(), stops: parsed, vehicle_no: form.vehicleNo.trim(), version };
}

function splitList(value: string) {
  return value.split(/[\n,]/).map((item) => item.trim()).filter(Boolean);
}

function isRouteStop(value: unknown): value is ReceiveTmsRoutePlanRequest["stops"][number] {
  if (typeof value !== "object" || value === null) return false;
  const stop = value as { estimated_arrival_at?: unknown; outbound_order_ids?: unknown; sequence?: unknown; store_id?: unknown };
  return typeof stop.estimated_arrival_at === "string" && Number.isInteger(stop.sequence) && typeof stop.store_id === "string" && Array.isArray(stop.outbound_order_ids) && stop.outbound_order_ids.every((id: unknown) => typeof id === "string");
}
