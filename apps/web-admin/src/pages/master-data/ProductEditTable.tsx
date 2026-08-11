import type { DataGridColumn } from "@wms/ui";

import type { MasterDataRow } from "@/features/master-data/master-data-queries";

export function productColumns(
  baseColumns: DataGridColumn<MasterDataRow>[],
  specialDrugCategoryLabels: ReadonlyMap<string, string> = new Map(),
): DataGridColumn<MasterDataRow>[] {
  return [
    ...baseColumns,
    {
      key: "dosageForm",
      header: "剂型",
      width: 150,
      minWidth: 130,
      filterValue: (row) => productValue(row, "dosageForm"),
      copyValue: (row) => productValue(row, "dosageForm"),
      filter: { type: "text" },
    },
    {
      key: "manufacturer",
      header: "生产厂家",
      width: 220,
      minWidth: 180,
      filterValue: (row) => productValue(row, "manufacturer"),
      copyValue: (row) => productValue(row, "manufacturer"),
      filter: { type: "text" },
    },
    {
      key: "specialDrugCategory",
      header: "特殊药品标识",
      width: 170,
      minWidth: 150,
      filterValue: (row) => specialDrugCategoryText(row, specialDrugCategoryLabels),
      copyValue: (row) => specialDrugCategoryText(row, specialDrugCategoryLabels),
      filter: { type: "text" },
      render: (row) => (
        <span className="text-sm">{specialDrugCategoryText(row, specialDrugCategoryLabels)}</span>
      ),
    },
    {
      key: "intermediatePackaging",
      header: "中包装转换比",
      width: 140,
      minWidth: 130,
      filterValue: intermediatePackagingText,
      copyValue: intermediatePackagingText,
      filter: { type: "text" },
      render: (row) => <span className="text-sm">{intermediatePackagingText(row)}</span>,
    },
    {
      key: "barcode69",
      header: "69 码",
      mono: true,
      width: 180,
      minWidth: 160,
      filterValue: (row) => productValue(row, "barcode69"),
      copyValue: (row) => productValue(row, "barcode69"),
      filter: { type: "text" },
    },
    {
      key: "productPackaging",
      header: "包装层级",
      width: 220,
      minWidth: 190,
      filterValue: productPackagingText,
      copyValue: productPackagingText,
      filter: { type: "text" },
      render: (row) => (
        <span className="text-sm">{productValue(row, "packagingText")}</span>
      ),
    },
    {
      key: "regulatoryCodes",
      header: "监管标识",
      width: 260,
      minWidth: 220,
      filterValue: regulatoryCodesText,
      copyValue: regulatoryCodesText,
      filter: { type: "text" },
      render: (row) => (
        <TwoLine
          top={`UDI ${productValue(row, "udiCode")}`}
          bottom={`电子监管码 ${productValue(row, "electronicRegulatoryCode")}`}
        />
      ),
    },
    {
      key: "unitSize",
      header: "单位尺寸",
      width: 260,
      minWidth: 220,
      filterValue: unitSizeText,
      copyValue: unitSizeText,
      filter: { type: "text" },
      render: (row) => (
        <TwoLine
          top={`长 ${productValue(row, "lengthMm")} mm / 宽 ${productValue(row, "widthMm")} mm`}
          bottom={`高 ${productValue(row, "heightMm")} mm`}
        />
      ),
    },
    {
      key: "unitWeightVolume",
      header: "重量 / 体积",
      width: 230,
      minWidth: 200,
      filterValue: unitWeightVolumeText,
      copyValue: unitWeightVolumeText,
      filter: { type: "text" },
      render: (row) => (
        <TwoLine
          top={`重量 ${productValue(row, "weightG")} g`}
          bottom={`体积 ${productValue(row, "volumeCm3")} cm³`}
        />
      ),
    },
    {
      key: "mappingTrace",
      header: "映射溯源",
      width: 220,
      minWidth: 190,
      filterValue: mappingTraceText,
      copyValue: mappingTraceText,
      filter: { type: "text" },
      render: (row) => <span className="text-sm">{mappingTraceText(row)}</span>,
    },
  ];
}

function productValue(
  row: MasterDataRow,
  key:
    | "barcode69"
    | "dosageForm"
    | "electronicRegulatoryCode"
    | "heightMm"
    | "lengthMm"
    | "manufacturer"
    | "packagingText"
    | "udiCode"
    | "volumeCm3"
    | "weightG"
    | "widthMm",
) {
  return row.productFields?.[key]?.trim() || "-";
}

/** 特殊药品标识：优先字典 code → 名称映射，无映射回退显示 code，空值显示 "-" */
function specialDrugCategoryText(
  row: MasterDataRow,
  labels: ReadonlyMap<string, string>,
) {
  const code = row.productFields?.specialDrugCategoryCode?.trim();
  if (!code) return "-";
  return labels.get(code) ?? code;
}

/** 中包装转换比：取非基础层级中 ratio_to_base > 1（或非最小 sort_order）的第一个，显示 "1:ratio"；无中包装显示 "-" */
function intermediatePackagingText(row: MasterDataRow) {
  const levels = (row.productFields?.packagingLevels ?? []).filter((level) => !level.isBase);
  if (levels.length === 0) return "-";
  const minSortOrder = Math.min(...levels.map((level) => level.sortOrder));
  const middle = levels.find(
    (level) => level.ratioToBase > 1 || level.sortOrder !== minSortOrder,
  );
  if (!middle || middle.ratioToBase <= 1) return "-";
  return `1:${middle.ratioToBase}`;
}

function productPackagingText(row: MasterDataRow) {
  return productValue(row, "packagingText");
}

function unitSizeText(row: MasterDataRow) {
  return `长 ${productValue(row, "lengthMm")} mm / 宽 ${productValue(row, "widthMm")} mm / 高 ${productValue(row, "heightMm")} mm`;
}

function unitWeightVolumeText(row: MasterDataRow) {
  return `重量 ${productValue(row, "weightG")} g / 体积 ${productValue(row, "volumeCm3")} cm³`;
}

function regulatoryCodesText(row: MasterDataRow) {
  return `UDI ${productValue(row, "udiCode")} / 电子监管码 ${productValue(row, "electronicRegulatoryCode")}`;
}

function mappingTraceText(row: MasterDataRow) {
  const traces = row.productFields?.mappingTraces ?? [];
  if (traces.length === 0) return "无外部映射";
  const latest = traces[traces.length - 1];
  return `${latest?.source_system ?? "-"} · ${traces.length} 条`;
}

function TwoLine({ top, bottom }: { top: string; bottom: string }) {
  return (
    <div className="text-sm">
      <div className="font-medium">{top}</div>
      <div className="text-xs text-muted-foreground">{bottom}</div>
    </div>
  );
}
