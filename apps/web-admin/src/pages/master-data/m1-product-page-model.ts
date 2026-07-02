import type { DataGridColumn } from "@wms/ui";
import {
  type MasterDataRow,
  productSourceLabel,
  type MasterDataViewId,
} from "@/features/master-data/master-data-queries";

export { productSourceLabel };

const productSourceColumn: DataGridColumn<MasterDataRow> = {
  key: "source",
  header: "来源",
  width: 160,
  minWidth: 140,
  sortable: true,
  sortValue: (row) => row.sourceValue ?? "-",
  filterValue: (row) => row.sourceValue ?? "-",
  copyValue: (row) => row.sourceValue ?? "-",
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
  if (viewId === "m1-products") return [...baseColumns, productSourceColumn];
  if (viewId === "m1-locations") return locationColumns;
  return baseColumns;
}

export function productActionLabels(viewId: MasterDataViewId) {
  return viewId === "m1-products" ? ["新建商品", "批量导入"] : [];
}

export function productTableClassName(viewId: MasterDataViewId) {
  if (viewId === "m1-locations") return "min-w-[1720px]";
  if (viewId === "m1-products") return "min-w-[1680px]";
  return "min-w-[1460px]";
}
