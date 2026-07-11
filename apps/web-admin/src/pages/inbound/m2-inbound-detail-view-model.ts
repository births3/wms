import type { ReceivingOrder } from "@/features/inbound/inbound-queries";

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
  { key: "productCode", label: "商品编码" },
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
  { key: "batchNo", label: "批号" },
  { key: "approvalNo", label: "批准文号" },
  { key: "importRegistrationCertificate", label: "进口注册证" },
  { key: "marketingAuthorizationHolder", label: "上市持有人" },
  { key: "batchQty", label: "批号数量", align: "right" },
  { key: "batchCasePackage", label: "批号件包装" },
  { key: "productionDate", label: "生产日期" },
  { key: "expiryDate", label: "有效期" },
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
    receiving: {
      title: "收货信息",
      state: processState(0, currentStage),
      rows: [
        ["承运商 / 车牌", "华东冷链 / 沪A-12345"],
        ["发运地点", "上海配送中心"],
        ["启运 / 到货", "2026-06-27 08:00 / 2026-06-27 10:00"],
        ["入库时间", "2026-06-27 10:15"],
        ["运输 / 温控 / 温度", "冷藏车 / 冷藏车 / 20℃"],
        ["联系人", "张三 / 13800000000 / 310101********0000"],
        ["随货核对", "印章已核对 / 备案件已核对"],
        ["数量闭合", `${expectedQty} / ${expectedQty} / 0 / 0 件`],
        ["第一收货员", "收货员 0101"],
        ["第二收货员", "收货员 0102"],
        ["异常备注", "-"],
      ],
    },
    inspection: {
      title: "验收信息",
      state: processState(1, currentStage),
      rows: [
        ["通过 / 拒收", `${expectedQty} / 0 件`],
        ["追溯码", "TC-M2-PC-0001"],
        ["质量状态", "合格"],
        ["四项核对", "外观 / 包装 / 说明书 / 标签均合格"],
        ["验收复核", "验收节点命中双人扫码 / 已签字"],
        ["验收 / 复核人", "验收员 0101 / 复核员 0102"],
        ["验收备注", "-"],
      ],
    },
    putaway: {
      title: "上架信息",
      state: processState(2, currentStage),
      rows: [
        ["容器 LPN", "LPN-M2-PC-0001"],
        ["推荐库位", "A-01-01 / A-01-02 / A-02-01"],
        ["实际库位", "待录入"],
        ["校验结果", "待执行"],
        ["上架备注", "-"],
      ],
    },
    completed: {
      title: "完成信息",
      state: processState(3, currentStage),
      rows: [["完成状态", currentStage >= 3 ? "已完成" : "未完成"]],
    },
  } satisfies Record<InboundDetailStage, { title: string; state: ProcessState; rows: Array<[string, string]> }>;
  return map[stage];
}

export function productInfoRows(order: Pick<ReceivingOrder, "lines">) {
  const rows = new Map<string, number>();
  for (const item of order.lines ?? []) {
    rows.set(item.product_code, (rows.get(item.product_code) ?? 0) + item.expected_qty);
  }
  return [...rows.entries()].map(([productCode, orderQty]) => ({
    ...productMasterFields(productCode),
    ...linePackageFields(orderQty),
    key: productCode,
    productCode: productCode || "-",
    orderQty: String(orderQty),
  }));
}

export function batchInfoRows(order: Pick<ReceivingOrder, "lines">) {
  return (order.lines ?? []).map((item) => ({
    ...batchLicenseFields(item.product_code),
    key: `${item.line_no}-${item.batch_no ?? ""}`,
    lineNo: `#${item.line_no}`,
    batchNo: item.batch_no || "-",
    batchQty: String(item.expected_qty),
    batchCasePackage: batchPackageText(item.expected_qty),
    productionDate: item.production_date || "-",
    expiryDate: item.expiry_date || "-",
  }));
}

export function orderLicenseRows(order: Pick<ReceivingOrder, "lines">): Array<[string, string]> {
  const rows = (order.lines ?? []).map((item) => batchLicenseFields(item.product_code));
  return [
    ["批准文号", uniqueText(rows.map((item) => item.approvalNo))],
    ["进口注册证", uniqueText(rows.map((item) => item.importRegistrationCertificate))],
    ["上市持有人", uniqueText(rows.map((item) => item.marketingAuthorizationHolder))],
  ];
}

function processState(index: number, current: number): ProcessState {
  if (index < current) return { label: "已完成", status: "completed" };
  if (index === current) return { label: "当前", status: "in_progress" };
  return { label: "待处理", status: "pending" };
}

function linePackageFields(orderQty: number) {
  // ponytail: ReceivingOrderLine 还没有产品包装主数据；后端补单位/中包/件包字段后替换这里。
  const unit = "件";
  const middlePackQty = 10;
  const casePackQty = 20;
  return {
    unit,
    caseQty: String(Math.floor(orderQty / casePackQty)),
    looseQty: String(orderQty % casePackQty),
    middlePackQty: `${middlePackQty} ${unit}/中包`,
    casePackQty: `${casePackQty} ${unit}/件`,
  };
}

function batchPackageText(batchQty: number) {
  const fields = linePackageFields(batchQty);
  return `${fields.caseQty} 件 + ${fields.looseQty} 零 / ${fields.casePackQty}`;
}

function productMasterFields(productCode: string) {
  // ponytail: ReceivingOrderLine 还没带商品档案字段；后端关联 M1 商品档案后替换这里。
  return {
    productName: productCode.includes("COLD") ? "冷链测试药品" : "感冒灵颗粒",
    specification: productCode.includes("COLD") ? "2ml*10支" : "10g*9袋",
    manufacturer: productCode.includes("COLD") ? "示例冷链药业" : "示例药业",
  };
}

function batchLicenseFields(productCode: string) {
  // ponytail: ReceivingOrderLine 还没带批号资质字段；后端关联批号/药监档案后替换这里。
  return {
    approvalNo: productCode.includes("COLD") ? "国药准字H20240001" : "国药准字Z0001",
    importRegistrationCertificate: "-",
    marketingAuthorizationHolder: productCode.includes("COLD") ? "示例冷链药业" : "示例药业",
  };
}

function uniqueText(values: string[]) {
  const unique = [...new Set(values.filter(Boolean))];
  if (unique.length === 0) return "-";
  if (unique.length <= 2) return unique.join(" / ");
  return `${unique[0]} 等 ${unique.length} 项`;
}
