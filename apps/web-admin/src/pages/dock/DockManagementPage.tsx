import * as React from "react";
import {
  Button,
  DataGrid,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  PageHeader,
  QueryPanel,
  StatusBadge,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridDeleteAction,
  type DataGridEditAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { CalendarClock, CalendarPlus } from "lucide-react";

import { useMasterDataRowsQuery } from "@/features/master-data/master-data-queries/queries";
import {
  useCreateDockMutation,
  useCreateDockAppointmentMutation,
  useUpdateDockAppointmentMutation,
  useCancelDockAppointmentMutation,
  useDockAppointmentsQuery,
  useDeleteDockMutation,
  useDocksQuery,
  useImportDocksMutation,
  useUpdateDockMutation,
  type Dock, type DockAppointment,
} from "@/features/dock/dock-queries";
import {
  DockAppointmentCreateDialog,
  type DockAppointmentForm,
} from "@/pages/dock/DockAppointmentCreateDialog";
import { DockAppointmentChangeDialog } from "@/pages/dock/DockAppointmentChangeDialog";
import { DockOccupancyBoard } from "@/pages/dock/DockOccupancyBoard";
import { formatDateTime } from "@/lib/format";
import { queryString } from "@/lib/query-value";
import { readSpreadsheetRows } from "@/lib/spreadsheet";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";

/**
 * 页面设计契约：列表型；主信息载体为 QueryPanel + DataGrid；标准动作放在 DataGrid；
 * 新增与状态维护通过 Dialog；不常驻预约明细、轨迹、审计或动作表单。
 */

export const dockQueryFields: QueryPanelField[] = [
  { key: "warehouseId", label: "仓库", type: "select", options: [] },
  { key: "keyword", label: "关键字", type: "text", placeholder: "月台编号 / 位置说明" },
  {
    key: "status",
    label: "状态",
    type: "multiSelect",
    options: [
      { label: "启用", value: "active" },
      { label: "维护中", value: "maintenance" },
      { label: "停用", value: "disabled" },
    ],
  },
];

export const dockCoreQueryFieldKeys = ["warehouseId", "keyword"];

type DockForm = {
  dockCode: string;
  dockType: string;
  temperatureZone: string;
  locationDescription: string;
};

type DockEditForm = {
  status: string;
  maintenanceRecoveryDate: string;
};

const dockTypeOptions = [
  { label: "收货", value: "receiving" },
  { label: "发货", value: "shipping" },
  { label: "收发共用", value: "both" },
];
const temperatureZoneOptions = [
  { label: "常温", value: "normal" },
  { label: "冷藏", value: "cold" },
  { label: "冷冻", value: "frozen" },
  { label: "冷链", value: "cold_chain" },
];
const statusOptions = [
  { label: "启用", value: "active" },
  { label: "维护中", value: "maintenance" },
  { label: "停用", value: "disabled" },
];

export function DockManagementPage() {
  const warehousesQuery = useMasterDataRowsQuery("m1-warehouses");
  const warehouses = warehousesQuery.data ?? [];
  const { draftQuery, setDraftQuery, appliedQuery, setAppliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(
      () => ({ ...defaultQuery(), warehouseId: warehouses[0]?.id ?? "" }),
      queryValueFromUnknown,
    );
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [createOpen, setCreateOpen] = React.useState(false);
  const [appointmentOpen, setAppointmentOpen] = React.useState(false);
  const [editOpen, setEditOpen] = React.useState(false);
  const [createForm, setCreateForm] = React.useState<DockForm>(emptyDockForm);
  const [appointmentForm, setAppointmentForm] = React.useState<DockAppointmentForm>(() => emptyDockAppointmentForm());
  const [appointmentValidationError, setAppointmentValidationError] = React.useState<string | null>(null);
  const [changeValidationError, setChangeValidationError] = React.useState<string | null>(null);
  const [appointments, setAppointments] = React.useState<DockAppointment[]>([]);
  const [appointmentRecordsOpen, setAppointmentRecordsOpen] = React.useState(false);
  const changeDialog = useDialogState<DockAppointment>();
  const cancelDialog = useDialogState<DockAppointment>();
  const [changeForm, setChangeForm] = React.useState<DockAppointmentForm>(() => emptyDockAppointmentForm());
  const [editForm, setEditForm] = React.useState<DockEditForm>(emptyDockEditForm);
  const [notice, setNotice] = React.useState<string | null>(null);
  const importInputRef = React.useRef<HTMLInputElement>(null);
  const warehouseId = queryString(appliedQuery.warehouseId) || null;
  const docksQuery = useDocksQuery(warehouseId);
  const appointmentsQuery = useDockAppointmentsQuery(warehouseId);
  const createDock = useCreateDockMutation();
  const createAppointment = useCreateDockAppointmentMutation();
  const updateAppointment = useUpdateDockAppointmentMutation();
  const cancelAppointment = useCancelDockAppointmentMutation();
  const importDocks = useImportDocksMutation();
  const updateDock = useUpdateDockMutation();
  const deleteDock = useDeleteDockMutation();
  const selectedDock = (docksQuery.data ?? []).find((dock) => dock.id === selectedId) ?? null;
  const selectedWarehouse = warehouses.find((warehouse) => warehouse.id === warehouseId);
  const warehouseLabel = selectedWarehouse ? `${selectedWarehouse.code} ${selectedWarehouse.name}` : "";

  React.useEffect(() => {
    if (warehouses.length === 0 || warehouses.some((warehouse) => warehouse.id === queryString(draftQuery.warehouseId))) return;
    const nextWarehouseId = warehouses[0].id;
    setDraftQuery((current) => ({ ...current, warehouseId: nextWarehouseId }));
    setAppliedQuery((current) => ({ ...current, warehouseId: nextWarehouseId }));
  }, [draftQuery.warehouseId, setAppliedQuery, setDraftQuery, warehouses]);

  const warehouseOptions = React.useMemo(
    () => warehouses.map((warehouse) => ({ label: `${warehouse.code} ${warehouse.name}`, value: warehouse.id })),
    [warehouses],
  );
  const docks = React.useMemo(
    () => filterDocks(docksQuery.data ?? [], appliedQuery),
    [appliedQuery, docksQuery.data],
  );
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(dockQueryFields, appliedQuery),
    [appliedQuery],
  );
  const appointmentColumns: DataGridColumn<DockAppointment>[] = [
    { key: "appointment_no", header: "预约编号", width: 170, minWidth: 150, mono: true, copyValue: (row) => row.appointment_no, render: (row) => <span className="font-mono">{row.appointment_no} · v{row.version}</span> },
    { key: "window_start_at", header: "时间窗", width: 250, minWidth: 220, render: (row) => `${formatDateTime(row.window_start_at)} - ${formatDateTime(row.window_end_at)}` },
    { key: "document_no", header: "关联单据", width: 180, minWidth: 150, render: (row) => row.document_no },
    { key: "vehicle_plate_no", header: "车辆 / 司机", width: 190, minWidth: 160, render: (row) => `${row.vehicle_plate_no || "未填车牌"} / ${row.driver_name}` },
    { key: "status", header: "状态", width: 120, minWidth: 100, render: (row) => <StatusBadge status={row.status === "cancelled" ? "isolated" : row.status === "arrived" ? "completed" : "pending"} label={appointmentStatusLabel(row.status)} size="sm" /> },
    { key: "actions", header: "操作", width: 180, minWidth: 160, render: (row) => <div className="flex gap-2"><Button type="button" size="sm" variant="outline" disabled={row.status === "cancelled" || row.status === "arrived"} onClick={(event) => { event.stopPropagation(); setChangeForm(appointmentFormFor(row)); setChangeValidationError(null); updateAppointment.reset(); changeDialog.openWith(row); }}>变更</Button><Button type="button" size="sm" variant="outline" disabled={row.status === "cancelled" || row.status === "arrived" || cancelAppointment.isPending} onClick={(event) => { event.stopPropagation(); cancelAppointment.reset(); cancelDialog.openWith(row); }}>取消</Button></div> },
  ];

  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新月台列表",
    disabled: docksQuery.isFetching || !warehouseId,
    onClick: () => {
      void docksQuery.refetch().then((result) => {
        if (!result.error) setNotice("月台列表已刷新");
      });
    },
  };
  const createAction: DataGridCreateAction = {
    label: "新增",
    description: "新增月台档案",
    disabled: !warehouseId || createDock.isPending,
    onClick: () => {
      createDock.reset();
      setCreateForm(emptyDockForm());
      setCreateOpen(true);
    },
  };
  const editAction: DataGridEditAction = {
    label: "编辑",
    description: "编辑选中月台状态",
    disabled: ({ selectedRowKeys }) => selectedRowKeys.length !== 1 || updateDock.isPending,
    onClick: ({ selectedRowKeys }) => {
      const dock = (docksQuery.data ?? []).find((item) => item.id === selectedRowKeys[0]);
      if (!dock) return;
      updateDock.reset();
      setSelectedId(dock.id);
      setEditForm(editFormFor(dock));
      setEditOpen(true);
    },
  };
  const deleteAction: DataGridDeleteAction = {
    label: "删除",
    description: "删除选中月台；存在关联预约时拒绝",
    disabled: ({ selectedRowKeys }) => selectedRowKeys.length !== 1 || deleteDock.isPending,
    onClick: ({ selectedRowKeys }) => {
      const dock = (docksQuery.data ?? []).find((item) => item.id === selectedRowKeys[0]);
      if (!dock || !window.confirm(`确认删除月台「${dock.dock_code}」？`)) return;
      void deleteDock.mutateAsync(dock.id).then(async () => {
        setSelectedId(null);
        await docksQuery.refetch();
        setNotice(`${dock.dock_code} 已删除`);
      }).catch(() => undefined);
    },
  };
  const importAction: DataGridToolbarAction = {
    key: "import",
    label: "导入",
    description: "导入 Excel 月台档案",
    icon: <span aria-hidden>⇧</span>,
    disabled: !warehouseId || importDocks.isPending,
    onClick: () => importInputRef.current?.click(),
  };
  const appointmentAction: DataGridToolbarAction = {
    key: "appointment",
    label: "预约",
    description: "为选中的启用月台创建预约",
    icon: <CalendarPlus className="size-4" aria-hidden />,
    disabled: !selectedDock || selectedDock.status !== "active" || createAppointment.isPending,
    onClick: () => {
      if (!selectedDock || selectedDock.status !== "active") return;
      createAppointment.reset();
      setAppointmentValidationError(null);
      setAppointmentForm(emptyDockAppointmentForm());
      setAppointmentOpen(true);
    },
  };
  const appointmentRecordsAction: DataGridToolbarAction = {
    key: "appointment-records",
    label: "预约记录",
    description: "查看本次会话已创建预约",
    icon: <CalendarClock className="size-4" aria-hidden />,
    disabled: appointments.length === 0,
    onClick: () => setAppointmentRecordsOpen(true),
  };

  async function submitCreate(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!warehouseId) return;
    try {
      await createDock.mutateAsync({
        warehouse_id: warehouseId,
        dock_code: createForm.dockCode.trim(),
        dock_type: createForm.dockType,
        temperature_zone: createForm.temperatureZone,
        location_description: createForm.locationDescription.trim() || null,
      });
      await docksQuery.refetch();
      setCreateOpen(false);
      setNotice("月台已新增");
    } catch {
      // 保留弹窗，错误信息显示在表单底部，便于修正后重试。
    }
  }

  async function submitEdit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedDock) return;
    try {
      await updateDock.mutateAsync({
        id: selectedDock.id,
        body: {
          status: editForm.status,
          maintenance_recovery_at: editForm.status === "maintenance"
            ? dateToIso(editForm.maintenanceRecoveryDate)
            : null,
        },
      });
      await docksQuery.refetch();
      setEditOpen(false);
      setNotice(`${selectedDock.dock_code} 状态已保存`);
    } catch {
      // 保留弹窗，错误信息显示在表单底部，便于修正后重试。
    }
  }

  async function submitAppointment(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!warehouseId || !selectedDock || selectedDock.status !== "active") return;
    const windowStartAt = dateTimeLocalToIso(appointmentForm.windowStartAt);
    const windowEndAt = dateTimeLocalToIso(appointmentForm.windowEndAt);
    if (!windowStartAt || !windowEndAt || new Date(windowEndAt) <= new Date(windowStartAt)) {
      setAppointmentValidationError("预约结束时间必须晚于开始时间");
      return;
    }
    setAppointmentValidationError(null);
    try {
      const created = await createAppointment.mutateAsync({
        appointment_no: appointmentForm.appointmentNo.trim(),
        dock_id: selectedDock.id,
        document_no: appointmentForm.documentNo.trim(),
        document_type: appointmentForm.documentType,
        driver_name: appointmentForm.driverName.trim(),
        driver_phone: appointmentForm.driverPhone.trim(),
        vehicle_plate_no: appointmentForm.vehiclePlateNo.trim() || null,
        vehicle_type: appointmentForm.vehicleType,
        warehouse_id: warehouseId,
        window_end_at: windowEndAt,
        window_start_at: windowStartAt,
      });
      setAppointments((current) => [created, ...current]);
      setAppointmentOpen(false);
      setNotice(`预约 ${appointmentForm.appointmentNo.trim()} 已创建`);
    } catch {
      // 保留弹窗，错误信息展示在表单中，便于修正后重试。
    }
  }

  async function handleImport(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file || !warehouseId) return;
    try {
      const rows = await readSpreadsheetRows(file);
      const docks = importDockRows(rows, warehouseId);
      const imported = await importDocks.mutateAsync({ warehouse_id: warehouseId, docks });
      await docksQuery.refetch();
      setSelectedId(null);
      setNotice(`已导入 ${imported.length} 个月台`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "导入月台失败");
    }
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="M1 月台管理"
        subtitle="维护仓库月台、作业类型、温区和维护状态"
        actions={notice ? <span className="self-center text-sm text-muted-foreground" role="status">{notice}</span> : undefined}
      />
      <input ref={importInputRef} className="hidden" type="file" accept=".xlsx,.csv" onChange={(event) => void handleImport(event)} />
      <QueryPanel
        fields={dockQueryFields}
        fieldOptions={{ warehouseId: warehouseOptions }}
        defaultVisibleFieldKeys={dockCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => {
          applyDockQuery(draftQuery);
          setNotice("月台列表已查询");
        }}
        onReset={clearDockQuery}
      />
      {warehousesQuery.error && <ErrorNotice message={warehousesQuery.error.message} />}
      {docksQuery.error && <ErrorNotice message={docksQuery.error.message} />}
      {createDock.error && <ErrorNotice message={createDock.error.message} />}
      {updateDock.error && <ErrorNotice message={updateDock.error.message} />}
      <DataGrid
        columns={dockColumns}
        data={docks}
        rowKey={(row) => row.id}
        caption={docksQuery.isPending ? "加载月台..." : undefined}
        emptyTitle={warehouseId ? "暂无月台档案" : "请选择仓库"}
        emptyDescription="选择仓库后查看或新增月台"
        storageKey="m1-docks-datagrid"
        exportFileBaseName="M1 月台管理"
        refreshAction={refreshAction}
        createAction={createAction}
        editAction={editAction}
        deleteAction={deleteAction}
        toolbarActions={[importAction, appointmentAction, appointmentRecordsAction]}
        selectedRowKeys={selectedId ? [selectedId] : []}
        onSelectedRowKeysChange={(keys) => setSelectedId(keys.at(-1) ?? null)}
        onRowClick={(row) => setSelectedId(row.id)}
        selectable
        tableClassName="min-w-[1280px]"
        queryState={appliedQuery}
        querySummaryItems={querySummaryItems}
        onApplyQueryState={applyDockQuery}
        onClearQueryState={clearDockQuery}
      />
      <DockOccupancyBoard
        warehouseSelected={Boolean(warehouseId)}
        warehouseLabel={warehouseLabel}
        docks={docksQuery.data ?? []}
        appointments={appointmentsQuery.data ?? []}
        loading={docksQuery.isPending || appointmentsQuery.isPending}
        error={appointmentsQuery.error?.message}
      />
      <DockCreateDialog
        open={createOpen}
        form={createForm}
        pending={createDock.isPending}
        onOpenChange={setCreateOpen}
        onFormChange={setCreateForm}
        onSubmit={submitCreate}
        errorMessage={createDock.error?.message}
      />
      <DockEditDialog
        open={editOpen}
        dock={selectedDock}
        form={editForm}
        pending={updateDock.isPending}
        onOpenChange={setEditOpen}
        onFormChange={setEditForm}
        onSubmit={submitEdit}
        errorMessage={updateDock.error?.message}
      />
      <DockAppointmentCreateDialog
        open={appointmentOpen}
        dock={selectedDock}
        warehouseLabel={warehouseLabel}
        form={appointmentForm}
        pending={createAppointment.isPending}
        onOpenChange={setAppointmentOpen}
        onFormChange={(next) => {
          setAppointmentValidationError(null);
          setAppointmentForm(next);
        }}
        onSubmit={submitAppointment}
        errorMessage={appointmentValidationError ?? createAppointment.error?.message}
      />
      <Dialog open={appointmentRecordsOpen} onOpenChange={(next) => !updateAppointment.isPending && !cancelAppointment.isPending && setAppointmentRecordsOpen(next)}>
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-6xl">
          <DialogHeader><DialogTitle>本次会话预约记录</DialogTitle><DialogDescription>仅展示本次会话通过当前页面创建或变更的预约，不代表历史预约列表。</DialogDescription></DialogHeader>
          <DataGrid columns={appointmentColumns} data={appointments} rowKey={(row) => row.id} emptyTitle="暂无预约记录" emptyDescription="创建预约后将在这里展示" storageKey="m1-dock-session-appointments" tableClassName="min-w-[1060px]" />
          <DialogFooter><DialogClose asChild><Button type="button" variant="outline">关闭</Button></DialogClose></DialogFooter>
        </DialogContent>
      </Dialog>
      <DockAppointmentChangeDialog
        open={changeDialog.open}
        appointment={changeDialog.target}
        docks={docksQuery.data ?? []}
        form={changeForm}
        pending={updateAppointment.isPending}
        onOpenChange={(next) => { if (!next) changeDialog.close(); }}
        onFormChange={(next) => {
          setChangeValidationError(null);
          setChangeForm(next);
        }}
        onSubmit={submitAppointmentChange}
        errorMessage={changeValidationError ?? updateAppointment.error?.message}
      />
      <Dialog open={cancelDialog.open} onOpenChange={(next) => !cancelAppointment.isPending && !next && cancelDialog.close()}>
        <DialogContent className="sm:max-w-md"><DialogHeader><DialogTitle>取消月台预约</DialogTitle><DialogDescription>确认取消预约 {cancelDialog.target?.appointment_no ?? ""}？已取消预约重复提交会保持已取消状态。</DialogDescription></DialogHeader>{cancelAppointment.error && <ErrorNotice message={cancelAppointment.error.message} />}<DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={cancelAppointment.isPending}>返回</Button></DialogClose><Button type="button" disabled={cancelAppointment.isPending || !cancelDialog.target} onClick={() => void submitAppointmentCancel()}>{cancelAppointment.isPending ? "取消中..." : "确认取消"}</Button></DialogFooter></DialogContent>
      </Dialog>
    </section>
  );

  function applyDockQuery(next: unknown) {
    applyQuery(queryValueFromUnknown(next));
    setSelectedId(null);
  }

  function clearDockQuery() {
    resetQuery();
    setSelectedId(null);
  }

  async function submitAppointmentChange(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const changeAppointment = changeDialog.target;
    if (!changeAppointment) return;
    const windowStartAt = dateTimeLocalToIso(changeForm.windowStartAt);
    const windowEndAt = dateTimeLocalToIso(changeForm.windowEndAt);
    if (!windowStartAt || !windowEndAt || new Date(windowEndAt) <= new Date(windowStartAt)) {
      updateAppointment.reset();
      setChangeValidationError("预约结束时间必须晚于开始时间");
      return;
    }
    setChangeValidationError(null);
    try {
      const updated = await updateAppointment.mutateAsync({ id: changeAppointment.id, body: {
        dock_id: changeForm.dockId || changeAppointment.dock_id,
        window_start_at: windowStartAt,
        window_end_at: windowEndAt,
        vehicle_plate_no: changeForm.vehiclePlateNo.trim() || null,
        vehicle_type: changeForm.vehicleType,
        driver_name: changeForm.driverName.trim(),
        driver_phone: changeForm.driverPhone.trim(),
        reason: changeForm.reason?.trim() || null,
      } });
      setAppointments((current) => [updated, ...current.map((item) => item.id === changeAppointment.id ? { ...item, status: "cancelled" } : item)]);
      changeDialog.close();
      setNotice(`预约 ${updated.appointment_no} 已生成新版本`);
    } catch { /* 保留弹窗，错误信息展示在表单中。 */ }
  }

  async function submitAppointmentCancel() {
    const cancelAppointmentTarget = cancelDialog.target;
    if (!cancelAppointmentTarget) return;
    try {
      const cancelled = await cancelAppointment.mutateAsync(cancelAppointmentTarget.id);
      setAppointments((current) => current.map((item) => item.id === cancelAppointmentTarget.id ? cancelled : item));
      cancelDialog.close();
      setNotice(`预约 ${cancelAppointmentTarget.appointment_no} 已取消`);
    } catch { /* 保留确认框，允许幂等重试。 */ }
  }
}

