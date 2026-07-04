import * as React from "react";
import { StatusBadge } from "../StatusBadge";
import { TwoPaneCatalog, type TwoPaneCatalogField } from "../TwoPaneCatalog";
import {
  summarizeSystemDictionaryParams,
  systemDictionarySourceText,
  type SystemDictionaryTwoPaneGroup,
  type SystemDictionaryTwoPaneItem,
} from "./system-dictionary-two-pane-logic";

export interface SystemDictionaryTwoPaneProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children"> {
  groups: SystemDictionaryTwoPaneGroup[];
  initialGroupCode?: string;
  selectedGroupCode?: string;
  onSelectedGroupCodeChange?: (groupCode: string) => void;
  storageKey?: string;
  selectable?: boolean;
  selectedItemKeys?: string[];
  onSelectedItemKeysChange?: (keys: string[]) => void;
  headerActions?: React.ReactNode;
  renderItemActions?: (item: SystemDictionaryTwoPaneItem) => React.ReactNode;
  emptyTitle?: string;
  emptyDescription?: string;
  loading?: boolean;
  error?: React.ReactNode;
  onRefresh?: () => void;
}

/**
 * SystemDictionaryTwoPane — 系统字典两层展示
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M1-011 系统字典 / 单据类型参数配置
 * Wave：Wave 3 管理端基础档案
 * 业务约束：左层只展示字典分类，右层只展示选中分类的字典项与参数摘要。
 *
 * @example
 *   <SystemDictionaryTwoPane groups={[{ code: "document_type", name: "单据类型", items: [] }]} />
 */
export const SystemDictionaryTwoPane = React.forwardRef<
  HTMLDivElement,
  SystemDictionaryTwoPaneProps
>(
  (
    {
      groups,
      initialGroupCode,
      selectedGroupCode,
      onSelectedGroupCodeChange,
      storageKey,
      selectable,
      selectedItemKeys,
      onSelectedItemKeysChange,
      headerActions,
      renderItemActions,
      emptyTitle = "暂无字典项",
      emptyDescription = "当前分类下还没有可展示的字典项。",
      loading,
      error,
      onRefresh,
      className,
      ...rest
    },
    ref
  ) => {
    const fields = React.useMemo<TwoPaneCatalogField<SystemDictionaryTwoPaneItem>[]>(
      () => [
        {
          key: "code",
          label: "编码",
          className: "flex-[1.4_1_12rem]",
          copyText: (item) => item.code,
          render: (item) => <span className="font-mono">{item.code}</span>,
        },
        {
          key: "enabled",
          label: "状态",
          className: "flex-[0.7_1_5rem]",
          render: (item) => (
            <StatusBadge
              status={item.enabled ? "completed" : "isolated"}
              label={item.enabled ? "启用" : "停用"}
              size="sm"
            />
          ),
        },
        {
          key: "source",
          label: "来源",
          className: "flex-[0.7_1_5rem]",
          render: (item) => systemDictionarySourceText(item.source),
        },
        {
          key: "params",
          label: "参数",
          className: "sm:col-span-2 xl:col-span-3",
          layout: "detail",
          render: (item) => <SystemDictionaryParams params={item.params} />,
        },
      ],
      []
    );

    return (
      <TwoPaneCatalog<SystemDictionaryTwoPaneItem>
        ref={ref}
        groups={groups}
        title="字典项"
        groupTitle="字典分类"
        itemTitle="字典项"
        fields={fields}
        initialGroupCode={initialGroupCode}
        selectedGroupCode={selectedGroupCode}
        onSelectedGroupCodeChange={onSelectedGroupCodeChange}
        storageKey={storageKey}
        selectable={selectable}
        selectedItemKeys={selectedItemKeys}
        onSelectedItemKeysChange={onSelectedItemKeysChange}
        headerActions={headerActions}
        renderItemActions={renderItemActions}
        loading={loading}
        error={error}
        onRefresh={onRefresh}
        getItemSearchText={(item) => [
          item.code,
          item.name,
          item.source,
          ...summarizeSystemDictionaryParams(item.params).map((param) => `${param.key}:${param.value}`),
        ]}
        emptyGroupTitle="暂无字典分类"
        emptyGroupDescription="请先确认系统字典接口是否返回分类。"
        emptyItemTitle={emptyTitle}
        emptyItemDescription={emptyDescription}
        className={className}
        {...rest}
      />
    );
  }
);
SystemDictionaryTwoPane.displayName = "SystemDictionaryTwoPane";

function SystemDictionaryParams({ params }: { params?: Record<string, unknown> }) {
  const items = summarizeSystemDictionaryParams(params);

  if (items.length === 0) return <span className="text-muted-foreground">-</span>;

  return (
    <span className="grid gap-2">
      {items.map((param) => (
        <span key={param.key} className="grid gap-1 rounded-md bg-muted px-2 py-1 text-xs sm:grid-cols-[10rem_1fr]">
          <span className="font-mono text-muted-foreground">{param.key}</span>
          <span className="break-all text-foreground">{param.value}</span>
        </span>
      ))}
    </span>
  );
}
