import * as React from "react";
import {
  Button,
  DataGrid,
  PageHeader,
  QueryPanel,
  StatusBadge,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { KeyRound, MapPinned, Power, Printer as PrinterIcon, Settings2, Unplug } from "lucide-react";

import type { CurrentUser } from "@/features/auth/auth-queries";
import { useMasterDataRowsQuery } from "@/features/master-data/master-data-queries/queries";
import {
  useCreatePrintSiteMutation,
  useCreatePrinterMutation,
  useCreatePrinterTrayMutation,
  useCreateSiteOwnerMappingMutation,
  useDeviceLeasesQuery,
  useDisableSiteOwnerMappingMutation,
  usePrintSitesQuery,
  useSiteOwnerMappingsQuery,
  usePrinterTraysQuery,
  usePrintersQuery,
  useReleaseDeviceLeaseMutation,
  useTestPrintMutation,
  useUpdatePrinterMutation,
  useUpdatePrinterTrayMutation,
  type DeviceLease,
  type PrintSite,
  type PrintSiteOwnerMapping,
  type Printer,
  type PrinterTray,
} from "@/features/print-orchestration/print-device-queries";
import { formatDateTime } from "@/lib/format";
import { queryString, queryValueFromUnknown } from "@/lib/query-value";
import {
  BUTTON_REFRESH,
  COLUMN_CREATED_AT,
  COLUMN_OWNER,
  COLUMN_STATUS,
  COLUMN_WAREHOUSE,
  FIELD_KEYWORD,
  STATUS_DEACTIVATED,
  STATUS_DISABLED,
  STATUS_ENABLED,
} from "@/lib/ui-strings";
import { usePageQueryState } from "@/lib/use-page-query-state";
import {
  DeviceWriteConfirmDialog,
  PrintSiteDialog,
  PrinterDialog,
  PrinterReleaseModeDialog,
  PrinterTrayDialog,
  ReleaseLeaseDialog,
  SiteOwnerMappingDialog,
  TestPrintDialog,
  busyStateLabel,
  type DeviceWriteConfirmation,
  releaseModeLabel,
} from "./H9PrintDeviceDialogs";

/**
 * 页面设计契约：设备维护工作台；主信息为 QueryPanel + Tabs + DataGrid；
 * 站点/映射/打印机/纸盒新建、测试打印与人工释放租约使用 Dialog；
 * Print Agent 注册与状态（US-H9-012）复用本菜单节点后续接入。
 */

export const h9PrintDeviceCoreQueryFieldKeys = ["keyword", "siteId"];

type DeviceWriteAction =
  | ({ kind: "disable-mapping"; siteId: string; mappingId: string } & DeviceWriteConfirmation)
  | ({ kind: "toggle-printer"; printerId: string; printerName: string; nextStatus: "active" | "disabled" } & DeviceWriteConfirmation)
  | ({ kind: "toggle-tray"; printerId: string; trayId: string; trayCode: string; nextEnabled: boolean } & DeviceWriteConfirmation);

export function H9PrintDevicePage({ currentUser }: { currentUser: CurrentUser }) {
  const warehousesQuery = useMasterDataRowsQuery("m1-warehouses");
  const warehouses = warehousesQuery.data ?? [];
  const sitesQuery = usePrintSitesQuery();
  const printersQuery = usePrintersQuery();
  const leasesQuery = useDeviceLeasesQuery();
  const createSite = useCreatePrintSiteMutation();
  const createMapping = useCreateSiteOwnerMappingMutation();
  const disableMapping = useDisableSiteOwnerMappingMutation();
  const createPrinter = useCreatePrinterMutation();
  const updatePrinter = useUpdatePrinterMutation();
  const createTray = useCreatePrinterTrayMutation();
  const updateTray = useUpdatePrinterTrayMutation();
  const testPrint = useTestPrintMutation();
  const releaseLease = useReleaseDeviceLeaseMutation();

  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(() => defaultQuery(), queryValueFromUnknown);
  const [siteIds, setSiteIds] = React.useState<string[]>([]);
  const [mappingIds, setMappingIds] = React.useState<string[]>([]);
  const [printerIds, setPrinterIds] = React.useState<string[]>([]);
  const [trayPrinterId, setTrayPrinterId] = React.useState("");
  const [trayIds, setTrayIds] = React.useState<string[]>([]);
  const [leaseIds, setLeaseIds] = React.useState<string[]>([]);
  const [siteOpen, setSiteOpen] = React.useState(false);
  const [mappingOpen, setMappingOpen] = React.useState(false);
  const [printerOpen, setPrinterOpen] = React.useState(false);
  const [releaseModeOpen, setReleaseModeOpen] = React.useState(false);
  const [trayOpen, setTrayOpen] = React.useState(false);
  const [testPrintOpen, setTestPrintOpen] = React.useState(false);
  const [releaseOpen, setReleaseOpen] = React.useState(false);
  const [writeAction, setWriteAction] = React.useState<DeviceWriteAction | null>(null);
  const [notice, setNotice] = React.useState<string | null>(null);
  const canWrite = currentUser.permissions.includes("h9.print_device.write");
  const canRelease = currentUser.permissions.includes("h9.device_lease.release");

  const sites = sitesQuery.data ?? [];
  const printers = React.useMemo(
    () => filterPrinters(printersQuery.data ?? [], appliedQuery),
    [appliedQuery, printersQuery.data],
  );
  const leases = React.useMemo(
    () => filterLeases(leasesQuery.data ?? [], appliedQuery, printersQuery.data ?? []),
    [appliedQuery, leasesQuery.data, printersQuery.data],
  );
  const filteredSites = React.useMemo(
    () => filterSites(sites, appliedQuery),
    [appliedQuery, sites],
  );
  const selectedSite = filteredSites.find((item) => siteIds.includes(item.id)) ?? null;
  const mappingsQuery = useSiteOwnerMappingsQuery(selectedSite?.id ?? "");
  const mappings = mappingsQuery.data ?? [];
  const selectedMapping = mappings.find((item) => mappingIds.includes(item.id)) ?? null;
  const selectedPrinter = printers.find((item) => printerIds.includes(item.id)) ?? null;
  const traysQuery = usePrinterTraysQuery(trayPrinterId);
  const trays = traysQuery.data ?? [];
  const selectedTray = trays.find((item) => trayIds.includes(item.id)) ?? null;
  const selectedPrinterTraysQuery = usePrinterTraysQuery(selectedPrinter?.id ?? "");
  const selectedLease = leases.find((item) => leaseIds.includes(item.id)) ?? null;

  React.useEffect(() => {
    if (printers.length === 0) return;
    if (!trayPrinterId || !printers.some((item) => item.id === trayPrinterId)) {
      setTrayPrinterId(printers[0].id);
    }
  }, [printers, trayPrinterId]);

  const siteOptions = React.useMemo(
    () => sites.map((item) => ({ value: item.id, label: `${item.site_code} ${item.site_name}` })),
    [sites],
  );
  const warehouseOptions = React.useMemo(
    () => warehouses.map((item) => ({ value: item.id, label: `${item.code} ${item.name}` })),
    [warehouses],
  );
  const warehouseLabels = React.useMemo(
    () => new Map(warehouses.map((item) => [item.id, `${item.code} ${item.name}`])),
    [warehouses],
  );
  const h9PrintDeviceQueryFields = React.useMemo(
    () => buildH9PrintDeviceQueryFields(siteOptions),
    [siteOptions],
  );
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(h9PrintDeviceQueryFields, appliedQuery),
    [appliedQuery, h9PrintDeviceQueryFields],
  );

  const siteCreateAction: DataGridCreateAction = {
    label: "新建站点",
    description: "登记一个物理打印站点",
    disabled: !canWrite || createSite.isPending,
    onClick: () => {
      createSite.reset();
      setSiteOpen(true);
    },
  };
  const mappingAction: DataGridToolbarAction = {
    key: "map-owner",
    label: "映射货主仓",
    description: "为选中站点显式映射货主 + 仓库",
    icon: <MapPinned className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedSite || createMapping.isPending,
    onClick: () => {
      createMapping.reset();
      setMappingOpen(true);
    },
  };
  const disableMappingAction: DataGridToolbarAction = {
    key: "disable-mapping",
    label: "停用映射",
    description: "软删选中的货主仓映射",
    icon: <Power className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedSite || !selectedMapping || selectedMapping.status !== "active" || disableMapping.isPending,
    onClick: () => {
      if (!selectedSite || !selectedMapping) return;
      disableMapping.reset();
      setWriteAction({
        kind: "disable-mapping",
        siteId: selectedSite.id,
        mappingId: selectedMapping.id,
        title: "停用货主仓映射",
        description: "确认停用该货主仓映射？停用为软删，历史记录仍保留。",
        confirmLabel: "确认停用",
        destructive: true,
      });
    },
  };
  const printerCreateAction: DataGridCreateAction = {
    label: "新建打印机",
    description: "在站点下登记打印机",
    disabled: !canWrite || createPrinter.isPending,
    onClick: () => {
      createPrinter.reset();
      setPrinterOpen(true);
    },
  };
  const togglePrinterAction: DataGridToolbarAction = {
    key: "toggle-printer",
    label: selectedPrinter?.status === "disabled" ? "启用打印机" : "停用打印机",
    description: "切换选中打印机启停状态",
    icon: <Power className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedPrinter || updatePrinter.isPending,
    onClick: () => {
      if (!selectedPrinter) return;
      const next = selectedPrinter.status === "disabled" ? "active" : "disabled";
      updatePrinter.reset();
      setWriteAction({
        kind: "toggle-printer",
        printerId: selectedPrinter.id,
        printerName: selectedPrinter.printer_name,
        nextStatus: next,
        title: `${next === "disabled" ? "停用" : "启用"}打印机`,
        description: `确认${next === "disabled" ? "停用" : "启用"}打印机“${selectedPrinter.printer_name}”？`,
        confirmLabel: `确认${next === "disabled" ? "停用" : "启用"}`,
        destructive: next === "disabled",
      });
    },
  };
  const releaseModeAction: DataGridToolbarAction = {
    key: "release-mode",
    label: "释放模式覆盖",
    description: "维护打印机级释放模式覆盖",
    icon: <Settings2 className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedPrinter || updatePrinter.isPending,
    onClick: () => {
      updatePrinter.reset();
      setReleaseModeOpen(true);
    },
  };
  const testPrintAction: DataGridToolbarAction = {
    key: "test-print",
    label: "测试打印",
    description: "对选中打印机的指定纸盒下发测试指令",
    icon: <PrinterIcon className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedPrinter || selectedPrinter.status !== "active" || testPrint.isPending,
    onClick: () => {
      testPrint.reset();
      setTestPrintOpen(true);
    },
  };
  const trayCreateAction: DataGridCreateAction = {
    label: "新建纸盒",
    description: "为当前打印机登记纸盒能力",
    disabled: !canWrite || !trayPrinterId || createTray.isPending,
    onClick: () => {
      createTray.reset();
      setTrayOpen(true);
    },
  };
  const toggleTrayAction: DataGridToolbarAction = {
    key: "toggle-tray",
    label: selectedTray && !selectedTray.enabled ? "启用纸盒" : "停用纸盒",
    description: "切换选中纸盒启用状态",
    icon: <Power className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedTray || updateTray.isPending,
    onClick: () => {
      if (!selectedTray) return;
      updateTray.reset();
      const nextEnabled = !selectedTray.enabled;
      setWriteAction({
        kind: "toggle-tray",
        printerId: selectedTray.printer_id,
        trayId: selectedTray.id,
        trayCode: selectedTray.tray_code,
        nextEnabled,
        title: `${nextEnabled ? "启用" : "停用"}纸盒`,
        description: `确认${nextEnabled ? "启用" : "停用"}纸盒 ${selectedTray.tray_code}？`,
        confirmLabel: `确认${nextEnabled ? "启用" : "停用"}`,
        destructive: !nextEnabled,
      });
    },
  };
  const releaseLeaseAction: DataGridToolbarAction = {
    key: "release-lease",
    label: "人工释放租约",
    description: "专用权限 + 原因 + 二次确认",
    icon: <Unplug className="size-4" aria-hidden />,
    disabled: !canRelease || !selectedLease || selectedLease.status !== "active" || releaseLease.isPending,
    onClick: () => {
      releaseLease.reset();
      setReleaseOpen(true);
    },
  };

  const trayPrinter = printers.find((item) => item.id === trayPrinterId) ?? null;
  const writePending =
    writeAction?.kind === "disable-mapping"
      ? disableMapping.isPending
      : writeAction?.kind === "toggle-printer"
        ? updatePrinter.isPending
        : writeAction?.kind === "toggle-tray"
          ? updateTray.isPending
          : false;
  const writeError =
    writeAction?.kind === "disable-mapping"
      ? disableMapping.error?.message
      : writeAction?.kind === "toggle-printer"
        ? updatePrinter.error?.message
        : writeAction?.kind === "toggle-tray"
          ? updateTray.error?.message
          : undefined;

  async function confirmWriteAction() {
    if (!writeAction) return;
    if (writeAction.kind === "disable-mapping") {
      await disableMapping.mutateAsync({
        siteId: writeAction.siteId,
        mappingId: writeAction.mappingId,
      });
      setMappingIds([]);
      setNotice("货主仓映射已停用（软删）");
    } else if (writeAction.kind === "toggle-printer") {
      await updatePrinter.mutateAsync({
        printerId: writeAction.printerId,
        request: { status: writeAction.nextStatus },
      });
      setNotice(`打印机“${writeAction.printerName}”已${writeAction.nextStatus === "disabled" ? "停用" : "启用"}`);
    } else {
      await updateTray.mutateAsync({
        printerId: writeAction.printerId,
        trayId: writeAction.trayId,
        request: { enabled: writeAction.nextEnabled },
      });
      setNotice(`纸盒 ${writeAction.trayCode} 已${writeAction.nextEnabled ? "启用" : "停用"}`);
    }
    setWriteAction(null);
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="设备·Print Agent 管理"
        subtitle="物理打印站点、货主仓映射、打印机、纸盒与设备租约；Print Agent 注册在 US-H9-012 接入本页"
        actions={notice ? <span className="self-center text-sm text-muted-foreground" role="status">{notice}</span> : undefined}
      />
      <QueryPanel
        fields={h9PrintDeviceQueryFields}
        defaultVisibleFieldKeys={h9PrintDeviceCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => {
          applyQuery(draftQuery);
          setSiteIds([]);
          setPrinterIds([]);
          setLeaseIds([]);
          setNotice("查询条件已应用");
        }}
        onReset={() => {
          resetQuery();
          setSiteIds([]);
          setPrinterIds([]);
          setLeaseIds([]);
        }}
      />
      <ErrorNotice
        message={
          sitesQuery.error?.message
          ?? printersQuery.error?.message
          ?? leasesQuery.error?.message
          ?? mappingsQuery.error?.message
          ?? traysQuery.error?.message
          ?? warehousesQuery.error?.message
        }
      />
      {!canWrite && <p className="text-sm text-muted-foreground">当前账号仅可查看打印设备配置。</p>}
      <Tabs defaultValue="sites">
        <TabsList className="h-auto flex-wrap">
          <TabsTrigger value="sites">站点（{filteredSites.length}）</TabsTrigger>
          <TabsTrigger value="printers">打印机（{printers.length}）</TabsTrigger>
          <TabsTrigger value="trays">纸盒（{trays.length}）</TabsTrigger>
          <TabsTrigger value="leases">租约（{leases.length}）</TabsTrigger>
        </TabsList>
        <TabsContent value="sites">
          <div className="space-y-4">
            <DataGrid
              columns={siteColumns}
              data={filteredSites}
              rowKey={(row) => row.id}
              storageKey="h9-print-sites"
              emptyTitle="暂无物理打印站点"
              emptyDescription="先新建站点，再映射货主仓并登记打印机"
              caption={sitesQuery.isPending ? "加载站点..." : undefined}
              refreshAction={refreshAction(sitesQuery, "站点")}
              createAction={siteCreateAction}
              toolbarActions={[mappingAction]}
              selectedRowKeys={siteIds}
              onSelectedRowKeysChange={(keys) => {
                setSiteIds(keys.length ? [keys.at(-1) as string] : []);
                setMappingIds([]);
              }}
              selectable
              queryState={appliedQuery}
              querySummaryItems={querySummaryItems}
              onApplyQueryState={(value) => applyQuery(queryValueFromUnknown(value))}
              onClearQueryState={resetQuery}
              tableClassName="min-w-[960px]"
            />
            <h3 className="text-sm font-medium">
              货主仓映射{selectedSite ? `：${selectedSite.site_code} ${selectedSite.site_name}` : "（请选择站点）"}
            </h3>
            <DataGrid
              columns={mappingColumns(warehouseLabels, currentUser)}
              data={mappings}
              rowKey={(row) => row.id}
              storageKey="h9-print-site-mappings"
              emptyTitle={selectedSite ? "该站点暂无货主仓映射" : "请选择站点"}
              emptyDescription="只有显式映射的货主 + 仓库可使用本站点设备；停用为软删"
              refreshAction={refreshAction(mappingsQuery, "货主仓映射")}
              toolbarActions={[disableMappingAction]}
              selectedRowKeys={mappingIds}
              onSelectedRowKeysChange={(keys) => setMappingIds(keys.length ? [keys.at(-1) as string] : [])}
              selectable
              tableClassName="min-w-[880px]"
            />
          </div>
        </TabsContent>
        <TabsContent value="printers">
          <DataGrid
            columns={printerColumns}
            data={printers}
            rowKey={(row) => row.id}
            storageKey="h9-printers"
            emptyTitle="暂无打印机"
            emptyDescription="打印机归属唯一站点；USB 打印机租约语义单机"
            caption={printersQuery.isPending ? "加载打印机..." : undefined}
            refreshAction={refreshAction(printersQuery, "打印机")}
            createAction={printerCreateAction}
            toolbarActions={[testPrintAction, releaseModeAction, togglePrinterAction]}
            selectedRowKeys={printerIds}
            onSelectedRowKeysChange={(keys) => setPrinterIds(keys.length ? [keys.at(-1) as string] : [])}
            selectable
            queryState={appliedQuery}
            querySummaryItems={querySummaryItems}
            tableClassName="min-w-[1240px]"
          />
        </TabsContent>
        <TabsContent value="trays">
          <div className="space-y-4">
            <label className="flex max-w-md flex-col gap-2 text-sm font-medium">
              <span>打印机</span>
              <select
                className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm"
                value={trayPrinterId}
                onChange={(event) => {
                  setTrayPrinterId(event.target.value);
                  setTrayIds([]);
                }}
              >
                <option value="">请选择打印机</option>
                {printers.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.site_code} · {item.printer_name}
                  </option>
                ))}
              </select>
            </label>
            <DataGrid
              columns={trayColumns}
              data={trays}
              rowKey={(row) => row.id}
              storageKey="h9-printer-trays"
              emptyTitle={trayPrinterId ? "该打印机暂无纸盒" : "请选择打印机"}
              emptyDescription="纸盒维护纸张能力、启用状态和设备标识"
              refreshAction={refreshAction(traysQuery, "纸盒")}
              createAction={trayCreateAction}
              toolbarActions={[toggleTrayAction]}
              selectedRowKeys={trayIds}
              onSelectedRowKeysChange={(keys) => setTrayIds(keys.length ? [keys.at(-1) as string] : [])}
              selectable
              tableClassName="min-w-[880px]"
            />
          </div>
        </TabsContent>
        <TabsContent value="leases">
          <p className="mb-3 text-sm text-muted-foreground">
            同一打印机同一时点只有一个活动租约；释放模式使用租约创建时的快照。人工释放需要专用权限、
            原因和二次确认；打印中、结果不明或待对账的租约任何人都不得释放。
          </p>
          <DataGrid
            columns={leaseColumns}
            data={leases}
            rowKey={(row) => row.id}
            storageKey="h9-device-leases"
            emptyTitle="暂无设备租约"
            emptyDescription="租约由 Print Agent（US-H9-012）签发；本页负责查看与授权人工释放"
            caption={leasesQuery.isPending ? "加载租约..." : undefined}
            refreshAction={refreshAction(leasesQuery, "租约")}
            toolbarActions={[releaseLeaseAction]}
            selectedRowKeys={leaseIds}
            onSelectedRowKeysChange={(keys) => setLeaseIds(keys.length ? [keys.at(-1) as string] : [])}
            selectable
            queryState={appliedQuery}
            querySummaryItems={querySummaryItems}
            tableClassName="min-w-[1240px]"
          />
        </TabsContent>
      </Tabs>
      <PrintSiteDialog
        open={siteOpen}
        pending={createSite.isPending}
        errorMessage={createSite.error?.message}
        onOpenChange={setSiteOpen}
        onSubmit={async (request) => {
          const site = await createSite.mutateAsync(request);
          setSiteOpen(false);
          setNotice(`站点 ${site.site_code} 已创建`);
        }}
      />
      <SiteOwnerMappingDialog
        open={mappingOpen}
        pending={createMapping.isPending}
        errorMessage={createMapping.error?.message}
        siteLabel={selectedSite ? `${selectedSite.site_code} ${selectedSite.site_name}` : ""}
        ownerLabel={`${currentUser.owner_code}（当前货主）`}
        warehouses={warehouseOptions}
        onOpenChange={setMappingOpen}
        onSubmit={async (warehouseId) => {
          if (!selectedSite) return;
          await createMapping.mutateAsync({
            siteId: selectedSite.id,
            request: { owner_id: currentUser.owner_id, warehouse_id: warehouseId },
          });
          setMappingOpen(false);
          setNotice("货主仓映射已创建");
        }}
      />
      <PrinterDialog
        open={printerOpen}
        pending={createPrinter.isPending}
        errorMessage={createPrinter.error?.message}
        sites={siteOptions}
        onOpenChange={setPrinterOpen}
        onSubmit={async (request) => {
          const printer = await createPrinter.mutateAsync(request);
          setPrinterOpen(false);
          setNotice(`打印机“${printer.printer_name}”已创建`);
        }}
      />
      <PrinterReleaseModeDialog
        open={releaseModeOpen}
        pending={updatePrinter.isPending}
        errorMessage={updatePrinter.error?.message}
        printer={selectedPrinter}
        onOpenChange={setReleaseModeOpen}
        onSubmit={async (mode) => {
          if (!selectedPrinter) return;
          const printer = await updatePrinter.mutateAsync({
            printerId: selectedPrinter.id,
            request: { release_mode_override: mode },
          });
          setReleaseModeOpen(false);
          setNotice(`打印机“${printer.printer_name}”生效释放模式：${releaseModeLabel(printer.effective_release_mode)}`);
        }}
      />
      <PrinterTrayDialog
        open={trayOpen}
        pending={createTray.isPending}
        errorMessage={createTray.error?.message}
        printer={trayPrinter}
        onOpenChange={setTrayOpen}
        onSubmit={async (request) => {
          if (!trayPrinterId) return;
          const tray = await createTray.mutateAsync({ printerId: trayPrinterId, request });
          setTrayOpen(false);
          setNotice(`纸盒 ${tray.tray_code} 已创建`);
        }}
      />
      <TestPrintDialog
        open={testPrintOpen}
        pending={testPrint.isPending}
        errorMessage={testPrint.error?.message ?? selectedPrinterTraysQuery.error?.message}
        printer={selectedPrinter}
        trays={selectedPrinterTraysQuery.data ?? []}
        onOpenChange={setTestPrintOpen}
        onSubmit={async (trayId) => {
          if (!selectedPrinter) return;
          const record = await testPrint.mutateAsync({ printerId: selectedPrinter.id, trayId });
          setTestPrintOpen(false);
          setNotice(`测试打印已下发（${testPrintResultLabel(record.result)}），等待硬件回执`);
        }}
      />
      <ReleaseLeaseDialog
        open={releaseOpen}
        pending={releaseLease.isPending}
        errorMessage={releaseLease.error?.message}
        lease={selectedLease}
        canRelease={canRelease}
        onOpenChange={setReleaseOpen}
        onSubmit={async (request) => {
          if (!selectedLease) return;
          await releaseLease.mutateAsync({ leaseId: selectedLease.id, request });
          setReleaseOpen(false);
          setLeaseIds([]);
          setNotice(`租约 ${selectedLease.lease_token} 已人工释放`);
        }}
      />
      <DeviceWriteConfirmDialog
        action={writeAction}
        pending={writePending}
        errorMessage={writeError}
        onOpenChange={(open) => !open && setWriteAction(null)}
        onConfirm={() => void confirmWriteAction().catch(() => undefined)}
      />
    </section>
  );
}

