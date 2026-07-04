import * as React from "react";
import { Check, Copy, RotateCw, Search, Settings2, X } from "lucide-react";
import { Button, Card, CardContent, Checkbox, Input } from "../../ui";
import { cn } from "../../lib/utils";
import { EmptyState } from "../EmptyState";
import {
  buildTwoPaneCatalogCopyTitle,
  filterTwoPaneCatalogGroups,
  filterTwoPaneCatalogItems,
  getTwoPaneCatalogSelectedGroup,
  normalizeTwoPaneCatalogFields,
  normalizeTwoPaneCatalogPreference,
  readTwoPaneCatalogFieldText,
  splitTwoPaneCatalogFields,
  summarizeTwoPaneCatalogGroup,
  toggleTwoPaneCatalogSelection,
  twoPaneCatalogText,
  type TwoPaneCatalogField,
  type TwoPaneCatalogGroup,
  type TwoPaneCatalogItemBase,
} from "./two-pane-catalog-logic";

export interface TwoPaneCatalogProps<TItem extends TwoPaneCatalogItemBase = TwoPaneCatalogItemBase>
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children"> {
  groups: TwoPaneCatalogGroup<TItem>[];
  title?: string;
  groupTitle?: string;
  itemTitle?: string;
  fields?: TwoPaneCatalogField<TItem>[];
  initialGroupCode?: string;
  selectedGroupCode?: string;
  onSelectedGroupCodeChange?: (groupCode: string) => void;
  storageKey?: string;
  selectable?: boolean;
  selectedItemKeys?: string[];
  onSelectedItemKeysChange?: (keys: string[]) => void;
  headerActions?: React.ReactNode;
  renderItemActions?: (item: TItem) => React.ReactNode;
  getItemSearchText?: (item: TItem) => readonly unknown[];
  emptyGroupTitle?: string;
  emptyGroupDescription?: string;
  emptyItemTitle?: string;
  emptyItemDescription?: string;
  loading?: boolean;
  error?: React.ReactNode;
  onRefresh?: () => void;
}

/**
 * TwoPaneCatalog — 分类明细两栏组件
 *
 * 层级：Layer 2 业务复合
 * 关联故事：US-M1-011
 * Wave：Wave 3 管理端基础档案
 * 业务约束：只承载分类、明细、筛选、字段显隐、选择和偏好保存，不承载表格型 DataGrid 能力。
 *
 * @example
 *   <TwoPaneCatalog groups={[{ code: "dict", name: "系统字典", items: [] }]} />
 */