const dockColumns: DataGridColumn<Dock>[] = [
  { key: "dock_code", header: "月台编号", width: 150, minWidth: 120, mono: true, sortable: true, sortValue: (row) => row.dock_code, filterValue: (row) => row.dock_code, copyValue: (row) => row.dock_code, filter: { type: "text" } },
  { key: "dock_type", header: "作业类型", width: 140, minWidth: 120, filterValue: (row) => row.dock_type, copyValue: (row) => dockTypeLabel(row.dock_type), filter: { type: "multiSelect", options: dockTypeOptions }, render: (row) => dockTypeLabel(row.dock_type) },
  { key: "temperature_zone", header: "温区", width: 120, minWidth: 100, filterValue: (row) => row.temperature_zone, copyValue: (row) => temperatureZoneLabel(row.temperature_zone), filter: { type: "multiSelect", options: temperatureZoneOptions }, render: (row) => temperatureZoneLabel(row.temperature_zone) },
  { key: "status", header: "状态", width: 120, minWidth: 100, sortable: true, sortValue: (row) => row.status, filterValue: (row) => row.status, copyValue: (row) => statusLabel(row.status), filter: { type: "multiSelect", options: statusOptions }, render: (row) => <StatusBadge status={row.status === "active" ? "completed" : row.status === "maintenance" ? "pending" : "isolated"} label={statusLabel(row.status)} size="sm" /> },
  { key: "maintenance_recovery_at", header: "预计恢复", width: 170, minWidth: 150, sortable: true, sortValue: (row) => row.maintenance_recovery_at ?? "", filterValue: (row) => row.maintenance_recovery_at ?? "", copyValue: (row) => row.maintenance_recovery_at ? formatDateTime(row.maintenance_recovery_at) : "", filter: { type: "dateRange" }, render: (row) => row.maintenance_recovery_at ? formatDateTime(row.maintenance_recovery_at) : "-" },
  { key: "location_description", header: "位置说明", width: 230, minWidth: 160, filterValue: (row) => row.location_description ?? "", copyValue: (row) => row.location_description ?? "", filter: { type: "text" }, render: (row) => row.location_description || "-" },
  { key: "created_at", header: "创建时间", width: 180, minWidth: 150, sortable: true, sortValue: (row) => row.created_at, filterValue: (row) => row.created_at, copyValue: (row) => formatDateTime(row.created_at), filter: { type: "dateRange" }, render: (row) => formatDateTime(row.created_at) },
  { key: "updated_at", header: "更新时间", width: 180, minWidth: 150, sortable: true, sortValue: (row) => row.updated_at, filterValue: (row) => row.updated_at, copyValue: (row) => formatDateTime(row.updated_at), filter: { type: "dateRange" }, render: (row) => formatDateTime(row.updated_at) },
];