const siteColumns: DataGridColumn<PrintSite>[] = [
  { key: "site_code", header: "站点编码", width: 160, mono: true, copyValue: (row) => row.site_code },
  { key: "site_name", header: "站点名称", width: 220, render: (row) => row.site_name },
  { key: "status", header: COLUMN_STATUS, width: 110, render: (row) => <StatusBadge status={row.status === "active" ? "completed" : "expired"} label={row.status === "active" ? STATUS_ENABLED : STATUS_DISABLED} size="sm" /> },
  { key: "created_at", header: COLUMN_CREATED_AT, width: 180, render: (row) => formatDateTime(row.created_at) },
];

function mappingColumns(
  warehouseLabels: Map<string, string>,
  currentUser: CurrentUser,
): DataGridColumn<PrintSiteOwnerMapping>[] {
  return [
    {
      key: "owner",
      header: COLUMN_OWNER,
      width: 200,
      render: (row) => (row.owner_id === currentUser.owner_id ? `${currentUser.owner_code}（当前货主）` : row.owner_id),
    },
    { key: "warehouse", header: COLUMN_WAREHOUSE, width: 220, render: (row) => warehouseLabels.get(row.warehouse_id) ?? row.warehouse_id },
    { key: "status", header: COLUMN_STATUS, width: 110, render: (row) => <StatusBadge status={row.status === "active" ? "completed" : "expired"} label={row.status === "active" ? "生效" : STATUS_DEACTIVATED} size="sm" /> },
    { key: "created_at", header: "映射时间", width: 180, render: (row) => formatDateTime(row.created_at) },
    { key: "disabled_at", header: "停用时间", width: 180, render: (row) => row.disabled_at ? formatDateTime(row.disabled_at) : "-" },
  ];
}

