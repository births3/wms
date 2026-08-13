import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  Checkbox,
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  PageHeader,
  StatusBadge,
  cn,
} from "@wms/ui";
import { GitBranch, RefreshCw, RotateCcw, Save, Send, ToggleLeft } from "lucide-react";

import {
  useBatchEnableAdminMenuMutation,
  useCreateAdminMenuNodeMutation,
  useDraftAdminMenuQuery,
  usePublishAdminMenuMutation,
  useRollbackAdminMenuMutation,
  useUpdateAdminMenuNodeMutation,
  type AdminMenuButtonPermission,
  type AdminMenuNode,
} from "@/features/admin-menu/admin-menu-queries";

const iconOptions = [
  "Activity",
  "ArrowUpCircle",
  "Bell",
  "BellRing",
  "BookOpen",
  "CheckCircle2",
  "ClipboardList",
  "Database",
  "History",
  "Inbox",
  "KeyRound",
  "Layers",
  "MapPinned",
  "PackageCheck",
  "PanelLeftOpen",
  "Printer",
  "Settings",
  "ShieldCheck",
  "Stamp",
  "Truck",
  "Users",
  "Warehouse",
];

const viewIdOptions = [
  "dashboard",
  "m1-products",
  "m1-business-partners",
  "m1-warehouses",
  "m1-zones",
  "m1-locations",
  "m1-system-dictionary",
  "dock-management",
  "m1-feature-flags",
  "m2-receiving",
  "m2-inbound-documents",
  "m-di-review",
  "m-di-stamp",
  "m2-inspecting",
  "m2-putaway",
  "m2-putaway-strategy",
  "m-di-platforms",
  "m3-batches",
  "m3-location-history",
  "m3-status-config",
  "m3-counts",
  "m3-maintenance",
  "m3-relocations",
  "mte-task-types",
  "mte-task-groups",
  "mte-task-dispatch",
  "m9-billing-rules",
  "m10-route-plans",
  "m4-orders",
  "m4-waves",
  "m4-review",
  "m4-returns",
  "mrc-reconciliation",
  "h1-menu-management",
  "h1-role-permission",
  "h1-session-management",
  "h1-api-keys",
  "h2-audit-trail",
  "h3-api-contract",
  "h4-wechat-settings",
  "h4-notify-configs",
  "h4-notify-records",
  "hal-alert-dashboard",
  "hal-alert-definitions",
  "hal-alert-escalations",
  "h5-express",
  "h8-erp-connectors",
  "h8-erp-messages",
  "h8-erp-interface-tables",
  "h9-print-templates",
  "h9-delivery-note-aggregation",
  "h9-print-devices",
  "mcg-numbering",
];

type Notice = { type: "success" | "error"; text: string } | null;

interface NodeForm {
  title: string;
  viewId: string;
  iconKey: string;
  permissionKey: string;
  sortOrder: string;
  enabled: boolean;
  buttons: AdminMenuButtonPermission[];
}

interface CreateChildForm {
  title: string;
  code: string;
  enabled: boolean;
  viewId: string;
}