function DockCreateDialog({ open, form, pending, onOpenChange, onFormChange, onSubmit, errorMessage }: { open: boolean; form: DockForm; pending: boolean; onOpenChange: (open: boolean) => void; onFormChange: (form: DockForm) => void; onSubmit: (event: React.FormEvent<HTMLFormElement>) => void; errorMessage?: string }) {
  return <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}><DialogContent className="sm:max-w-lg"><form className="grid gap-4" onSubmit={onSubmit}><DialogHeader><DialogTitle>新增月台</DialogTitle><DialogDescription>保存后按当前仓库展示月台档案。</DialogDescription></DialogHeader><label className="grid gap-1 text-sm">月台编号<Input required maxLength={32} value={form.dockCode} onChange={(event) => onFormChange({ ...form, dockCode: event.target.value })} /></label><SelectField label="作业类型" value={form.dockType} options={dockTypeOptions} onChange={(value) => onFormChange({ ...form, dockType: value })} /><SelectField label="温区" value={form.temperatureZone} options={temperatureZoneOptions} onChange={(value) => onFormChange({ ...form, temperatureZone: value })} /><label className="grid gap-1 text-sm">位置说明<Input value={form.locationDescription} onChange={(event) => onFormChange({ ...form, locationDescription: event.target.value })} placeholder="例如东门卸货区" /></label>{errorMessage && <ErrorNotice message={errorMessage} />}<DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose><Button type="submit" disabled={pending}>{pending ? "保存中..." : "保存"}</Button></DialogFooter></form></DialogContent></Dialog>;
}