const printerColumns: DataGridColumn<Printer>[] = [
  { key: "printer_name", header: "打印机名称", width: 220, copyValue: (row) => row.printer_name },
  { key: "site", header: "所属站点", width: 200, render: (row) => `${row.site_code} ${row.site_name}` },
  { key: "printer_model", header: "型号", width: 180, render: (row) => row.printer_model ?? "-" },
  { key: "connection_type", header: "连接类型", width: 140, render: (row) => <StatusBadge status={row.connection_type === "usb" ? "pending" : "completed"} label={row.connection_type === "usb" ? "USB（单机）" : "网络"} size="sm" /> },
  { key: "status", header: COLUMN_STATUS, width: 110, render: (row) => <StatusBadge status={row.status === "active" ? "completed" : "expired"} label={row.status === "active" ? STATUS_ENABLED : STATUS_DISABLED} size="sm" /> },
  { key: "release_mode", header: "释放模式", width: 200, render: (row) => `${releaseModeLabel(row.effective_release_mode)}${row.release_mode_override ? "（单机覆盖）" : "（全局默认）"}` },
  { key: "created_at", header: COLUMN_CREATED_AT, width: 180, defaultHidden: true, render: (row) => formatDateTime(row.created_at) },
];

const trayColumns: DataGridColumn<PrinterTray>[] = [
  { key: "tray_code", header: "设备标识", width: 150, mono: true, copyValue: (row) => row.tray_code },
  { key: "paper_size", header: "纸张尺寸", width: 130, render: (row) => row.paper_size },
  { key: "paper_type", header: "纸张类型", width: 180, render: (row) => row.paper_type },
  { key: "enabled", header: "启用状态", width: 110, render: (row) => <StatusBadge status={row.enabled ? "completed" : "expired"} label={row.enabled ? STATUS_ENABLED : STATUS_DISABLED} size="sm" /> },
  { key: "created_at", header: COLUMN_CREATED_AT, width: 180, render: (row) => formatDateTime(row.created_at) },
];

