import type { DataGridColumn } from "@wms/ui";
import {
  type MasterDataRow,
  productSourceLabel,
  type MasterDataViewId,
} from "@/features/master-data/master-data-queries";

export { productSourceLabel };

const sourceColumn: DataGridColumn<MasterDataRow> = {
  key: "source",
  header: "来源",
  width: 160,
  minWidth: 140,
  sortable: true,
  sortValue: (row) => row.sourceValue ?? "-",
  filterValue: (row) => row.sourceValue ?? "-",
  copyValue: (row) => row.sourceValue ?? "-",
  render: (row) => row.sourceValue ?? "-",
  filter: {
    type: "multiSelect",
    options: [
      { label: "手工新建", value: "手工新建" },
      { label: "批量导入", value: "批量导入" },
      { label: "API接口导入", value: "API接口导入" },
    ],
  },
};

const businessPartnerTypeColumn: DataGridColumn<MasterDataRow> = {
  key: "businessPartnerType",
  header: "类型",
  width: 140,
  minWidth: 120,
  sortable: true,
  sortValue: (row) => row.partnerTypeLabel ?? "-",
  filterValue: (row) => row.partnerTypeLabel ?? "-",
  copyValue: (row) => row.partnerTypeLabel ?? "-",
  render: (row) => row.partnerTypeLabel ?? "-",
  filter: {
    type: "multiSelect",
    options: [
      { label: "供应商", value: "供应商" },
      { label: "客户/门店", value: "客户/门店" },
    ],
  },
};

/** 商品档案业务字段列（治理脚本要求 productCoreColumns + 明确 header 文案） */
export const productCoreColumns: DataGridColumn<MasterDataRow>[] = [
  {
    key: "primary",
    header: "规格",
    width: 230,
    minWidth: 190,
    filterValue: (row) => `${row.primaryLabel} ${row.primaryValue}`,
    copyValue: (row) => row.primaryValue,
    filter: { type: "text" },
    render: (row) => row.primaryValue || "-",
  },
  {
    key: "secondary",
    header: "批准文号",
    width: 240,
    minWidth: 200,
    filterValue: (row) => `${row.secondaryLabel} ${row.secondaryValue}`,
    copyValue: (row) => row.secondaryValue,
    filter: { type: "text" },
    render: (row) => row.secondaryValue || "-",
  },
  {
    key: "extra",
    header: "储存条件",
    width: 160,
    minWidth: 140,
    filterValue: (row) => `${row.extraLabel} ${row.extraValue}`,
    copyValue: (row) => row.extraValue,
    filter: { type: "text" },
    render: (row) => row.extraValue || "-",
  },
];

/** 按 view 把通用 primary/secondary/extra 列头改成真实业务语义 */
const viewFieldHeaders: Partial<
  Record<MasterDataViewId, { primary: string; secondary: string; extra: string }>
> = {
  "m1-business-partners": { primary: "资质证号", secondary: "联系人 / 类型", extra: "档案类型 / 货主" },
  "m1-warehouses": { primary: "货主", secondary: "档案类型", extra: "仓库名称" },
  "m1-zones": { primary: "仓库", secondary: "库区", extra: "库位数" },
  "m1-system-dictionary": { primary: "字典分类", secondary: "来源", extra: "参数" },
};

function withViewFieldHeaders(
  viewId: MasterDataViewId,
  baseColumns: DataGridColumn<MasterDataRow>[],
): DataGridColumn<MasterDataRow>[] {
  const headers = viewFieldHeaders[viewId];
  if (!headers) return baseColumns;
  return baseColumns.map((column) => {
    if (column.key === "primary") return { ...column, header: headers.primary };
    if (column.key === "secondary") return { ...column, header: headers.secondary };
    if (column.key === "extra") return { ...column, header: headers.extra };
    return column;
  });
}

function withProductCoreColumns(baseColumns: DataGridColumn<MasterDataRow>[]): DataGridColumn<MasterDataRow>[] {
  const byKey = new Map(productCoreColumns.map((column) => [column.key, column]));
  return baseColumns.map((column) => byKey.get(column.key) ?? column);
}

export function masterDataColumns(
  viewId: MasterDataViewId,
  baseColumns: DataGridColumn<MasterDataRow>[],
  locationColumns: DataGridColumn<MasterDataRow>[],
) {
  if (viewId === "m1-locations") return locationColumns;
  if (viewId === "m1-products") {
    return [...withProductCoreColumns(baseColumns), sourceColumn];
  }
  const columns = withViewFieldHeaders(viewId, baseColumns);
  if (viewId === "m1-business-partners") {
    return [...columns, businessPartnerTypeColumn, sourceColumn];
  }
  return columns;
}

export function masterDataActionLabels(viewId: MasterDataViewId) {
  if (viewId === "m1-products") return ["新建商品", "批量导入"];
  if (viewId === "m1-business-partners") return ["新建供应商", "导入供应商", "新建客户", "导入客户"];
  return [];
}

export function productTableClassName(viewId: MasterDataViewId) {
  if (viewId === "m1-locations") return "min-w-[1720px]";
  if (viewId === "m1-products") return "min-w-[2380px]";
  if (viewId === "m1-business-partners") return "min-w-[1680px]";
  return "min-w-[1460px]";
}