function DockEditDialog({ open, dock, form, pending, onOpenChange, onFormChange, onSubmit, errorMessage }: { open: boolean; dock: Dock | null; form: DockEditForm; pending: boolean; onOpenChange: (open: boolean) => void; onFormChange: (form: DockEditForm) => void; onSubmit: (event: React.FormEvent<HTMLFormElement>) => void; errorMessage?: string }) {
  return <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}><DialogContent className="sm:max-w-lg"><form className="grid gap-4" onSubmit={onSubmit}><DialogHeader><DialogTitle>编辑月台</DialogTitle><DialogDescription>{dock ? `${dock.dock_code} · ${dockTypeLabel(dock.dock_type)}` : "编辑月台状态"}</DialogDescription></DialogHeader><SelectField label="状态" value={form.status} options={statusOptions} onChange={(value) => onFormChange({ ...form, status: value })} />{form.status === "maintenance" && <label className="grid gap-1 text-sm">预计恢复日期<Input required type="date" value={form.maintenanceRecoveryDate} onChange={(event) => onFormChange({ ...form, maintenanceRecoveryDate: event.target.value })} /></label>}{errorMessage && <ErrorNotice message={errorMessage} />}<DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose><Button type="submit" disabled={pending || !form.status}>{pending ? "保存中..." : "保存"}</Button></DialogFooter></form></DialogContent></Dialog>;
}