const leaseColumns: DataGridColumn<DeviceLease>[] = [
  { key: "printer_name", header: "打印机", width: 200, render: (row) => row.printer_name },
  { key: "connection_type", header: "连接类型", width: 130, render: (row) => row.connection_type === "usb" ? "USB（单机）" : "网络" },
  { key: "lease_token", header: "租约令牌", width: 220, mono: true, copyValue: (row) => row.lease_token },
  { key: "holder_agent_id", header: "持有 Agent", width: 200, render: (row) => row.holder_agent_id ?? "（待 US-H9-012 接入）" },
  { key: "release_mode", header: "释放模式快照", width: 150, render: (row) => releaseModeLabel(row.release_mode) },
  { key: "busy_state", header: "安全状态", width: 130, render: (row) => <StatusBadge status={row.busy_state === "idle" ? "completed" : "in_progress"} label={busyStateLabel(row.busy_state)} size="sm" /> },
  { key: "status", header: "租约状态", width: 120, render: (row) => <StatusBadge status={row.status === "active" ? "completed" : "expired"} label={row.status === "active" ? "活动" : "已释放"} size="sm" /> },
  { key: "assigned_at", header: "分配时间", width: 180, defaultHidden: true, render: (row) => formatDateTime(row.assigned_at) },
  { key: "released_at", header: "释放时间", width: 180, render: (row) => row.released_at ? formatDateTime(row.released_at) : "-" },
  { key: "release_reason", header: "释放原因", width: 220, render: (row) => row.release_reason ?? "-" },
];

