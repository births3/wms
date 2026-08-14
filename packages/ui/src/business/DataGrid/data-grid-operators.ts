import type { DataGridFilterType } from "./data-grid-logic";

/**
 * DataGridFilterOperator — 数据网格筛选算子操作符
 *
 * 覆盖文本、数字、日期、枚举、空值全场景
 */
export type DataGridFilterOperator =
  // 文本类
  | "contains" // 包含
  | "notContains" // 不包含
  | "eq" // 等于
  | "ne" // 不等于
  | "startsWith" // 开头是
  | "endsWith" // 结尾是
  // 数字/日期比较
  | "gt" // 大于 / 晚于
  | "gte" // 大于等于
  | "lt" // 小于 / 早于
  | "lte" // 小于等于
  | "between" // 介于
  // 日期专属相对时间
  | "today" // 今天
  | "thisWeek" // 本周
  | "thisMonth" // 本月
  // 枚举/集合类
  | "isAnyOf" // 属于任意一个
  | "isNoneOf" // 不属于任意一个
  // 空值判断
  | "isEmpty" // 为空
  | "isNotEmpty"; // 不为空

export interface DataGridOperatorDefinition {
  value: DataGridFilterOperator;
  label: string;
  shortLabel: string;
  noValueRequired?: boolean;
}

export const DATA_GRID_OPERATORS: Record<DataGridFilterOperator, DataGridOperatorDefinition> = {
  contains: { value: "contains", label: "包含", shortLabel: "包含" },
  notContains: { value: "notContains", label: "不包含", shortLabel: "不含" },
  eq: { value: "eq", label: "等于", shortLabel: "=" },
  ne: { value: "ne", label: "不等于", shortLabel: "≠" },
  startsWith: { value: "startsWith", label: "开头为", shortLabel: "开头为" },
  endsWith: { value: "endsWith", label: "结尾为", shortLabel: "结尾为" },
  gt: { value: "gt", label: "大于", shortLabel: ">" },
  gte: { value: "gte", label: "大于等于", shortLabel: "≥" },
  lt: { value: "lt", label: "小于", shortLabel: "<" },
  lte: { value: "lte", label: "小于等于", shortLabel: "≤" },
  between: { value: "between", label: "介于", shortLabel: "介于" },
  today: { value: "today", label: "今天", shortLabel: "今天", noValueRequired: true },
  thisWeek: { value: "thisWeek", label: "本周", shortLabel: "本周", noValueRequired: true },
  thisMonth: { value: "thisMonth", label: "本月", shortLabel: "本月", noValueRequired: true },
  isAnyOf: { value: "isAnyOf", label: "属于任意", shortLabel: "属于" },
  isNoneOf: { value: "isNoneOf", label: "不属于", shortLabel: "不属于" },
  isEmpty: { value: "isEmpty", label: "为空", shortLabel: "为空", noValueRequired: true },
  isNotEmpty: { value: "isNotEmpty", label: "不为空", shortLabel: "不为空", noValueRequired: true },
};

const TEXT_OPERATORS: DataGridFilterOperator[] = [
  "contains",
  "notContains",
  "eq",
  "ne",
  "startsWith",
  "endsWith",
  "isEmpty",
  "isNotEmpty",
];

const NUMBER_OPERATORS: DataGridFilterOperator[] = [
  "eq",
  "ne",
  "gt",
  "gte",
  "lt",
  "lte",
  "between",
  "isEmpty",
  "isNotEmpty",
];

const DATE_OPERATORS: DataGridFilterOperator[] = [
  "between",
  "eq",
  "ne",
  "gt",
  "gte",
  "lt",
  "lte",
  "today",
  "thisWeek",
  "thisMonth",
  "isEmpty",
  "isNotEmpty",
];

const SELECT_OPERATORS: DataGridFilterOperator[] = [
  "eq",
  "ne",
  "isAnyOf",
  "isNoneOf",
  "isEmpty",
  "isNotEmpty",
];

const MULTI_SELECT_OPERATORS: DataGridFilterOperator[] = [
  "isAnyOf",
  "isNoneOf",
  "isEmpty",
  "isNotEmpty",
];

/**
 * 根据列筛选类型返回支持的操作符列表
 */
export function getOperatorsForFilterType(type: DataGridFilterType | undefined): DataGridFilterOperator[] {
  switch (type) {
    case "numberRange":
      return NUMBER_OPERATORS;
    case "dateRange":
      return DATE_OPERATORS;
    case "select":
      return SELECT_OPERATORS;
    case "multiSelect":
      return MULTI_SELECT_OPERATORS;
    case "text":
    default:
      return TEXT_OPERATORS;
  }
}

/**
 * 获取指定类型的默认操作符
 */
export function getDefaultOperatorForFilterType(type: DataGridFilterType | undefined): DataGridFilterOperator {
  switch (type) {
    case "numberRange":
      return "gte";
    case "dateRange":
      return "between";
    case "select":
      return "eq";
    case "multiSelect":
      return "isAnyOf";
    case "text":
    default:
      return "contains";
  }
}

/**
 * 操作符是否无需输入值
 */
export function operatorRequiresNoValue(operator: DataGridFilterOperator): boolean {
  return Boolean(DATA_GRID_OPERATORS[operator]?.noValueRequired);
}

/**
 * 获取操作符的中文显示名
 */
export function getOperatorLabel(operator: DataGridFilterOperator, short = false): string {
  const def = DATA_GRID_OPERATORS[operator];
  if (!def) return operator;
  return short ? def.shortLabel : def.label;
}

/**
 * 单条高级筛选条件项
 */
export interface DataGridFilterItem {
  id: string;
  columnKey: string;
  operator: DataGridFilterOperator;
  value?: unknown;
}

/**
 * 高级筛选聚合状态
 */
export interface DataGridAdvancedFilterState {
  joinOperator: "and" | "or";
  items: DataGridFilterItem[];
}
