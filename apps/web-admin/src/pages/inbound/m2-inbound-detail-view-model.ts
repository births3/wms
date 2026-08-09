import type { ReceivingOrder } from "@/features/inbound/inbound-queries";
import {
  COLUMN_BATCH_NO,
  COLUMN_PRODUCT_CODE,
  COLUMN_QUALITY_STATUS,
  FIELD_VALIDITY,
  STATUS_COMPLETED,
  STATUS_PENDING,
  STATUS_PENDING_INPUT,
} from "@/lib/ui-strings";

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

export function processDetail(stage: InboundDetailStage, expectedQty: number, currentStage: number) {
  const map = {
    // ponytail: 列表/详情接口尚未返回收货、验收、上架回执，缺数据一律「待录入 / -」，不虚构现场记录。
    receiving: {
      title: "收货信息",
      state: processState(0, currentStage),
      rows: [
        ["承运商 / 车牌", STATUS_PENDING_INPUT],
        ["发运地点", STATUS_PENDING_INPUT],
        ["启运 / 到货", STATUS_PENDING_INPUT],
        ["入库时间", STATUS_PENDING_INPUT],
        ["运输 / 温控 / 温度", STATUS_PENDING_INPUT],
        ["联系人", STATUS_PENDING_INPUT],
        ["随货核对", STATUS_PENDING_INPUT],
        ["数量闭合", `预报 ${expectedQty} 件 / 实收待录入`],
        ["第一收货员", STATUS_PENDING_INPUT],
        ["第二收货员", STATUS_PENDING_INPUT],
        ["异常备注", "-"],
      ],
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