export function H1AdminMenuPage() {
  const draftQuery = useDraftAdminMenuQuery();
  const createMutation = useCreateAdminMenuNodeMutation();
  const updateMutation = useUpdateAdminMenuNodeMutation();
  const batchEnableMutation = useBatchEnableAdminMenuMutation();
  const publishMutation = usePublishAdminMenuMutation();
  const rollbackMutation = useRollbackAdminMenuMutation();
  const nodes = draftQuery.data?.data ?? [];
  const flatNodes = React.useMemo(() => flattenNodes(nodes), [nodes]);
  const [treeSearch, setTreeSearch] = React.useState("");
  const filteredNodes = React.useMemo(() => filterMenuTree(nodes, treeSearch), [nodes, treeSearch]);
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [checkedIds, setCheckedIds] = React.useState<string[]>([]);
  const [dragId, setDragId] = React.useState<string | null>(null);
  const [notice, setNotice] = React.useState<Notice>(null);
  const selectedNode = flatNodes.find((node) => node.id === selectedId) ?? flatNodes[0] ?? null;
  const [form, setForm] = React.useState<NodeForm>(() => formFromNode(null));
  const [newActionKey, setNewActionKey] = React.useState("");
  const [newActionLabel, setNewActionLabel] = React.useState("");
  const [createDialogOpen, setCreateDialogOpen] = React.useState(false);
  const [createParentSnapshot, setCreateParentSnapshot] = React.useState<AdminMenuNode | null>(null);
  const [createForm, setCreateForm] = React.useState<CreateChildForm>({
    title: "",
    code: "",
    enabled: true,
    viewId: "",
  });
  const busy = createMutation.isPending || updateMutation.isPending || batchEnableMutation.isPending || publishMutation.isPending || rollbackMutation.isPending;
  const createParentNode = createDialogOpen ? createParentSnapshot : null;
  const createChildLevel = createParentNode && createParentNode.level < 3 ? createParentNode.level + 1 : null;
  const createFormValid = Boolean(createForm.title.trim() && createForm.code.trim());

  React.useEffect(() => {
    if (!selectedId && flatNodes[0]) setSelectedId(flatNodes[0].id);
  }, [flatNodes, selectedId]);

  // 仅当选中节点变化、或该节点服务端数据版本变化（保存 / 批量停用 / 回滚后）时才重置表单，
  // 避免每次树数据刷新引用变化就把用户未保存的编辑覆盖成旧值。
  const selectedNodeSignature = selectedNode ? JSON.stringify(formFromNode(selectedNode)) : null;

  React.useEffect(() => {
    setForm(selectedNodeSignature === null ? formFromNode(null) : JSON.parse(selectedNodeSignature) as NodeForm);
  }, [selectedNode?.id, selectedNodeSignature]);

  async function run<T>(task: Promise<T>, success: string, fallback: string): Promise<boolean> {
    setNotice(null);
    try {
      await task;
      setNotice({ type: "success", text: success });
      return true;
    } catch (error) {
      setNotice({ type: "error", text: error instanceof Error ? error.message : fallback });
      return false;
    }
  }

  async function saveSelected() {
    if (!selectedNode) return;
    if (!window.confirm(`确认保存菜单节点「${selectedNode.title}」？`)) return;
    await run(
      updateMutation.mutateAsync({
        id: selectedNode.id,
        body: {
          title: form.title,
          view_id: selectedNode.level === 3 ? form.viewId : undefined,
          icon_key: form.iconKey,
          permission_key: form.permissionKey,
          sort_order: Number(form.sortOrder || selectedNode.sort_order),
          enabled: form.enabled,
          button_permissions: form.buttons,
        },
      }),
      "菜单节点已保存",
      "保存菜单节点失败",
    );
  }

  function openCreateChildDialog() {
    if (!selectedNode || selectedNode.level >= 3) return;
    const stamp = Date.now();
    setCreateParentSnapshot(selectedNode);
    setCreateForm({
      title: selectedNode.level === 1 ? "新能力组" : "新页面",
      code: `custom.${stamp}`,
      enabled: true,
      viewId: "",
    });
    setCreateDialogOpen(true);
  }

  async function createChildFromDialog() {
    if (!createParentSnapshot || createParentSnapshot.level >= 3) return;
    const title = createForm.title.trim();
    const code = createForm.code.trim();
    if (!title || !code) return;
    // 优先用树中最新父节点取 children 排序；挂靠目标始终用打开弹窗时锁定的 parent_id
    const parentLive = flatNodes.find((node) => node.id === createParentSnapshot.id) ?? createParentSnapshot;
    const ok = await run(
      createMutation.mutateAsync({
        parent_id: createParentSnapshot.id,
        code,
        title,
        view_id: createParentSnapshot.level === 2 ? createForm.viewId.trim() || undefined : undefined,
        icon_key: "ShieldCheck",
        permission_key: `menu.${code}`,
        sort_order: parentLive.children.length * 10 + 10,
        enabled: createForm.enabled,
        button_permissions: [],
      }),
      "菜单节点已新增",
      "新增菜单节点失败",
    );
    if (ok) {
      setCreateDialogOpen(false);
      setCreateParentSnapshot(null);
    }
  }

  async function batchDisableSelected() {
    if (!checkedIds.length) return;
    if (!window.confirm(`确认停用选中的 ${checkedIds.length} 个菜单节点？`)) return;
    await run(batchEnableMutation.mutateAsync({ ids: checkedIds, enabled: false }), "已批量停用", "批量停用失败");
  }

  async function rollbackMenu() {
    if (!window.confirm("确认回滚当前菜单草稿？")) return;
    await run(rollbackMutation.mutateAsync({ target_version_no: null }), "菜单已回滚", "回滚菜单失败");
  }

  async function publishMenu() {
    if (!window.confirm("确认发布 PC 菜单管理配置？")) return;
    await run(publishMutation.mutateAsync({ note: "PC 菜单管理发布" }), "菜单已发布", "发布菜单失败");
  }

  async function dropOn(target: AdminMenuNode) {
    if (!dragId || dragId === target.id) return;
    const parentId = target.level < 3 ? target.id : target.parent_id;
    await run(
      updateMutation.mutateAsync({
        id: dragId,
        body: { parent_id: parentId ?? undefined, sort_order: target.sort_order + 1 },
      }),
      "菜单拖拽已保存",
      "保存拖拽结果失败",
    );
    setDragId(null);
  }

  function toggleChecked(id: string, checked: boolean) {
    setCheckedIds((current) => checked ? Array.from(new Set([...current, id])) : current.filter((item) => item !== id));
  }

  function addButtonPermission() {
    if (!newActionKey.trim() || !newActionLabel.trim()) return;
    setForm((current) => ({
      ...current,
      buttons: [
        ...current.buttons.filter((button) => button.action_key !== newActionKey.trim()),
        {
          action_key: newActionKey.trim(),
          action_label: newActionLabel.trim(),
          action_kind: "private",
          enabled: true,
          sort_order: current.buttons.length * 10 + 10,
        },
      ],
    }));
    setNewActionKey("");
    setNewActionLabel("");
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <PageHeader />
        <div className="flex flex-wrap gap-2">
          <Button type="button" variant="outline" size="sm" disabled={draftQuery.isFetching} onClick={() => void draftQuery.refetch()}>
            <RefreshCw className="size-4" aria-hidden />刷新
          </Button>
          <Button type="button" variant="outline" size="sm" disabled={busy || checkedIds.length === 0} onClick={() => void batchDisableSelected()}>
            <ToggleLeft className="size-4" aria-hidden />停用
          </Button>
          <Button type="button" variant="outline" size="sm" disabled={busy} onClick={() => void rollbackMenu()}>
            <RotateCcw className="size-4" aria-hidden />回滚
          </Button>
          <Button type="button" size="sm" disabled={busy} onClick={() => void publishMenu()}>
            <Send className="size-4" aria-hidden />发布
          </Button>
        </div>
      </div>

      <NoticePanel notice={notice} />

      <div className="grid gap-4 lg:grid-cols-[minmax(22rem,0.95fr)_minmax(28rem,1.3fr)]">
        <Card className="rounded-lg shadow-sm">
          <CardContent className="space-y-3 p-4">
            <div className="flex items-center justify-between gap-3">
              <h2 className="text-base font-semibold tracking-normal">菜单树</h2>
              <Button type="button" variant="outline" size="sm" disabled={!selectedNode || selectedNode.level >= 3 || busy} onClick={openCreateChildDialog}>
                新增
              </Button>
            </div>
            <Input
              value={treeSearch}
              onChange={(event) => setTreeSearch(event.target.value)}
              placeholder="搜索菜单标题或编码"
              aria-label="搜索菜单标题或编码"
            />
            {draftQuery.error ? <p className="text-sm text-destructive">{draftQuery.error.message}</p> : null}
            <div className="space-y-1">
              {filteredNodes.length === 0 ? (
                <p className="px-1 py-2 text-sm text-muted-foreground">
                  {treeSearch.trim() ? "未找到匹配的菜单节点" : "暂无菜单节点"}
                </p>
              ) : (
                filteredNodes.map((node) => (
                  <MenuTreeNode
                    key={node.id}
                    node={node}
                    selectedId={selectedNode?.id ?? ""}
                    checkedIds={checkedIds}
                    onSelect={setSelectedId}
                    onChecked={toggleChecked}
                    onDragStart={setDragId}
                    onDrop={(target) => void dropOn(target)}
                  />
                ))
              )}
            </div>
          </CardContent>
        </Card>

        <Card className="rounded-lg shadow-sm">
          <CardContent className="space-y-4 p-5">
            <div className="flex items-center justify-between gap-3">
              <h2 className="text-base font-semibold tracking-normal">节点配置</h2>
              {selectedNode ? <StatusBadge status={form.enabled ? "completed" : "offline_cached"} label={form.enabled ? "启用" : "停用"} size="sm" /> : null}
            </div>
            {!selectedNode ? (
              <p className="text-sm text-muted-foreground">请选择左侧菜单节点。</p>
            ) : (
              <>
                <div className="grid gap-3 md:grid-cols-2">
                  <Field label="名称"><Input value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} /></Field>
                  <Field label="权限键"><Input value={form.permissionKey} onChange={(e) => setForm({ ...form, permissionKey: e.target.value })} /></Field>
                  <Field label="图标">
                    <select className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={form.iconKey} onChange={(e) => setForm({ ...form, iconKey: e.target.value })}>
                      {iconOptions.map((icon) => <option key={icon} value={icon}>{icon}</option>)}
                    </select>
                  </Field>
                  <Field label="绑定 view_id">
                    <select className="h-10 rounded-md border border-input bg-background px-3 text-sm" disabled={selectedNode.level !== 3} value={form.viewId} onChange={(e) => setForm({ ...form, viewId: e.target.value })}>
                      <option value="">不绑定</option>
                      {viewIdOptions.map((viewId) => <option key={viewId} value={viewId}>{viewId}</option>)}
                    </select>
                  </Field>
                  <Field label="排序"><Input value={form.sortOrder} onChange={(e) => setForm({ ...form, sortOrder: e.target.value })} /></Field>
                  <label className="flex items-center gap-2 pt-6 text-sm">
                    <Checkbox checked={form.enabled} onCheckedChange={(value) => setForm({ ...form, enabled: value === true })} />启用
                  </label>
                </div>

                <div className="space-y-2">
                  <h3 className="text-sm font-semibold tracking-normal">按钮权限点</h3>
                  <div className="grid gap-2 md:grid-cols-[1fr_1fr_auto]">
                    <Input placeholder="action_key" value={newActionKey} onChange={(event) => setNewActionKey(event.target.value)} />
                    <Input placeholder="按钮名称" value={newActionLabel} onChange={(event) => setNewActionLabel(event.target.value)} />
                    <Button type="button" variant="outline" onClick={addButtonPermission}>添加</Button>
                  </div>
                  <div className="grid gap-2 md:grid-cols-2">
                    {form.buttons.map((button) => (
                      <label key={button.action_key} className="flex items-center justify-between gap-2 rounded-md border px-3 py-2 text-sm">
                        <span className="min-w-0 truncate">{button.action_label} / {button.action_key}</span>
                        <Checkbox checked={button.enabled} onCheckedChange={(value) => setForm({ ...form, buttons: form.buttons.map((item) => item.action_key === button.action_key ? { ...item, enabled: value === true } : item) })} />
                      </label>
                    ))}
                  </div>
                </div>

                <Button type="button" disabled={busy} onClick={() => void saveSelected()}>
                  <Save className="size-4" aria-hidden />保存
                </Button>
              </>
            )}
          </CardContent>
        </Card>
      </div>
      <Dialog
        open={createDialogOpen}
        onOpenChange={(open) => {
          setCreateDialogOpen(open);
          if (!open) setCreateParentSnapshot(null);
        }}
      >
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>新增菜单节点</DialogTitle>
          </DialogHeader>
          <div className="grid gap-3">
            <Field label="父节点">
              <Input
                value={createParentNode ? `${createParentNode.title}（L${createParentNode.level}）` : ""}
                readOnly
                disabled
                aria-label="父节点"
              />
            </Field>
            <Field label="层级">
              <Input
                value={createChildLevel ? `L${createChildLevel}` : ""}
                readOnly
                disabled
                aria-label="层级"
              />
            </Field>
            <Field label="菜单名称">
              <Input
                value={createForm.title}
                onChange={(event) => setCreateForm({ ...createForm, title: event.target.value })}
                aria-label="菜单名称"
              />
            </Field>
            <Field label="编码">
              <Input
                value={createForm.code}
                onChange={(event) => setCreateForm({ ...createForm, code: event.target.value })}
                aria-label="编码"
                placeholder="custom.timestamp"
              />
            </Field>
            <label className="flex items-center gap-2 text-sm text-foreground">
              <Checkbox
                checked={createForm.enabled}
                onCheckedChange={(value) => setCreateForm({ ...createForm, enabled: value === true })}
                aria-label="启用"
              />
              启用
            </label>
            {createParentNode?.level === 2 ? (
              <Field label="绑定 view_id">
                <select
                  className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                  value={createForm.viewId}
                  onChange={(event) => setCreateForm({ ...createForm, viewId: event.target.value })}
                  aria-label="绑定 view_id"
                >
                  <option value="">不绑定</option>
                  {viewIdOptions.map((viewId) => <option key={viewId} value={viewId}>{viewId}</option>)}
                </select>
              </Field>
            ) : null}
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                setCreateDialogOpen(false);
                setCreateParentSnapshot(null);
              }}
            >
              取消
            </Button>
            <Button type="button" disabled={busy || !createFormValid} onClick={() => void createChildFromDialog()}>新增</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}

