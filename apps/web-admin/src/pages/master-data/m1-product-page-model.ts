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

export function masterDataColumns(
  viewId: MasterDataViewId,
  baseColumns: DataGridColumn<MasterDataRow>[],
  locationColumns: DataGridColumn<MasterDataRow>[],
) {
  if (viewId === "m1-products" || viewId === "m1-suppliers" || viewId === "m1-customers") {
    return [...baseColumns, sourceColumn];
  }
  if (viewId === "m1-locations") return locationColumns;
  return baseColumns;
}

export function masterDataActionLabels(viewId: MasterDataViewId) {
  if (viewId === "m1-products") return ["新建商品", "批量导入"];
  if (viewId === "m1-suppliers") return ["新建供应商", "批量导入"];
  if (viewId === "m1-customers") return ["新建客户", "批量导入"];
  return [];
}

export function productTableClassName(viewId: MasterDataViewId) {
  if (viewId === "m1-locations") return "min-w-[1720px]";
  if (viewId === "m1-products") return "min-w-[1680px]";
  return "min-w-[1460px]";
}
