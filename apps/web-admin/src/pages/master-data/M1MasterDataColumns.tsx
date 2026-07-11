import { StatusBadge, type DataGridColumn, type StatusKey } from "@wms/ui";

import type {
  LocationMasterDataFields,
  MasterDataRow,
} from "@/features/master-data/master-data-queries";

export const baseMasterDataColumns: DataGridColumn<MasterDataRow>[] = [
  {
    key: "code",
    header: "编码",
    mono: true,
    width: 220,
    minWidth: 180,
    sortable: true,
    sortValue: (row) => row.code,
    filterValue: (row) => row.code,
    copyValue: (row) => row.code,
    filter: { type: "text" },
    render: (row) => <span className="text-primary">{row.code}</span>,
  },
  {
    key: "name",
    header: "名称",
    width: 240,
    minWidth: 200,
    sortable: true,
    sortValue: (row) => row.name,
    filterValue: (row) => row.name,
    copyValue: (row) => row.name,
    filter: { type: "text" },
  },
  {
    key: "status",
    header: "状态",
    width: 130,
    minWidth: 120,
    sortable: true,
    sortValue: (row) => row.statusLabel,
    filterValue: (row) => statusFilterValue(row.status),
    copyValue: (row) => row.statusLabel,
    filter: {
      type: "multiSelect",
      options: [
        { label: "启用/可用", value: "active" },
        { label: "停用", value: "disabled" },
        { label: "其他", value: "other" },
      ],
    },
    render: (row) => <StatusBadge status={statusKey(row.status)} label={row.statusLabel} size="sm" />,
  },
  {
    key: "primary",
    header: "关键字段",
    width: 230,
    minWidth: 190,
    filterValue: (row) => `${row.primaryLabel} ${row.primaryValue}`,
    copyValue: (row) => `${row.primaryLabel}: ${row.primaryValue}`,
    filter: { type: "text" },
    render: (row) => <FieldText label={row.primaryLabel} value={row.primaryValue} />,
  },
  {
    key: "secondary",
    header: "扩展字段",
    width: 240,
    minWidth: 200,
    filterValue: (row) => `${row.secondaryLabel} ${row.secondaryValue}`,
    copyValue: (row) => `${row.secondaryLabel}: ${row.secondaryValue}`,
    filter: { type: "text" },
    render: (row) => <FieldText label={row.secondaryLabel} value={row.secondaryValue} />,
  },
  {
    key: "extra",
    header: "运行字段",
    width: 260,
    minWidth: 220,
    filterValue: (row) => `${row.extraLabel} ${row.extraValue}`,
    copyValue: (row) => `${row.extraLabel}: ${row.extraValue}`,
    filter: { type: "text" },
    render: (row) => <FieldText label={row.extraLabel} value={row.extraValue} />,
  },
  {
    key: "createdAt",
    header: "创建时间",
    width: 190,
    minWidth: 170,
    sortable: true,
    sortValue: (row) => row.createdAt,
    filterValue: (row) => row.createdAt,
    copyValue: (row) => formatDateTime(row.createdAt),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.createdAt),
  },
  {
    key: "updatedAt",
    header: "更新时间",
    width: 190,
    minWidth: 170,
    sortable: true,
    sortValue: (row) => row.updatedAt,
    filterValue: (row) => row.updatedAt,
    copyValue: (row) => formatDateTime(row.updatedAt),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.updatedAt),
  },
];