function MenuTreeNode({
  node,
  selectedId,
  checkedIds,
  onSelect,
  onChecked,
  onDragStart,
  onDrop,
}: {
  node: AdminMenuNode;
  selectedId: string;
  checkedIds: string[];
  onSelect: (id: string) => void;
  onChecked: (id: string, checked: boolean) => void;
  onDragStart: (id: string) => void;
  onDrop: (node: AdminMenuNode) => void;
}) {
  const selected = selectedId === node.id;
  return (
    <div className="space-y-1">
      <div
        draggable
        onDragStart={() => onDragStart(node.id)}
        onDragOver={(event) => event.preventDefault()}
        onDrop={() => onDrop(node)}
        className={cn(
          "flex cursor-pointer items-center gap-2 rounded-md border px-2 py-2 text-sm",
          menuLevelClass(node.level),
          selected ? "border-primary bg-primary/10 text-primary" : "hover:bg-muted",
        )}
        onClick={() => onSelect(node.id)}
      >
        <Checkbox checked={checkedIds.includes(node.id)} onCheckedChange={(value) => onChecked(node.id, value === true)} />
        <GitBranch className="size-4 shrink-0" aria-hidden />
        <span className="min-w-0 flex-1 truncate" title={`${node.title}（${node.code}）`}>{node.title}</span>
        <span className="shrink-0 text-xs text-muted-foreground">L{node.level}</span>
      </div>
      {node.children.map((child) => (
        <MenuTreeNode key={child.id} node={child} selectedId={selectedId} checkedIds={checkedIds} onSelect={onSelect} onChecked={onChecked} onDragStart={onDragStart} onDrop={onDrop} />
      ))}
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="grid gap-1 text-xs text-muted-foreground">{label}{children}</label>;
}

function NoticePanel({ notice }: { notice: Notice }) {
  if (!notice) return null;
  return <div className={cn("rounded-md border px-4 py-3 text-sm", notice.type === "success" ? "border-wms-success/30 bg-wms-success/10 text-wms-success" : "border-destructive/30 bg-destructive/10 text-destructive")}>{notice.text}</div>;
}

function formFromNode(node: AdminMenuNode | null): NodeForm {
  return {
    title: node?.title ?? "",
    viewId: node?.view_id ?? "",
    iconKey: node?.icon_key ?? "ShieldCheck",
    permissionKey: node?.permission_key ?? "",
    sortOrder: String(node?.sort_order ?? 10),
    enabled: node?.enabled ?? true,
    buttons: node?.button_permissions ?? [],
  };
}

function flattenNodes(nodes: AdminMenuNode[]): AdminMenuNode[] {
  return nodes.flatMap((node) => [node, ...flattenNodes(node.children)]);
}

/** 按标题/编码过滤菜单树：匹配节点保留完整子树，仅后代匹配时保留祖先路径。 */
function filterMenuTree(nodes: AdminMenuNode[], keyword: string): AdminMenuNode[] {
  const query = keyword.trim().toLowerCase();
  if (!query) return nodes;

  const nodeMatches = (node: AdminMenuNode) =>
    node.title.toLowerCase().includes(query) || node.code.toLowerCase().includes(query);

  const walk = (list: AdminMenuNode[]): AdminMenuNode[] => {
    const result: AdminMenuNode[] = [];
    for (const node of list) {
      const selfMatch = nodeMatches(node);
      const children = walk(node.children);
      if (selfMatch || children.length > 0) {
        result.push({
          ...node,
          children: selfMatch ? node.children : children,
        });
      }
    }
    return result;
  };

  return walk(nodes);
}

function menuLevelClass(level: number) {
  if (level === 1) return "bg-primary/5 font-semibold";
  if (level === 2) return "ml-4 bg-muted/60";
  return "ml-8 bg-background";
}