function buildH9PrintDeviceQueryFields(
  siteOptions: Array<{ value: string; label: string }>,
): QueryPanelField[] {
  return [
    { key: "keyword", label: FIELD_KEYWORD, type: "text", placeholder: "站点 / 打印机 / 租约令牌" },
    { key: "siteId", label: "站点", type: "select", options: siteOptions },
    {
      key: "connectionType",
      label: "连接类型",
      type: "select",
      options: [
        { value: "network", label: "网络" },
        { value: "usb", label: "USB（单机）" },
      ],
    },
  ];
}

function defaultQuery(): QueryPanelValue {
  return { keyword: "", siteId: "", connectionType: "" };
}

function keywordOf(query: QueryPanelValue) {
  return queryString(query.keyword).trim().toLocaleLowerCase("zh-CN");
}

function filterSites(rows: PrintSite[], query: QueryPanelValue) {
  const keyword = keywordOf(query);
  const siteId = queryString(query.siteId);
  return rows.filter((row) =>
    (!siteId || row.id === siteId)
    && (!keyword || `${row.site_code} ${row.site_name}`.toLocaleLowerCase("zh-CN").includes(keyword)),
  );
}

function filterPrinters(rows: Printer[], query: QueryPanelValue) {
  const keyword = keywordOf(query);
  const siteId = queryString(query.siteId);
  const connectionType = queryString(query.connectionType);
  return rows.filter((row) =>
    (!siteId || row.site_id === siteId)
    && (!connectionType || row.connection_type === connectionType)
    && (!keyword
      || `${row.printer_name} ${row.printer_model ?? ""} ${row.site_code} ${row.site_name}`
        .toLocaleLowerCase("zh-CN")
        .includes(keyword)),
  );
}

function filterLeases(rows: DeviceLease[], query: QueryPanelValue, printers: Printer[]) {
  const keyword = keywordOf(query);
  const siteId = queryString(query.siteId);
  const connectionType = queryString(query.connectionType);
  const printerSites = new Map(printers.map((item) => [item.id, item.site_id]));
  return rows.filter((row) =>
    (!siteId || printerSites.get(row.printer_id) === siteId)
    && (!connectionType || row.connection_type === connectionType)
    && (!keyword || `${row.printer_name} ${row.lease_token}`.toLocaleLowerCase("zh-CN").includes(keyword)),
  );
}

function testPrintResultLabel(result: string) {
  return result === "dispatched" ? "已下发测试指令" : result === "succeeded" ? "成功" : result === "failed" ? "失败" : result;
}

function refreshAction(query: { isFetching: boolean; refetch: () => Promise<unknown> }, label: string): DataGridRefreshAction {
  return { label: BUTTON_REFRESH, description: `刷新${label}`, disabled: query.isFetching, onClick: () => void query.refetch() };
}

function ErrorNotice({ message }: { message?: string }) {
  return message ? <p className="rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive" role="alert">{message}</p> : null;
}
