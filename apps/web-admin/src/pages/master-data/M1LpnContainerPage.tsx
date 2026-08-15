import * as React from "react";
import {
  Checkbox,
  DataGrid,
  FormDialogTemplate,
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
  type DataGridRefreshAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";

import {
  useCreateLpnContainerMutation,
  useLpnContainersQuery,
  useLpnTypePoliciesQuery,
  useUpsertLpnTypePolicyMutation,
  type LpnContainer,
} from "@/features/master-data/lpn-container-queries";
import { queryString } from "@/lib/query-value";
import { BUTTON_ADD, BUTTON_REFRESH, BUTTON_SAVE, FIELD_KEYWORD } from "@/lib/ui-strings";
import { usePageQueryState } from "@/lib/use-page-query-state";

/**
 * 页面设计契约：列表型；ListPageTemplate + QueryPanel + DataGrid；
 * 新增走 FormDialogTemplate；类型策略用第二张 DataGrid，不常驻审计。
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
    ],
  },
];
export const lpnCoreQueryFieldKeys = ["keyword", "containerType"];

export function M1LpnContainerPage() {
  const listQuery = useLpnContainersQuery();
  const createMutation = useCreateLpnContainerMutation();
  const policiesQuery = useLpnTypePoliciesQuery();
  const upsertPolicy = useUpsertLpnTypePolicyMutation();
  const [createOpen, setCreateOpen] = React.useState(false);
  const [containerType, setContainerType] = React.useState("pallet");
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } = usePageQueryState<QueryPanelValue>(
    () => ({ keyword: "", containerType: "", status: "" }),
  );

  const keyword = queryString(appliedQuery.keyword).toLowerCase();
  const typeFilter = queryString(appliedQuery.containerType);
  const statusFilter = queryString(appliedQuery.status);
  const rows = (listQuery.data ?? []).filter((row) => {
    const matchesKeyword = !keyword || `${row.lpn_code} ${row.container_type}`.toLowerCase().includes(keyword);
    const matchesType = !typeFilter || row.container_type === typeFilter;
    const matchesStatus = !statusFilter || row.status === statusFilter;
    return matchesKeyword && matchesType && matchesStatus;
  });

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
    onClick: () => setCreateOpen(true),
  };

  return (
    <ListPageTemplate
      header={{ title: "M1 容器管理", subtitle: "US-M1-004a 容器主档 / 类型策略" }}
      notice={
        listQuery.error
          ? { kind: "error", text: listQuery.error.message }
          : createMutation.error
            ? { kind: "error", text: createMutation.error.message }
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
        refreshAction,
        createAction,
      }}
      dialogs={
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
            void createMutation.mutateAsync({ container_type: containerType, capacity_cm3: null }).then(() => setCreateOpen(false));
          }}
        >
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
        </FormDialogTemplate>
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
