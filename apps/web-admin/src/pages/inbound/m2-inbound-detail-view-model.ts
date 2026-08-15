import type {
  ReceivingOrder,
  ReceivingOrderReceipt,
} from "../../features/inbound/inbound-queries.ts";
import { maskSensitiveDisplayValue } from "../../lib/mask-sensitive.ts";
import {
  COLUMN_BATCH_NO,
  COLUMN_PRODUCT_CODE,
  COLUMN_QUALITY_STATUS,
  FIELD_VALIDITY,
  STATUS_COMPLETED,
  STATUS_PENDING,
  STATUS_PENDING_INPUT,
} from "../../lib/ui-strings.ts";

export type InboundDetailStage = "receiving" | "inspection" | "putaway" | "completed";
export type InboundDetailFieldSection = "product" | "order" | "batch" | "process";

export const inboundDetailFieldSections: Record<InboundDetailFieldSection, { title: string }> = {
  product: { title: "商品信息" },
  order: { title: "订单信息" },
  batch: { title: "批号信息" },
  process: { title: "流程信息" },
};

export type ProductInfoFieldKey =
  | "productCode"
  | "productName"
  | "specification"
  | "manufacturer"
  | "orderQty"
  | "unit"
  | "caseQty"
  | "looseQty"
  | "middlePackQty"
  | "casePackQty";

export const productInfoFieldDefinitions: Array<{ key: ProductInfoFieldKey; label: string; align?: "left" | "right" }> = [
  { key: "productCode", label: COLUMN_PRODUCT_CODE },
  { key: "productName", label: "品名" },
  { key: "specification", label: "规格" },
  { key: "manufacturer", label: "生产厂家" },
  { key: "orderQty", label: "订单数量", align: "right" },
  { key: "unit", label: "单位" },
  { key: "caseQty", label: "件数", align: "right" },
  { key: "looseQty", label: "零数", align: "right" },
  { key: "middlePackQty", label: "中包数量" },
  { key: "casePackQty", label: "件包数量" },
];

export type BatchInfoFieldKey =
  | "lineNo"
  | "batchNo"
  | "approvalNo"
  | "importRegistrationCertificate"
  | "marketingAuthorizationHolder"
  | "batchQty"
  | "batchCasePackage"
  | "productionDate"
  | "expiryDate";

export const batchInfoFieldDefinitions: Array<{ key: BatchInfoFieldKey; label: string; align?: "left" | "right" }> = [
  { key: "lineNo", label: "行号" },
  { key: "batchNo", label: COLUMN_BATCH_NO },
  { key: "approvalNo", label: "批准文号" },
  { key: "importRegistrationCertificate", label: "进口注册证" },
  { key: "marketingAuthorizationHolder", label: "上市持有人" },
  { key: "batchQty", label: "批号数量", align: "right" },
  { key: "batchCasePackage", label: "批号件包装" },
  { key: "productionDate", label: "生产日期" },
  { key: "expiryDate", label: FIELD_VALIDITY },
];

export const inboundDetailStages: Array<{ label: string; stage: InboundDetailStage; index: number }> = [
  { label: "收货", stage: "receiving", index: 0 },
  { label: "验收", stage: "inspection", index: 1 },
  { label: "上架", stage: "putaway", index: 2 },
  { label: "完成", stage: "completed", index: 3 },
];

export interface ProcessState {
  label: string;
  status: "completed" | "in_progress" | "pending";
}

export function inboundDetailStageIndex(status: string | null | undefined) {
  if (!status) return 0;
  if (status === "completed") return 3;
  if (status.includes("putaway")) return 2;
  if (status.includes("inspect")) return 1;
  return 0;
}

export function processDetail(
  stage: InboundDetailStage,
  expectedQty: number,
  currentStage: number,
  receipt?: ReceivingOrderReceipt | null,
) {
  const map = {
    receiving: {
      title: "收货信息",
      state: processState(0, currentStage),
      rows: receivingRows(expectedQty, receipt),
    },
    inspection: {
      title: "验收信息",
      state: processState(1, currentStage),
      rows: [
        ["通过 / 拒收", STATUS_PENDING_INPUT],
        ["追溯码", STATUS_PENDING_INPUT],
        [COLUMN_QUALITY_STATUS, STATUS_PENDING_INPUT],
        ["四项核对", STATUS_PENDING_INPUT],
        ["验收复核", STATUS_PENDING_INPUT],
        ["验收 / 复核人", STATUS_PENDING_INPUT],
        ["验收备注", "-"],
      ],
    },
    putaway: {
      title: "上架信息",
      state: processState(2, currentStage),
      rows: [
        ["容器 LPN", STATUS_PENDING_INPUT],
        ["推荐库位", "-"],
        ["实际库位", STATUS_PENDING_INPUT],
        ["校验结果", "待执行"],
        ["上架备注", "-"],
      ],
    },
    completed: {
      title: "完成信息",
      state: processState(3, currentStage),
      rows: [["完成状态", currentStage >= 3 ? STATUS_COMPLETED : "未完成"]],
    },
  } satisfies Record<InboundDetailStage, { title: string; state: ProcessState; rows: Array<[string, string]> }>;
  return map[stage];
}