export const locationMasterDataColumns: DataGridColumn<MasterDataRow>[] = [
  {
    key: "owner",
    header: "货主",
    width: 160,
    minWidth: 140,
    sortable: true,
    sortValue: (row) => locationValue(row, "owner"),
    filterValue: (row) => locationValue(row, "owner"),
    copyValue: (row) => locationValue(row, "owner"),
    filter: { type: "text" },
    render: (row) => shortDisplayId(locationValue(row, "owner")),
  },
  {
    key: "warehouse",
    header: "仓库 / 库区",
    width: 280,
    minWidth: 240,
    filterValue: (row) => `${locationValue(row, "warehouse")} ${locationValue(row, "zone")}`,
    copyValue: (row) => `${locationValue(row, "warehouse")} / ${locationValue(row, "zone")}`,
    filter: { type: "text" },
    render: (row) => (
      <FieldText label={`库区 ${locationValue(row, "zone")}`} value={locationValue(row, "warehouse")} />
    ),
  },
  {
    key: "code",
    header: "库位编码",
    mono: true,
    width: 190,
    minWidth: 170,
    sortable: true,
    sortValue: (row) => row.code,
    filterValue: (row) => row.code,
    copyValue: (row) => row.code,
    filter: { type: "text" },
    render: (row) => <span className="text-primary">{row.code}</span>,
  },
  {
    key: "coordinate",
    header: "区域 / 排列层",
    width: 210,
    minWidth: 190,
    filterValue: (row) =>
      `${locationValue(row, "area")} ${locationValue(row, "rowNo")} ${locationValue(row, "columnNo")} ${locationValue(row, "layerNo")}`,
    copyValue: (row) =>
      `区域 ${locationValue(row, "area")} / 排 ${locationValue(row, "rowNo")} / 列 ${locationValue(row, "columnNo")} / 层 ${locationValue(row, "layerNo")}`,
    filter: { type: "text" },
    render: (row) => (
      <FieldText
        label={`排 ${locationValue(row, "rowNo")} / 列 ${locationValue(row, "columnNo")} / 层 ${locationValue(row, "layerNo")}`}
        value={`区域 ${locationValue(row, "area")}`}
      />
    ),
  },
  {
    key: "locationType",
    header: "库位类型",
    width: 140,
    minWidth: 120,
    sortable: true,
    sortValue: (row) => locationValue(row, "locationType"),
    filterValue: (row) => locationValue(row, "locationType"),
    copyValue: (row) => locationValue(row, "locationType"),
    filter: { type: "text" },
  },
  {
    key: "status",
    header: "状态",
    width: 130,
    minWidth: 120,
    sortable: true,
    sortValue: (row) => row.statusLabel,
    filterValue: (row) => statusFilterValue(row.status),
    copyValue: (row) => row.statusLabel,
    filter: {
      type: "multiSelect",
      options: [
        { label: "可用", value: "active" },
        { label: "停用/锁定", value: "disabled" },
        { label: "其他", value: "other" },
      ],
    },
    render: (row) => <StatusBadge status={statusKey(row.status)} label={row.statusLabel} size="sm" />,
  },
  {
    key: "volume",
    header: "已用 / 最大体积",
    width: 180,
    minWidth: 160,
    filterValue: (row) => locationValue(row, "volume"),
    copyValue: (row) => locationValue(row, "volume"),
    filter: { type: "text" },
  },
  {
    key: "maxSku",
    header: "最大 SKU",
    width: 130,
    minWidth: 120,
    sortable: true,
    sortValue: (row) => Number.parseInt(locationValue(row, "maxSku"), 10) || 0,
    filterValue: (row) => locationValue(row, "maxSku"),
    copyValue: (row) => locationValue(row, "maxSku"),
    filter: { type: "text" },
  },
  {
    key: "createdAt",
    header: "创建时间",
    width: 190,
    minWidth: 170,
    sortable: true,
    sortValue: (row) => row.createdAt,
    filterValue: (row) => row.createdAt,
    copyValue: (row) => formatDateTime(row.createdAt),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.createdAt),
  },
  {
    key: "updatedAt",
    header: "更新时间",
    width: 190,
    minWidth: 170,
    sortable: true,
    sortValue: (row) => row.updatedAt,
    filterValue: (row) => row.updatedAt,
    copyValue: (row) => formatDateTime(row.updatedAt),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.updatedAt),
  },
];

function FieldText({ label, value }: { label: string; value: string }) {
  return (
    <div className="text-sm">
      <div className="font-medium">{value}</div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </div>
  );
}

function locationValue(row: MasterDataRow, key: keyof LocationMasterDataFields) {
  return row.locationFields?.[key] ?? "-";
}

function statusKey(status: string): StatusKey {
  if (status === "active" || status === "available") return "completed";
  if (status === "disabled" || status === "inactive" || status === "locked") return "isolated";
  return "pending";
}

function statusFilterValue(status: string) {
  if (status === "active" || status === "available") return "active";
  if (status === "disabled" || status === "inactive" || status === "locked") return "disabled";
  return "other";
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value || "-";
  return date.toLocaleString("zh-CN", { hour12: false });
}

function shortDisplayId(value: string) {
  if (!value || value === "-") return "-";
  return value.length > 8 ? value.slice(0, 8) : value;
}