function TwoPaneCatalogInner<TItem extends TwoPaneCatalogItemBase>(
  {
    groups,
    title,
    groupTitle = "分类",
    itemTitle = "明细",
    fields = [],
    initialGroupCode,
    selectedGroupCode,
    onSelectedGroupCodeChange,
    storageKey,
    selectable = false,
    selectedItemKeys,
    onSelectedItemKeysChange,
    headerActions,
    renderItemActions,
    getItemSearchText,
    emptyGroupTitle = "暂无分类",
    emptyGroupDescription = "当前没有可展示的分类。",
    emptyItemTitle = "暂无明细",
    emptyItemDescription = "当前分类下没有可展示的明细。",
    loading = false,
    error,
    onRefresh,
    className,
    ...rest
  }: TwoPaneCatalogProps<TItem>,
  ref: React.ForwardedRef<HTMLDivElement>
) {
  const fieldKeys = React.useMemo(() => fields.map((field) => field.key), [fields]);
  const storedPreference = React.useMemo(
    () => readPreference(storageKey, groups, fieldKeys),
    [storageKey, groups, fieldKeys]
  );
  const [internalSelectedCode, setInternalSelectedCode] = React.useState(
    selectedGroupCode ?? initialGroupCode ?? storedPreference.selectedGroupCode
  );
  const [groupQuery, setGroupQuery] = React.useState(storedPreference.groupQuery);
  const [itemQuery, setItemQuery] = React.useState(storedPreference.itemQuery);
  const [visibleFieldKeys, setVisibleFieldKeys] = React.useState(() =>
    normalizeTwoPaneCatalogFields(fields, storedPreference.hiddenFieldKeys)
  );
  const [fieldsOpen, setFieldsOpen] = React.useState(false);
  const [internalSelectedKeys, setInternalSelectedKeys] = React.useState<string[]>([]);
  const [copiedKey, setCopiedKey] = React.useState("");

  const visibleFieldKeySet = React.useMemo(() => new Set(visibleFieldKeys), [visibleFieldKeys]);
  const visibleFieldGroups = React.useMemo(
    () => splitTwoPaneCatalogFields(fields, visibleFieldKeys),
    [fields, visibleFieldKeys]
  );
  const selectedKeys = selectedItemKeys ?? internalSelectedKeys;
  const visibleGroups = filterTwoPaneCatalogGroups(groups, groupQuery);
  const selectedCode = selectedGroupCode ?? internalSelectedCode;
  const selectedGroup = getTwoPaneCatalogSelectedGroup(groups, selectedCode);
  const visibleItems = selectedGroup
    ? filterTwoPaneCatalogItems(selectedGroup.items, itemQuery, getItemSearchText)
    : [];
  const visibleItemKeys = visibleItems.map((item) => item.code);
  const visibleSelectedCount = visibleItemKeys.filter((key) => selectedKeys.includes(key)).length;
  const allVisibleSelected = visibleItemKeys.length > 0 && visibleSelectedCount === visibleItemKeys.length;
  const filtersActive = Boolean(groupQuery || itemQuery);

  React.useEffect(() => {
    setVisibleFieldKeys((current) => {
      const valid = new Set(fieldKeys);
      const next = current.filter((key) => valid.has(key));
      const missingDefaults = fields
        .filter((field) => field.defaultVisible !== false && !next.includes(field.key))
        .map((field) => field.key);
      return [...next, ...missingDefaults];
    });
  }, [fieldKeys, fields]);

  React.useEffect(() => {
    if (!storageKey || typeof window === "undefined") return;
    const hiddenFieldKeys = fieldKeys.filter((key) => !visibleFieldKeySet.has(key));
    window.localStorage.setItem(
      storageKey,
      JSON.stringify({
        selectedGroupCode: selectedGroup?.code ?? "",
        groupQuery,
        itemQuery,
        hiddenFieldKeys,
      })
    );
  }, [fieldKeys, groupQuery, itemQuery, selectedGroup?.code, storageKey, visibleFieldKeySet]);

  function selectGroup(groupCode: string) {
    setInternalSelectedCode(groupCode);
    onSelectedGroupCodeChange?.(groupCode);
  }

  function updateSelectedKeys(next: string[]) {
    setInternalSelectedKeys(next);
    onSelectedItemKeysChange?.(next);
  }

  function toggleItem(itemCode: string, checked: boolean) {
    updateSelectedKeys(toggleTwoPaneCatalogSelection(selectedKeys, itemCode, checked));
  }

  function toggleVisibleItems(checked: boolean) {
    const visible = new Set(visibleItemKeys);
    const remaining = selectedKeys.filter((key) => !visible.has(key));
    updateSelectedKeys(checked ? [...remaining, ...visibleItemKeys] : remaining);
  }

  function toggleField(fieldKey: string, checked: boolean) {
    setVisibleFieldKeys((current) =>
      checked ? Array.from(new Set([...current, fieldKey])) : current.filter((key) => key !== fieldKey)
    );
  }

  async function copyValue(key: string, value: string) {
    if (!value || typeof navigator === "undefined" || !navigator.clipboard) return;
    await navigator.clipboard.writeText(value);
    setCopiedKey(key);
    window.setTimeout(() => setCopiedKey((current) => (current === key ? "" : current)), 1200);
  }

  return (
    <Card ref={ref} className={cn("overflow-hidden rounded-lg shadow-sm", className)} {...rest}>
      <CardContent className="grid gap-0 p-0 md:grid-cols-[minmax(15rem,20rem)_1fr]">
        <aside className="border-b bg-muted/20 p-3 md:border-b-0 md:border-r">
          <div className="mb-2 flex items-center justify-between gap-2">
            <div className="text-xs font-medium text-muted-foreground">{groupTitle}</div>
            <span className="text-xs text-muted-foreground">{visibleGroups.length}/{groups.length}</span>
          </div>
          <div className="relative mb-3">
            <Search className="pointer-events-none absolute left-2 top-2.5 size-4 text-muted-foreground" />
            <Input
              value={groupQuery}
              onChange={(event) => setGroupQuery(event.target.value)}
              placeholder={`筛选${groupTitle}`}
              className="pl-8"
            />
          </div>

          {groups.length === 0 ? (
            <EmptyState title={emptyGroupTitle} description={emptyGroupDescription} className="min-h-64" />
          ) : visibleGroups.length > 0 ? (
            <div className="flex flex-col gap-2">
              {visibleGroups.map((group) => {
                const summary = summarizeTwoPaneCatalogGroup(group);
                const selected = selectedGroup?.code === group.code;
                return (
                  <button
                    key={group.code}
                    type="button"
                    aria-pressed={selected}
                    onClick={() => selectGroup(group.code)}
                    className={cn(
                      "rounded-md border px-3 py-2 text-left transition-colors",
                      selected
                        ? "border-primary bg-primary/10 text-primary"
                        : "border-border bg-background hover:bg-muted/60"
                    )}
                  >
                    <span className="flex min-w-0 items-start justify-between gap-3">
                      <span className="min-w-0">
                        <span className="block truncate text-sm font-medium">{summary.name}</span>
                        <span className="mt-0.5 block truncate font-mono text-xs text-muted-foreground">
                          {summary.code}
                        </span>
                      </span>
                      <span className="shrink-0 rounded-md bg-background px-2 py-1 text-xs font-medium text-foreground">
                        {summary.enabledCount}/{summary.totalCount}
                      </span>
                    </span>
                  </button>
                );
              })}
            </div>
          ) : (
            <EmptyState title="没有匹配分类" description="请调整左侧筛选条件。" className="min-h-64" />
          )}
        </aside>

        <section className="min-w-0 p-4">
          <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
            <div className="min-w-0">
              {title ? <h2 className="truncate text-sm font-semibold">{title}</h2> : null}
              <div className="text-xs text-muted-foreground">
                共 {visibleItems.length} 项{selectable ? `，已选 ${visibleSelectedCount} 项` : ""}
              </div>
            </div>
            <div className="flex flex-wrap items-center justify-end gap-2">
              {headerActions}
              {onRefresh ? (
                <Button type="button" variant="outline" size="sm" onClick={onRefresh} title="刷新">
                  <RotateCw className="size-4" />
                  刷新
                </Button>
              ) : null}
              {fields.length > 0 ? (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setFieldsOpen((current) => !current)}
                  aria-expanded={fieldsOpen}
                  title="字段显示"
                >
                  <Settings2 className="size-4" />
                  字段
                </Button>
              ) : null}
            </div>
          </div>

          <div className="mb-3 flex flex-wrap items-center gap-2">
            {selectable ? (
              <Checkbox
                checked={allVisibleSelected}
                onCheckedChange={(checked) => toggleVisibleItems(checked === true)}
                aria-label="全选或取消明细"
                title={allVisibleSelected ? "取消" : "全选"}
              />
            ) : null}
            <div className="relative min-w-60 flex-1">
              <Search className="pointer-events-none absolute left-2 top-2.5 size-4 text-muted-foreground" />
              <Input
                value={itemQuery}
                onChange={(event) => setItemQuery(event.target.value)}
                placeholder={`筛选${itemTitle}`}
                className="pl-8"
              />
            </div>
          </div>

          {fieldsOpen ? (
            <div className="mb-3 rounded-md border bg-background p-3">
              <div className="mb-2 flex items-center justify-between gap-2">
                <div className="text-xs font-medium text-muted-foreground">字段显示</div>
                <Checkbox
                  checked={visibleFieldKeys.length === fields.length}
                  onCheckedChange={(checked) => setVisibleFieldKeys(checked === true ? fieldKeys : [])}
                  aria-label="全选或取消字段"
                  title={visibleFieldKeys.length === fields.length ? "取消" : "全选"}
                />
              </div>
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
                {fields.map((field) => (
                  <label key={field.key} className="flex items-center gap-2 text-sm">
                    <Checkbox
                      checked={visibleFieldKeySet.has(field.key)}
                      onCheckedChange={(checked) => toggleField(field.key, checked === true)}
                      aria-label={`显示${field.label}`}
                    />
                    <span>{field.label}</span>
                  </label>
                ))}
              </div>
            </div>
          ) : null}

          {filtersActive ? (
            <div className="mb-3 flex flex-wrap items-center justify-between gap-2 rounded-md border border-primary/20 bg-primary/5 px-3 py-2 text-xs text-primary">
              <span>
                已筛选：{groupQuery ? `${groupTitle}=${groupQuery}` : ""}
                {groupQuery && itemQuery ? "，" : ""}
                {itemQuery ? `${itemTitle}=${itemQuery}` : ""}
              </span>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-primary hover:text-primary"
                onClick={() => {
                  setGroupQuery("");
                  setItemQuery("");
                }}
              >
                <X className="size-4" />
                清除
              </Button>
            </div>
          ) : null}

          {loading ? (
            <EmptyState title="加载中" description="正在读取数据。" className="min-h-64" />
          ) : error ? (
            <EmptyState title="加载失败" description={String(error)} className="min-h-64" />
          ) : selectedGroup && visibleItems.length > 0 ? (
            <div className="overflow-hidden rounded-md border bg-background">
              <div className="flex items-center gap-3 border-b bg-muted/40 px-3 py-2 text-xs font-medium text-muted-foreground">
                {selectable ? <span className="w-8 shrink-0" /> : null}
                <span className="min-w-0 flex-[1.1_1_8rem]">名称</span>
                {visibleFieldGroups.columns.map((field) => (
                  <span key={field.key} className={cn("min-w-0 flex-1 truncate", field.className)}>
                    {field.label}
                  </span>
                ))}
                <span className="ml-auto w-40 shrink-0 text-right">操作</span>
              </div>
              <ul className="divide-y">
              {visibleItems.map((item) => (
                <li
                  key={item.code}
                  className={cn(
                    "px-3 py-3 transition-colors hover:bg-muted/30",
                    selectedKeys.includes(item.code) && "bg-primary/5"
                  )}
                >
                  <div className="flex items-center gap-3">
                      {selectable ? (
                        <div className="w-8 shrink-0">
                          <Checkbox
                            checked={selectedKeys.includes(item.code)}
                            onCheckedChange={(checked) => toggleItem(item.code, checked === true)}
                            aria-label={`选择${item.name}`}
                          />
                        </div>
                      ) : null}
                      <div className="min-w-0 flex-[1.1_1_8rem]">
                        <div className="truncate text-sm font-medium">{item.name}</div>
                      </div>
                    {visibleFieldGroups.columns.map((field) => {
                      const copyText = field.copyText?.(item);
                      const fieldCopyKey = `${item.code}:${field.key}`;
                      return (
                        <div key={field.key} className={cn("min-w-0 flex-1 text-sm text-foreground", field.className)}>
                          <CatalogFieldValue
                            item={item}
                            field={field}
                            copyText={copyText}
                            copied={copiedKey === fieldCopyKey}
                            onCopy={() => copyText && void copyValue(fieldCopyKey, copyText)}
                          />
                        </div>
                      );
                    })}
                    {renderItemActions ? (
                      <div className="ml-auto flex w-40 shrink-0 flex-wrap items-center justify-end gap-2">
                        {renderItemActions(item)}
                      </div>
                    ) : (
                      <span className="ml-auto w-40 shrink-0" />
                    )}
                  </div>

                  {visibleFieldGroups.details.length > 0 ? (
                    <dl className={cn("mt-3 grid gap-2", selectable && "pl-11")}>
                      {visibleFieldGroups.details.map((field) => {
                          const copyText = field.copyText?.(item);
                          const fieldCopyKey = `${item.code}:${field.key}`;
                          return (
                            <div
                              key={field.key}
                              className={cn("min-w-0 rounded-md bg-muted/50 px-3 py-2", field.className)}
                            >
                              <dt className="text-xs text-muted-foreground">{field.label}</dt>
                              <dd className="mt-1 min-w-0 text-sm text-foreground">
                                <CatalogFieldValue
                                  item={item}
                                  field={field}
                                  copyText={copyText}
                                  copied={copiedKey === fieldCopyKey}
                                  onCopy={() => copyText && void copyValue(fieldCopyKey, copyText)}
                                />
                              </dd>
                            </div>
                          );
                        })}
                    </dl>
                  ) : null}
                </li>
              ))}
            </ul>
            </div>
          ) : (
            <EmptyState title={emptyItemTitle} description={emptyItemDescription} className="min-h-64" />
          )}
        </section>
      </CardContent>
    </Card>
  );
}

