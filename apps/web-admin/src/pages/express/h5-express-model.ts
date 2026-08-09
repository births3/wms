/** H5 快递对接页面与弹窗共用的字典（独立小模块，避免页面 ↔ 弹窗互相引用形成循环依赖）。 */

import { STATUS_CANCELLED } from "@/lib/ui-strings";

export const providerOptions = [
  { label: "自有配送", value: "own_fleet" },
  { label: "三方快递", value: "third_party_express" },
];

export function providerLabel(value: string) {
  return value === "own_fleet" ? "自有配送" : "三方快递";
}

export function waybillStatusLabel(status: string) {
  if (status === "created" || status === "pushed") return "已下单";
  if (status === "printed") return "已打印";
  if (status === "in_transit") return "运输中";
  if (status === "delivered") return "已签收";
  if (status === "cancelled") return STATUS_CANCELLED;
  return status;
}

export function waybillStatusKey(status: string): "completed" | "pending" | "isolated" | "in_progress" {
  if (status === "delivered" || status === "printed") return "completed";
  if (status === "cancelled") return "isolated";
  if (status === "in_transit") return "in_progress";
  return "pending";
}
