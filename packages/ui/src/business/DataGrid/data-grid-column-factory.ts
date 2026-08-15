/**
 * data-grid-column-factory — DataGrid 标准列工厂函数
 *
 * 层级：Layer 2 业务复合
 * 作用：将常用的状态徽章、等宽单号/编码、日期时间、高亮数值等重复列配置收敛为 1 行声明式调用。
 */

import * as React from "react";
import type { StatusKey } from "../StatusBadge/StatusBadge";
import { StatusBadge } from "../StatusBadge/StatusBadge";
import type { DataGridColumn } from "./data-grid-types";

// ==========================================
// 1. 状态列工厂 (Status Column)
// ==========================================

export interface CreateStatusColumnOptions<T> {
  key: keyof T & string;
  header?: string;
  width?: number;
  minWidth?: number;
  statusMap?: Record<string, { label: string; status: StatusKey }>;
  getStatus?: (row: T) => StatusKey;
  getLabel?: (row: T) => string;
  sortable?: boolean;
}

export function createStatusColumn<T>(options: CreateStatusColumnOptions<T>): DataGridColumn<T> {
  const {
    key,
    header = "状态",
    width = 120,
    minWidth = 100,
    statusMap,
    getStatus,
    getLabel,
    sortable = true,
  } = options;

  return {
    key,
    header,
    width,
    minWidth,
    sortable,
    sortValue: (row: T) => String(row[key] ?? ""),
    filterValue: (row: T) => String(row[key] ?? ""),
    copyValue: (row: T) => {
      if (getLabel) return getLabel(row);
      const raw = String(row[key] ?? "");
      return statusMap?.[raw]?.label ?? raw;
    },
    render: (row: T) => {
      const raw = String(row[key] ?? "");
      const badgeStatus: StatusKey = getStatus ? getStatus(row) : statusMap?.[raw]?.status ?? "isolated";
      const badgeLabel = getLabel ? getLabel(row) : statusMap?.[raw]?.label ?? raw;
      return React.createElement(StatusBadge, { status: badgeStatus, label: badgeLabel, size: "sm" });
    },
  };
}

// ==========================================
// 2. 等宽单号 / 编码列工厂 (Mono Column)
// ==========================================

export interface CreateMonoColumnOptions<T> {
  key: keyof T & string;
  header: string;
  width?: number;
  minWidth?: number;
  copyable?: boolean;
  sortable?: boolean;
}

export function createMonoColumn<T>(options: CreateMonoColumnOptions<T>): DataGridColumn<T> {
  const {
    key,
    header,
    width = 160,
    minWidth = 120,
    copyable = true,
    sortable = true,
  } = options;

  return {
    key,
    header,
    width,
    minWidth,
    mono: true,
    copyable,
    sortable,
    sortValue: (row: T) => String(row[key] ?? ""),
    filterValue: (row: T) => String(row[key] ?? ""),
    copyValue: (row: T) => String(row[key] ?? ""),
    render: (row: T) => React.createElement("span", { className: "font-mono text-xs font-medium text-foreground" }, String(row[key] ?? "-")),
  };
}

// ==========================================
// 3. 日期时间列工厂 (Date / Time Column)
// ==========================================

export interface CreateDateColumnOptions<T> {
  key: keyof T & string;
  header: string;
  width?: number;
  minWidth?: number;
  formatter?: (val: unknown) => string;
  sortable?: boolean;
}

export function createDateColumn<T>(options: CreateDateColumnOptions<T>): DataGridColumn<T> {
  const {
    key,
    header,
    width = 160,
    minWidth = 140,
    formatter,
    sortable = true,
  } = options;

  return {
    key,
    header,
    width,
    minWidth,
    sortable,
    sortValue: (row: T) => String(row[key] ?? ""),
    filterValue: (row: T) => String(row[key] ?? ""),
    copyValue: (row: T) => (formatter ? formatter(row[key]) : String(row[key] ?? "")),
    render: (row: T) => {
      const raw = row[key];
      if (!raw) return React.createElement("span", { className: "text-muted-foreground" }, "-");
      const formatted = formatter ? formatter(raw) : String(raw);
      return React.createElement("span", { className: "tabular-nums text-xs text-muted-foreground" }, formatted);
    },
  };
}

// ==========================================
// 4. 数值 / 数量 / 金额列工厂 (Numeric Column)
// ==========================================

export interface CreateNumericColumnOptions<T> {
  key: keyof T & string;
  header: string;
  width?: number;
  minWidth?: number;
  unit?: string;
  sortable?: boolean;
}

export function createNumericColumn<T>(options: CreateNumericColumnOptions<T>): DataGridColumn<T> {
  const {
    key,
    header,
    width = 110,
    minWidth = 90,
    unit,
    sortable = true,
  } = options;

  return {
    key,
    header,
    width,
    minWidth,
    align: "right",
    sortable,
    sortValue: (row: T) => Number(row[key] ?? 0),
    filterValue: (row: T) => Number(row[key] ?? 0),
    copyValue: (row: T) => `${row[key] ?? 0}${unit ? ` ${unit}` : ""}`,
    render: (row: T) => {
      const val = row[key];
      const children: (string | React.ReactNode)[] = [Number(val ?? 0).toLocaleString()];
      if (unit) {
        children.push(React.createElement("span", { key: "unit", className: "ml-1 text-xs font-normal text-muted-foreground" }, unit));
      }
      return React.createElement("span", { className: "font-semibold tabular-nums text-foreground" }, ...children);
    },
  };
}
