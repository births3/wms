/**
 * 打印编排（H9）状态与文案映射收敛。
 * 组套/规则生命周期状态 → 完成态与中文文案；冻结实例状态、分类 PDF 处理状态；
 * 归集维度字段编码 → 字段名。字符串值逐字沿用各页面历史实现。
 */

import {
  COLUMN_DOCUMENT_TYPE,
  STATUS_CANCELLED,
  STATUS_DEACTIVATED,
  STATUS_DRAFT,
  STATUS_PENDING,
  STATUS_PUBLISHED,
} from "@/lib/ui-strings";

/** 组套/规则生命周期状态 → 完成态（StatusBadge 用）。 */
export function statusCompletion(status: string) {
  return status === "published" ? "completed" : status === "disabled" ? "expired" : status === "tested" ? "in_progress" : "pending";
}

/** 组套/规则生命周期状态 → 中文文案。 */
export function statusLabel(status: string) {
  return status === "published" ? STATUS_PUBLISHED : status === "disabled" ? STATUS_DEACTIVATED : status === "tested" ? "已测试" : STATUS_DRAFT;
}

/** 冻结组套实例状态 → 中文文案。 */
export function instanceStatusLabel(status: string) {
  return status === "queued" ? "待打印" : status === "cancelled" ? STATUS_CANCELLED : "等待分类 PDF";
}

/** 分类 PDF 处理状态 → 中文文案。 */
export function processingStatusLabel(status: string) {
  return status === "ready"
    ? "已就绪"
    : status === "failed"
      ? "生成失败"
      : status === "processing"
        ? "处理中"
        : STATUS_PENDING;
}

/** 归集维度字段编码 → 字段名。 */
export const fieldLabels: Record<string, string> = {
  document_type: COLUMN_DOCUMENT_TYPE,
  erp_order_no: "ERP 订单号",
  invoice_no: "发票号",
  transport_mode_code: "运输方式",
  department_code: "业务部门",
  sales_group_code: "销售组",
  order_group_no: "订单组号",
  business_type_code: "业务类型",
};

/** 归集维度字段编码 → 字段名，未知编码原样返回。 */
export function aggregationFieldLabel(code: string) {
  return fieldLabels[code] ?? code;
}