function receivingRows(expectedQty: number, receipt?: ReceivingOrderReceipt | null): Array<[string, string]> {
  const details = receipt?.details;
  const batches = details?.sales_return_batches ?? [];
  const rejectedBatches = batches.filter((batch) => Number(batch.rejected_qty) > 0);
  return [
    ["发运地点", fieldValue(details?.origin)],
    ["车牌号", fieldValue(details?.vehicle_no)],
    ["启运时间", dateTimeValue(details?.departure_at)],
    ["到货时间", dateTimeValue(details?.arrival_at)],
    ["收货入库时间", dateTimeValue(details?.storage_at)],
    ["运输方式", fieldValue(details?.transport_mode)],
    ["承运商", fieldValue(details?.carrier)],
    ["联系人（送货人）", fieldValue(details?.contact_name)],
    ["电话", maskSensitiveDisplayValue(details?.contact_phone) ?? STATUS_PENDING_INPUT],
    ["身份证", maskSensitiveDisplayValue(details?.contact_id_no) ?? STATUS_PENDING_INPUT],
    ["印章样式核对", fieldValue(details?.seal_checked)],
    ["备案件样式核对", fieldValue(details?.filing_checked)],
    ["预报数量", `${expectedQty} 件`],
    ["送货数量", quantityValue(details?.delivery_qty)],
    ["实际到货数量", quantityValue(receipt?.actual_qty)],
    ["缺货数量", quantityValue(receipt?.shortage_qty)],
    ["拒收数量", quantityValue(receipt?.rejected_qty)],
    ["拒收备注", fieldValue(receipt?.exception_note, "-")],
    [
      "销售退货批号 + 数量",
      batches.length > 0
        ? batches.map((batch) => `${batch.batch_no} × ${batch.quantity} 件`).join("；")
        : "-",
    ],
    [
      "销售退货批号级拒收明细",
      rejectedBatches.length > 0
        ? rejectedBatches
            .map((batch) => `${batch.batch_no}：拒收 ${batch.rejected_qty} 件（${batch.reject_reason || "未填写原因"}）`)
            .join("；")
        : "-",
    ],
    ["第二收货员验证", fieldValue(details?.second_receiver_id)],
    [
      "到货温度",
      receipt?.arrival_temperature_celsius == null
        ? "-"
        : `${receipt.arrival_temperature_celsius} °C`,
    ],
    ["温控方式", fieldValue(details?.temperature_control_method)],
  ];
}

function fieldValue(value: string | null | undefined, fallback = STATUS_PENDING_INPUT) {
  return value?.trim() || fallback;
}

function quantityValue(value: string | null | undefined) {
  return value == null ? STATUS_PENDING_INPUT : `${value} 件`;
}

function dateTimeValue(value: string | null | undefined) {
  if (!value) return STATUS_PENDING_INPUT;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
    timeZone: "Asia/Shanghai",
  }).format(date);
}

export function productInfoRows(order: Pick<ReceivingOrder, "lines">) {
  const rows = new Map<string, number>();
  for (const item of order.lines ?? []) {
    rows.set(item.product_code, (rows.get(item.product_code) ?? 0) + Number(item.expected_qty));
  }
  return [...rows.entries()].map(([productCode, orderQty]) => ({
    ...productMasterFields(),
    ...linePackageFields(),
    key: productCode,
    productCode: productCode || "-",
    orderQty: String(orderQty),
  }));
}

export function batchInfoRows(order: Pick<ReceivingOrder, "lines">) {
  return (order.lines ?? []).map((item) => ({
    ...batchLicenseFields(),
    key: `${item.line_no}-${item.batch_no ?? ""}`,
    lineNo: `#${item.line_no}`,
    batchNo: item.batch_no || "-",
    batchQty: String(item.expected_qty),
    batchCasePackage: "-",
    productionDate: item.production_date || "-",
    expiryDate: item.expiry_date || "-",
  }));
}

export function orderLicenseRows(order: Pick<ReceivingOrder, "lines">): Array<[string, string]> {
  const rows = (order.lines ?? []).map(() => batchLicenseFields());
  return [
    ["批准文号", uniqueText(rows.map((item) => item.approvalNo))],
    ["进口注册证", uniqueText(rows.map((item) => item.importRegistrationCertificate))],
    ["上市持有人", uniqueText(rows.map((item) => item.marketingAuthorizationHolder))],
  ];
}

function processState(index: number, current: number): ProcessState {
  if (index < current) return { label: STATUS_COMPLETED, status: "completed" };
  if (index === current) return { label: "当前", status: "in_progress" };
  return { label: STATUS_PENDING, status: "pending" };
}

function linePackageFields() {
  // ponytail: ReceivingOrderLine 还没有产品包装主数据；缺数据展示「-」，后端补单位/中包/件包字段后替换这里。
  return {
    unit: "-",
    caseQty: "-",
    looseQty: "-",
    middlePackQty: "-",
    casePackQty: "-",
  };
}

function productMasterFields() {
  // ponytail: ReceivingOrderLine 还没带商品档案字段；缺数据展示「-」，后端关联 M1 商品档案后替换这里。
  return {
    productName: "-",
    specification: "-",
    manufacturer: "-",
  };
}

function batchLicenseFields() {
  // ponytail: ReceivingOrderLine 还没带批号资质字段；缺数据展示「-」，后端关联批号/药监档案后替换这里。
  return {
    approvalNo: "-",
    importRegistrationCertificate: "-",
    marketingAuthorizationHolder: "-",
  };
}

function uniqueText(values: string[]) {
  const unique = [...new Set(values.filter(Boolean))];
  if (unique.length === 0) return "-";
  if (unique.length <= 2) return unique.join(" / ");
  return `${unique[0]} 等 ${unique.length} 项`;
}
