import * as React from "react";
import { cn } from "../../lib/utils";
import { Card, CardContent } from "../../ui/card";
import { PageHeader, type PageHeaderProps } from "../PageHeader/PageHeader";
import {
  QueryPanel,
  buildQueryPanelSummaryItems,
  type QueryPanelField,
  type QueryPanelOption,
  type QueryPanelProps,
  type QueryPanelQuickFilter,
  type QueryPanelValue,
} from "../QueryPanel";
import { DataGrid, type DataGridProps } from "../DataGrid";

/**
 * ListPageTemplate — WMS 管理端列表型页面标准模板组件
 *
 * 层级：Layer 2 业务复合
 * 关联故事：所有列表型管理端页面
 * Wave：Wave 6 管理端页面标准化
 * 业务约束：统一封装 PageHeader + QueryPanel + 弹性 Card + DataGrid 联动与视口撑满到底部标准。
 *
 * @example
 *   <ListPageTemplate queryFields={fields} queryValue={query} onQuery={fetch} gridProps={gridProps} />
 */
export interface ListPageNotice {
  kind: "error" | "success" | "info" | "warning";
  text: string;
}

export interface ListPageTemplateProps<T = unknown>
  extends Omit<React.HTMLAttributes<HTMLElement>, "title"> {
  /** 页面头部配置 */
  header?: PageHeaderProps;
  /** 状态提示横幅 */
  notice?: ListPageNotice | null;
  /** 自定义横幅插槽 */
  banner?: React.ReactNode;
  /** 查询面板配置 */
  queryFields?: QueryPanelField[];
  fieldOptions?: Record<string, QueryPanelOption[]>;
  coreQueryFieldKeys?: string[];
  queryValue?: QueryPanelValue;
  onQueryValueChange?: (value: QueryPanelValue) => void;
  onQuery?: () => void;
  onReset?: () => void;
  quickFilters?: QueryPanelQuickFilter[];
  quickFiltersAriaLabel?: string;
  onQuickFilterClick?: (key: string) => void;
  /** 查询/加载中状态 */
  loading?: boolean;
  queryPanelProps?: Partial<QueryPanelProps>;
  /** DataGrid 表格配置 */
  gridProps?: DataGridProps<T>;
  /** 动作与详情弹窗槽位 */
  dialogs?: React.ReactNode;
  /** 额外的自定义插槽 */
  children?: React.ReactNode;
}

export const ListPageTemplate = React.forwardRef(function ListPageTemplate<T>(
  {
    header,
    notice,
    banner,
    queryFields,
    fieldOptions,
    coreQueryFieldKeys,
    queryValue,
    onQueryValueChange,
    onQuery,
    onReset,
    quickFilters,
    quickFiltersAriaLabel,
    onQuickFilterClick,
    loading,
    queryPanelProps,
    gridProps,
    dialogs,
    children,
    className,
    ...rest
  }: ListPageTemplateProps<T>,
  ref: React.ForwardedRef<HTMLElement>,
) {
  // 自动构建 querySummaryItems
  const derivedSummaryItems = React.useMemo(() => {
    if (!queryFields || !queryValue) return undefined;
    return buildQueryPanelSummaryItems(queryFields, queryValue);
  }, [queryFields, queryValue]);

  // 双向清空联动
  const handleClearGridQueryState = React.useCallback(
    (key?: string) => {
      if (gridProps?.onClearQueryState) {
        gridProps.onClearQueryState(key);
      }
      if (key && queryValue && onQueryValueChange) {
        const next = { ...queryValue };
        delete next[key];
        onQueryValueChange(next);
        onQuery?.();
      } else if (!key && onReset) {
        onReset();
      }
    },
    [gridProps?.onClearQueryState, queryValue, onQueryValueChange, onQuery, onReset],
  );

  return (
    <section
      ref={ref}
      className={cn(
        "flex w-full flex-1 min-h-0 flex-col gap-5 px-4 py-8 lg:px-8",
        className,
      )}
      {...rest}
    >
      <PageHeader {...header} />

      {banner}

      {!banner && notice && (
        <div
          role={notice.kind === "error" ? "alert" : "status"}
          className={cn(
            "rounded-md border px-3 py-2 text-sm",
            notice.kind === "error" && "border-destructive/30 bg-destructive/10 text-destructive",
            notice.kind === "success" && "border-wms-success/30 bg-wms-success/10 text-wms-success",
            notice.kind === "warning" && "border-wms-warning/30 bg-wms-warning/10 text-wms-warning",
            notice.kind === "info" && "border-primary/30 bg-primary/10 text-primary",
          )}
        >
          {notice.text}
        </div>
      )}

      {queryFields && queryFields.length > 0 && (
        <QueryPanel
          fields={queryFields}
          fieldOptions={fieldOptions}
          defaultVisibleFieldKeys={coreQueryFieldKeys}
          value={queryValue ?? {}}
          onValueChange={onQueryValueChange}
          onQuery={onQuery}
          onReset={onReset}
          quickFilters={quickFilters}
          quickFiltersAriaLabel={quickFiltersAriaLabel}
          onQuickFilterClick={onQuickFilterClick}
          loading={loading ?? queryPanelProps?.loading}
          {...queryPanelProps}
        />
      )}

      {gridProps && (
        <Card className="flex flex-1 flex-col min-h-0 overflow-hidden">
          <CardContent className="flex flex-1 flex-col min-h-0 p-5">
            <DataGrid
              queryState={gridProps.queryState ?? queryValue}
              querySummaryItems={gridProps.querySummaryItems ?? derivedSummaryItems}
              onClearQueryState={handleClearGridQueryState}
              {...gridProps}
            />
          </CardContent>
        </Card>
      )}

      {children}
      {dialogs}
    </section>
  );
}) as <T = unknown>(
  props: ListPageTemplateProps<T> & { ref?: React.ForwardedRef<HTMLElement> },
) => React.ReactElement;
(ListPageTemplate as { displayName?: string }).displayName = "ListPageTemplate";
