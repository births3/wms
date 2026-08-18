import * as React from "react";
import {
  Checkbox,
  DataGrid,
  FormDialogTemplate,
  Input,
  Label,
  ListPageTemplate,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  formatDateTime,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridDeleteAction,
  type DataGridEditAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";

import {
  useBatchCreateLpnContainersMutation,
  useCreateLpnContainerMutation,
  useDeleteLpnContainerMutation,
  useLpnContainersQuery,
  useLpnTypePoliciesQuery,
  useUpdateLpnContainerMutation,
  useUpsertLpnTypePolicyMutation,
  type LpnContainer,
} from "@/features/master-data/lpn-container-queries";
import { lpnContainerMenuItem } from "@/features/master-data/lpn-container-nav";
import { LpnQualityLockDialogs } from "@/pages/master-data/M1LpnQualityLockDialogs";
import { queryString } from "@/lib/query-value";
import { BUTTON_ADD, BUTTON_REFRESH, BUTTON_SAVE, FIELD_KEYWORD } from "@/lib/ui-strings";
import { usePageQueryState } from "@/lib/use-page-query-state";

/**
 * 页面设计契约：列表型；ListPageTemplate + QueryPanel + DataGrid；
 * 新增/批量新增/编辑走 FormDialogTemplate；删除走工具栏软删除；类型策略用第二张 DataGrid，不常驻审计。
 */

const TYPE_OPTIONS = [
  { label: "托盘", value: "pallet" },
  { label: "周转箱", value: "tote" },
  { label: "出库箱", value: "outbound_box" },
  { label: "保温箱", value: "insulated_box" },
  { label: "盲标签", value: "blind_label" },
];

const STATUS_LABELS: Record<string, string> = {
  idle: "空闲",
  in_use: "在用",
  in_transit: "在途",
  recycling: "回收中",
  shipped: "已出库",
  disabled: "已停用",
};

const LOCK_LABELS: Record<string, string> = {
  qualified: "合格",
  quarantine: "隔离",
  rejected: "不合格",
};

export const lpnQueryFields: QueryPanelField[] = [
  { key: "keyword", label: FIELD_KEYWORD, type: "text", placeholder: "LPN / 类型" },
  { key: "containerType", label: "容器类型", type: "select", options: TYPE_OPTIONS },
  {
    key: "status",
    label: "状态",
    type: "select",
    options: [
      { label: "空闲", value: "idle" },
      { label: "在用", value: "in_use" },
      { label: "在途", value: "in_transit" },
      { label: "回收中", value: "recycling" },
      { label: "已出库", value: "shipped" },
      { label: "已停用", value: "disabled" },
    ],
  },
];
export const lpnCoreQueryFieldKeys = ["keyword", "containerType"];
const LPN_BATCH_CREATE_MAX_COUNT = 100;

function parseCapacity(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const value = Number(trimmed);
  return Number.isFinite(value) && value > 0 ? Math.trunc(value) : null;
}

function parseBatchCount(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const value = Number(trimmed);
  return Number.isInteger(value) && value >= 1 && value <= LPN_BATCH_CREATE_MAX_COUNT ? value : null;
}

export function M1LpnContainerPage() {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } = usePageQueryState<QueryPanelValue>(
    () => ({ keyword: "", containerType: "", status: "" }),
  );
  const listQuery = useLpnContainersQuery({
    keyword: queryString(appliedQuery.keyword),
    containerType: queryString(appliedQuery.containerType),
    status: queryString(appliedQuery.status),
  });
  const createMutation = useCreateLpnContainerMutation();
  const batchCreateMutation = useBatchCreateLpnContainersMutation();
  const updateMutation = useUpdateLpnContainerMutation();
  const deleteMutation = useDeleteLpnContainerMutation();
  const policiesQuery = useLpnTypePoliciesQuery();
  const upsertPolicy = useUpsertLpnTypePolicyMutation();
  const [createOpen, setCreateOpen] = React.useState(false);
  const [batchOpen, setBatchOpen] = React.useState(false);
  const [editOpen, setEditOpen] = React.useState(false);
  const [containerType, setContainerType] = React.useState("pallet");
  const [capacityText, setCapacityText] = React.useState("");
  const [batchCountText, setBatchCountText] = React.useState("10");
  const [batchCountError, setBatchCountError] = React.useState<string | null>(null);
  const [lockOpen, setLockOpen] = React.useState(false);
  const [changeLockOpen, setChangeLockOpen] = React.useState(false);
  const [releaseOpen, setReleaseOpen] = React.useState(false);
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const rows = listQuery.data ?? [];
  const selected = rows.find((row) => row.id === selectedId) ?? null;

  const policyRows = TYPE_OPTIONS.map((type) => {
    const policy = policiesQuery.data?.find((item) => item.container_type === type.value);
    return {
      container_type: type.value,
      label: type.label,
      allow_mix_batch: policy?.allow_mix_batch ?? false,
      allow_mix_sku: policy?.allow_mix_sku ?? false,
    };
  });

  const columns: DataGridColumn<LpnContainer>[] = [
    { key: "lpn_code", header: "LPN", mono: true, width: 220, copyValue: (row) => row.lpn_code, render: (row) => row.lpn_code },
    { key: "container_type", header: "类型", width: 140, render: (row) => TYPE_OPTIONS.find((item) => item.value === row.container_type)?.label ?? row.container_type },
    { key: "status", header: "状态", width: 120, render: (row) => STATUS_LABELS[row.status] ?? row.status },
    {
      key: "current_lock_category",
      header: "质量锁",
      width: 100,
      render: (row) => LOCK_LABELS[row.current_lock_category ?? "qualified"] ?? row.current_lock_category ?? "合格",
    },
    { key: "location_id", header: "当前库位", width: 220, mono: true, render: (row) => row.location_id ?? "—" },
    { key: "created_at", header: "创建时间", width: 180, render: (row) => formatDateTime(row.created_at) },
    { key: "capacity_cm3", header: "容量 cm³", width: 120, render: (row) => row.capacity_cm3 ?? "—" },
  ];

  const policyColumns: DataGridColumn<(typeof policyRows)[number]>[] = [
    { key: "label", header: "容器类型", width: 160, render: (row) => row.label },
    {
      key: "allow_mix_batch",
      header: "混批",
      width: 100,
      render: (row) => (
        <Checkbox
          checked={row.allow_mix_batch}
          aria-label={`${row.label}混批`}
          onCheckedChange={(checked) => {
            void upsertPolicy.mutateAsync({
              container_type: row.container_type,
              allow_mix_batch: checked === true,
              allow_mix_sku: row.allow_mix_sku,
            });
          }}
        />
      ),
    },
    {
      key: "allow_mix_sku",
      header: "混品",
      width: 100,
      render: (row) => (
        <Checkbox
          checked={row.allow_mix_sku}
          aria-label={`${row.label}混品`}
          onCheckedChange={(checked) => {
            void upsertPolicy.mutateAsync({
              container_type: row.container_type,
              allow_mix_batch: row.allow_mix_batch,
              allow_mix_sku: checked === true,
            });
          }}
        />
      ),
    },
  ];

  const refreshAction: DataGridRefreshAction = {
    label: BUTTON_REFRESH,
    onClick: () => {
      void listQuery.refetch();
    },
  };
  const createAction: DataGridCreateAction = {
    label: BUTTON_ADD,
    onClick: () => {
      setContainerType("pallet");
      setCapacityText("");
      setCreateOpen(true);
    },
  };
  const batchCreateAction: DataGridToolbarAction = {
    key: "batch-create",
    label: "批量新增",
    description: "按类型一次生成多个空闲容器",
    onClick: () => {
      setContainerType("pallet");
      setCapacityText("");
      setBatchCountText("10");
      setBatchCountError(null);
      setBatchOpen(true);
    },
  };
  const applyLockAction: DataGridToolbarAction = {
    key: "quality-lock",
    label: "加锁",
    description: "对在用容器施加隔离/不合格锁",
    disabled: ({ selectedRowKeys }) => selectedRowKeys.length !== 1,
    onClick: ({ selectedRowKeys }) => {
      setSelectedId(selectedRowKeys[0] ?? null);
      setLockOpen(true);
    },
  };
  const changeLockAction: DataGridToolbarAction = {
    key: "quality-lock-change",
    label: "换原因",
    description: "更换已加锁容器的原因或类别",
    disabled: ({ selectedRowKeys }) => selectedRowKeys.length !== 1,
    onClick: ({ selectedRowKeys }) => {
      setSelectedId(selectedRowKeys[0] ?? null);
      setChangeLockOpen(true);
    },
  };
  const releaseLockAction: DataGridToolbarAction = {
    key: "quality-lock-release",
    label: "解锁",
    description: "解除容器质量锁",
    disabled: ({ selectedRowKeys }) => selectedRowKeys.length !== 1,
    onClick: ({ selectedRowKeys }) => {
      setSelectedId(selectedRowKeys[0] ?? null);
      setReleaseOpen(true);
    },
  };
  const editAction: DataGridEditAction = {
    label: "编辑",
    description: "编辑选中容器容量",
    disabled: ({ selectedRowKeys }) => selectedRowKeys.length !== 1 || updateMutation.isPending,
    onClick: ({ selectedRowKeys }) => {
      const row = rows.find((item) => item.id === selectedRowKeys[0]);
      if (!row || row.status === "disabled") return;
      setSelectedId(row.id);
      setCapacityText(row.capacity_cm3 == null ? "" : String(row.capacity_cm3));
      setEditOpen(true);
    },
  };
  const deleteAction: DataGridDeleteAction = {
    label: "删除",
    description: "软删除选中空闲容器",
    disabled: ({ selectedRowKeys }) => selectedRowKeys.length !== 1 || deleteMutation.isPending,
    onClick: ({ selectedRowKeys }) => {
      const row = rows.find((item) => item.id === selectedRowKeys[0]);
      if (!row || !window.confirm(`确认停用容器「${row.lpn_code}」？停用后列表默认不再显示。`)) return;
      void deleteMutation.mutateAsync(row.id).then(() => setSelectedId(null));
    },
  };

  return (
    <ListPageTemplate
      header={{ title: lpnContainerMenuItem.title, subtitle: "US-M1-004a 容器主档 / 类型策略" }}
      notice={
        listQuery.error
          ? { kind: "error", text: listQuery.error.message }
          : createMutation.error
            ? { kind: "error", text: createMutation.error.message }
            : batchCreateMutation.error
              ? { kind: "error", text: batchCreateMutation.error.message }
            : updateMutation.error
              ? { kind: "error", text: updateMutation.error.message }
              : deleteMutation.error
                ? { kind: "error", text: deleteMutation.error.message }
                : policiesQuery.error
                  ? { kind: "error", text: policiesQuery.error.message }
                  : upsertPolicy.error
                    ? { kind: "error", text: upsertPolicy.error.message }
                    : null
      }
      queryFields={lpnQueryFields}
      coreQueryFieldKeys={lpnCoreQueryFieldKeys}
      queryValue={draftQuery}
      onQueryValueChange={setDraftQuery}
      onQuery={() => applyQuery(draftQuery)}
      onReset={resetQuery}
      loading={listQuery.isPending || listQuery.isFetching}
      gridProps={{
        columns,
        data: rows,
        rowKey: (row) => row.id,
        storageKey: "m1-lpn-containers",
        emptyTitle: "暂无容器",
        selectable: true,
        selectedRowKeys: selectedId ? [selectedId] : [],
        onSelectedRowKeysChange: (keys) => setSelectedId(keys.at(-1) ?? null),
        refreshAction,
        createAction,
        editAction,
        deleteAction,
        toolbarActions: [batchCreateAction, applyLockAction, changeLockAction, releaseLockAction],
      }}
      dialogs={
        <>
        <FormDialogTemplate
          open={createOpen}
          onOpenChange={setCreateOpen}
          title="创建容器"
          description="LPN 由服务端按容器类型编号规则生成，客户端不传码。"
          submitLabel={BUTTON_SAVE}
          loading={createMutation.isPending}
          errorMessage={createMutation.error?.message}
          onSubmit={(event) => {
            event.preventDefault();
            void createMutation
              .mutateAsync({ container_type: containerType, capacity_cm3: parseCapacity(capacityText) })
              .then(() => setCreateOpen(false));
          }}
        >
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="lpn-container-type">类型</Label>
              <Select value={containerType} onValueChange={setContainerType}>
                <SelectTrigger id="lpn-container-type" aria-label="类型">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {TYPE_OPTIONS.map((item) => (
                    <SelectItem key={item.value} value={item.value}>
                      {item.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="lpn-container-capacity">容量 cm³</Label>
              <Input
                id="lpn-container-capacity"
                inputMode="numeric"
                value={capacityText}
                onChange={(event) => setCapacityText(event.target.value)}
                placeholder="可选"
                aria-label="容量 cm³"
              />
            </div>
          </div>
        </FormDialogTemplate>
        <FormDialogTemplate
          open={batchOpen}
          onOpenChange={setBatchOpen}
          title="批量新增容器"
          description={`同一类型一次生成多个空闲 LPN，数量 1-${LPN_BATCH_CREATE_MAX_COUNT}。`}
          submitLabel={BUTTON_SAVE}
          loading={batchCreateMutation.isPending}
          errorMessage={batchCreateMutation.error?.message ?? batchCountError}
          onSubmit={(event) => {
            event.preventDefault();
            const count = parseBatchCount(batchCountText);
            if (count == null) {
              setBatchCountError(`数量必须为 1-${LPN_BATCH_CREATE_MAX_COUNT} 的整数`);
              return;
            }
            setBatchCountError(null);
            void batchCreateMutation
              .mutateAsync({
                container_type: containerType,
                capacity_cm3: parseCapacity(capacityText),
                count,
              })
              .then(() => setBatchOpen(false));
          }}
        >
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="lpn-batch-type">类型</Label>
              <Select value={containerType} onValueChange={setContainerType}>
                <SelectTrigger id="lpn-batch-type" aria-label="类型">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {TYPE_OPTIONS.map((item) => (
                    <SelectItem key={item.value} value={item.value}>
                      {item.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="lpn-batch-count">数量</Label>
              <Input
                id="lpn-batch-count"
                inputMode="numeric"
                value={batchCountText}
                onChange={(event) => setBatchCountText(event.target.value)}
                aria-label="批量数量"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="lpn-batch-capacity">容量 cm³</Label>
              <Input
                id="lpn-batch-capacity"
                inputMode="numeric"
                value={capacityText}
                onChange={(event) => setCapacityText(event.target.value)}
                placeholder="可选"
                aria-label="容量 cm³"
              />
            </div>
          </div>
        </FormDialogTemplate>
        <FormDialogTemplate
          open={editOpen}
          onOpenChange={setEditOpen}
          title="编辑容器"
          description={selected ? `LPN ${selected.lpn_code}，类型不可改。` : "请先选择容器。"}
          submitLabel={BUTTON_SAVE}
          loading={updateMutation.isPending}
          errorMessage={updateMutation.error?.message}
          onSubmit={(event) => {
            event.preventDefault();
            if (!selected) return;
            void updateMutation
              .mutateAsync({
                id: selected.id,
                body: { status: null, location_id: null, capacity_cm3: parseCapacity(capacityText) },
              })
              .then(() => setEditOpen(false));
          }}
        >
          <div className="space-y-2">
            <Label htmlFor="lpn-edit-capacity">容量 cm³</Label>
            <Input
              id="lpn-edit-capacity"
              inputMode="numeric"
              value={capacityText}
              onChange={(event) => setCapacityText(event.target.value)}
              placeholder="可选"
              aria-label="容量 cm³"
            />
          </div>
        </FormDialogTemplate>
        <LpnQualityLockDialogs
          selected={selected}
          lockOpen={lockOpen}
          changeOpen={changeLockOpen}
          releaseOpen={releaseOpen}
          onLockOpenChange={setLockOpen}
          onChangeOpenChange={setChangeLockOpen}
          onReleaseOpenChange={setReleaseOpen}
        />
        </>
      }
    >
      <DataGrid
        columns={policyColumns}
        data={policyRows}
        rowKey={(row) => row.container_type}
        storageKey="m1-lpn-type-policies"
        caption="类型策略（默认禁止混批/混品）"
      />
    </ListPageTemplate>
  );
}