function CatalogFieldValue<TItem extends TwoPaneCatalogItemBase>({
  item,
  field,
  copyText,
  copied,
  onCopy,
}: {
  item: TItem;
  field: TwoPaneCatalogField<TItem>;
  copyText?: string;
  copied: boolean;
  onCopy: () => void;
}) {
  const content = field.render ? field.render(item) : readTwoPaneCatalogFieldText(item, field.key);

  return (
    <>
      {copyText ? (
        <button
          type="button"
          className="inline-flex max-w-full items-center gap-1 text-left hover:text-primary"
          onClick={onCopy}
          title={buildTwoPaneCatalogCopyTitle(copyText)}
        >
          <span className="truncate">{content}</span>
          <Copy className="size-3.5 shrink-0" />
        </button>
      ) : (
        content
      )}
      <span
        className={cn(
          "ml-2 inline-flex w-12 items-center gap-1 text-xs text-primary transition-opacity",
          copied ? "opacity-100" : "opacity-0"
        )}
      >
        <Check className="size-3" />
        已复制
      </span>
    </>
  );
}

const ForwardedTwoPaneCatalog = React.forwardRef(TwoPaneCatalogInner);
ForwardedTwoPaneCatalog.displayName = "TwoPaneCatalog";

export const TwoPaneCatalog = ForwardedTwoPaneCatalog as <
  TItem extends TwoPaneCatalogItemBase = TwoPaneCatalogItemBase,
>(
  props: TwoPaneCatalogProps<TItem> & React.RefAttributes<HTMLDivElement>
) => React.ReactElement | null;

function readPreference<TItem extends TwoPaneCatalogItemBase>(
  storageKey: string | undefined,
  groups: TwoPaneCatalogGroup<TItem>[],
  fieldKeys: string[]
) {
  if (!storageKey || typeof window === "undefined") {
    return normalizeTwoPaneCatalogPreference(null, groups, fieldKeys);
  }
  try {
    const storedValue = JSON.parse(window.localStorage.getItem(storageKey) ?? "null");
    return normalizeTwoPaneCatalogPreference(storedValue, groups, fieldKeys);
  } catch {
    return normalizeTwoPaneCatalogPreference(null, groups, fieldKeys);
  }
}
