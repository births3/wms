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

function productCoreColumns(baseColumns: DataGridColumn<MasterDataRow>[]): DataGridColumn<MasterDataRow>[] {
  return baseColumns.map((column) => {
    if (column.key === "primary") return { ...column, header: "规格" };
    if (column.key === "secondary") return { ...column, header: "批准文号" };
    if (column.key === "extra") return { ...column, header: "储存条件" };
    return column;
  });
}

export function masterDataColumns(
  viewId: MasterDataViewId,
  baseColumns: DataGridColumn<MasterDataRow>[],
  locationColumns: DataGridColumn<MasterDataRow>[],
) {
  if (viewId === "m1-products") {
    return [...productCoreColumns(baseColumns), sourceColumn];
  }
  if (viewId === "m1-business-partners") {
    return [...baseColumns, businessPartnerTypeColumn, sourceColumn];
  }
  if (viewId === "m1-locations") return locationColumns;
  return baseColumns;
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