function SelectField({ label, value, options, onChange }: { label: string; value: string; options: Array<{ label: string; value: string }>; onChange: (value: string) => void }) {
  return <label className="grid gap-1 text-sm">{label}<select className="h-9 rounded-md border border-input bg-background px-3 text-sm" value={value} onChange={(event) => onChange(event.target.value)}>{options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select></label>;
}

function ErrorNotice({ message }: { message: string }) { return <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">{message}</div>; }
function emptyDockForm(): DockForm { return { dockCode: "", dockType: "receiving", temperatureZone: "normal", locationDescription: "" }; }
function emptyDockEditForm(): DockEditForm { return { status: "active", maintenanceRecoveryDate: "" }; }
function editFormFor(dock: Dock): DockEditForm { return { status: dock.status, maintenanceRecoveryDate: dock.maintenance_recovery_at?.slice(0, 10) ?? "" }; }
function defaultQuery(): QueryPanelValue { return { warehouseId: "", keyword: "", status: [] }; }
/** 与 @/lib/query-value 不同：月台查询需要按固定字段重建（透传未知字段会污染 warehouseId 逻辑）。 */
function queryValueFromUnknown(value: unknown): QueryPanelValue {
  if (!value || typeof value !== "object") return defaultQuery();
  const record = value as QueryPanelValue;
  return {
    warehouseId: queryString(record.warehouseId),
    keyword: queryString(record.keyword),
    status: Array.isArray(record.status) ? record.status.filter((item): item is string => typeof item === "string") : [],
  };
}
function filterDocks(docks: Dock[], query: QueryPanelValue) {
  const keyword = queryString(query.keyword).trim().toLowerCase();
  const statuses = new Set(Array.isArray(query.status) ? query.status : []);
  return docks.filter((dock) => {
    const searchable = `${dock.dock_code} ${dock.location_description ?? ""}`.toLowerCase();
    return (!keyword || searchable.includes(keyword)) && (statuses.size === 0 || statuses.has(dock.status));
  });
}
function importDockRows(rows: string[][], warehouseId: string) {
  const [header, ...data] = rows;
  if (!header) throw new Error("导入文件为空");
  const index = new Map(header.map((value, position) => [normalizeHeader(value), position]));
  const read = (row: string[], aliases: string[]) => {
    const position = aliases.map(normalizeHeader).map((key) => index.get(key)).find((value) => value !== undefined);
    return position === undefined ? "" : row[position]?.trim() ?? "";
  };
  const docks = data.filter((row) => row.some((value) => value.trim())).map((row, position) => {
    const dockCode = read(row, ["dock_code", "月台编号", "编号"]);
    if (!dockCode) throw new Error(`第 ${position + 2} 行缺少月台编号`);
    const dockType = importOption(read(row, ["dock_type", "作业类型"]), dockTypeOptions, "作业类型", position);
    const temperatureZone = importOption(read(row, ["temperature_zone", "温区"]), temperatureZoneOptions, "温区", position);
    return {
      warehouse_id: warehouseId,
      dock_code: dockCode,
      dock_type: dockType,
      temperature_zone: temperatureZone,
      location_description: read(row, ["location_description", "位置说明"]) || null,
    };
  });
  if (!docks.length) throw new Error("导入文件没有数据行");
  return docks;
}
function importOption(value: string, options: Array<{ label: string; value: string }>, field: string, row: number) {
  const match = options.find((option) => option.value === value || option.label === value);
  if (!match) throw new Error(`第 ${row + 2} 行${field}无效`);
  return match.value;
}
function normalizeHeader(value: string) { return value.trim().toLowerCase().replace(/[\s_-]/g, ""); }
function dockTypeLabel(value: string) { return dockTypeOptions.find((option) => option.value === value)?.label ?? value; }
function temperatureZoneLabel(value: string) { return temperatureZoneOptions.find((option) => option.value === value)?.label ?? value; }
function statusLabel(value: string) { return statusOptions.find((option) => option.value === value)?.label ?? value; }
function dateToIso(value: string) { return value ? new Date(`${value}T00:00:00`).toISOString() : null; }
function emptyDockAppointmentForm(): DockAppointmentForm {
  const start = new Date(Date.now() + 60 * 60 * 1000);
  const end = new Date(start.getTime() + 60 * 60 * 1000);
  return {
    appointmentNo: `AP-${formatAppointmentNo(start)}`,
    documentType: "purchase_inbound",
    documentNo: "",
    windowStartAt: toDateTimeLocal(start),
    windowEndAt: toDateTimeLocal(end),
    vehiclePlateNo: "",
    vehicleType: "normal",
    driverName: "",
    driverPhone: "",
  };
}
function toDateTimeLocal(value: Date) {
  const offset = value.getTimezoneOffset();
  return new Date(value.getTime() - offset * 60 * 1000).toISOString().slice(0, 16);
}
function formatAppointmentNo(value: Date) { return value.toISOString().replace(/\D/g, "").slice(0, 14); }
function dateTimeLocalToIso(value: string) {
  const date = new Date(value);
  return value && !Number.isNaN(date.getTime()) ? date.toISOString() : null;
}
function appointmentFormFor(appointment: DockAppointment): DockAppointmentForm {
  return { dockId: appointment.dock_id, appointmentNo: appointment.appointment_no, documentType: appointment.document_type, documentNo: appointment.document_no, windowStartAt: toDateTimeLocal(new Date(appointment.window_start_at)), windowEndAt: toDateTimeLocal(new Date(appointment.window_end_at)), vehiclePlateNo: appointment.vehicle_plate_no ?? "", vehicleType: appointment.vehicle_type, driverName: appointment.driver_name, driverPhone: appointment.driver_phone, reason: "" };
}
function appointmentStatusLabel(value: string) { return ({ pending: "待确认", confirmed: "已确认", arrived: "已到达", cancelled: "已取消" } as Record<string, string>)[value] ?? value; }
