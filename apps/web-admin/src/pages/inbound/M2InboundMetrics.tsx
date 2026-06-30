/**
 * M2InboundMetrics — 入库作业指标卡
 *
 * 层级：Layer 3 页面局部组件
 * 关联故事：US-M2-008
 * Wave：Wave 6
 */

import { Card, CardContent } from "@wms/ui";

import type { ReceivingOrder } from "@/features/inbound/inbound-queries";
import { countByStatus } from "./m2-inbound-page-helpers";

interface M2InboundMetricsProps {
  orders: ReceivingOrder[];
}

export function M2InboundMetrics({ orders }: M2InboundMetricsProps) {
  return (
    <div className="grid gap-3 md:grid-cols-4">
      <Metric label="待处理" value={countByStatus(orders, "receiving")} tone="primary" />
      <Metric label="验收中" value={countByStatus(orders, "inspecting")} tone="warning" />
      <Metric label="上架中" value={countByStatus(orders, "putaway")} tone="success" />
      <Metric label="本页合计" value={orders.length} tone="muted" />
    </div>
  );
}

function Metric({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone: "primary" | "warning" | "success" | "muted";
}) {
  const toneClass = {
    primary: "text-primary",
    warning: "text-wms-warning",
    success: "text-wms-success",
    muted: "text-foreground",
  }[tone];
  return (
    <Card className="rounded-lg shadow-sm">
      <CardContent className="p-4">
        <p className="text-xs font-medium text-muted-foreground">{label}</p>
        <p className={`mt-2 text-2xl font-semibold tracking-normal ${toneClass}`}>{value}</p>
      </CardContent>
    </Card>
  );
}
