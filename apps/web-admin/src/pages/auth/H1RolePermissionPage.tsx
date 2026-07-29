import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  Checkbox,
  DataGrid,
  Dialog,
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
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { ChevronDown, ChevronRight, ShieldCheck, UsersRound } from "lucide-react";

import type { CurrentUser } from "@/features/auth/auth-queries";
import { CreateUserDialog } from "@/pages/auth/CreateUserDialog";
import {
  useBatchAssignRolesMutation,
  useCreateUserMutation,
  useCreateRoleMutation,
  useDeleteRoleMutation,
  usePermissionsQuery,
  useReplaceRolePermissionsMutation,
  useRoleUsersQuery,
  useRolesQuery,
  useUpdateRoleMutation,
  type Permission,
  type Role,
  type RoleUser,
  type CreateUserRequest,
} from "@/features/auth/role-permission-queries";
import { queryString, queryValueFromUnknown } from "@/lib/query-value";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";

/**
 * H1RolePermissionPage — 角色、权限矩阵和用户批量授权管理
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-H1-002
 * Wave：Wave 1 W1.A
 * 业务约束：角色权限写入使用完整权限集原子替换，并由 h1.roles.manage 控制按钮。
 * 页面设计契约：配置型；主信息载体为 QueryPanel + DataGrid 角色列表和权限矩阵；标准动作放在 DataGrid；私有动作通过 Dialog；不常驻用户明细、审计或写入表单。
 *
 * @example
 *   <H1RolePermissionPage currentUser={currentUser} />
 */

const ROLE_MANAGE_PERMISSION = "h1.roles.manage";
const DATA_SCOPE_OPTIONS = [
  { label: "自己", value: "self" },
  { label: "本仓", value: "warehouse" },
  { label: "本货主", value: "owner" },
  { label: "全部", value: "all" },
];

const h1RoleQueryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text", placeholder: "角色编码 / 名称", ariaLabel: "搜索角色" },
  { key: "dataScope", label: "数据范围", type: "select", options: DATA_SCOPE_OPTIONS },
];
const h1RoleCoreQueryFieldKeys = ["keyword"];

type Notice = { type: "success" | "error"; text: string } | null;
type RoleForm = { id: string | null; roleCode: string; roleName: string; dataScope: string; parentRoleId: string };
type RoleQuery = { keyword: string; dataScope: string };

export function H1RolePermissionPage({ currentUser }: { currentUser: CurrentUser }) {
  const canManage = currentUser.permissions.includes(ROLE_MANAGE_PERMISSION);
  const rolesQuery = useRolesQuery(canManage);
  const permissionsQuery = usePermissionsQuery(canManage);
  const usersQuery = useRoleUsersQuery(canManage);
  const createMutation = useCreateRoleMutation();
  const updateMutation = useUpdateRoleMutation();
  const deleteMutation = useDeleteRoleMutation();
  const permissionsMutation = useReplaceRolePermissionsMutation();
  const batchMutation = useBatchAssignRolesMutation();
  const createUserMutation = useCreateUserMutation();
  const roles = rolesQuery.data ?? [];
  const users = usersQuery.data ?? [];
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultRoleQuery, normalizeRoleQuery);
  const [selectedRoleId, setSelectedRoleId] = React.useState<string | null>(null);
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [permissionCodes, setPermissionCodes] = React.useState<string[]>([]);
  const {
    open: roleDialogOpen,
    target: roleFormTarget,
    openWith: openRoleFormWith,
    setOpen: setRoleDialogOpen,
    setTarget: setRoleForm,
  } = useDialogState<RoleForm>();
  const roleForm = roleFormTarget ?? emptyRoleForm();
  const [deleteDialogOpen, setDeleteDialogOpen] = React.useState(false);
  const [batchDialogOpen, setBatchDialogOpen] = React.useState(false);
  const [createUserDialogOpen, setCreateUserDialogOpen] = React.useState(false);
  const [selectedUserIds, setSelectedUserIds] = React.useState<string[]>([]);
  const [assignRoleIds, setAssignRoleIds] = React.useState<string[]>([]);
  const [notice, setNotice] = React.useState<Notice>(null);
  const filteredRoles = React.useMemo(() => filterRoles(roles, normalizeRoleQuery(appliedQuery)), [appliedQuery, roles]);
  const selectedRole = roles.find((role) => role.id === selectedRoleId) ?? null;
  const permissions = permissionsQuery.data ?? [];
  const permissionGroups = React.useMemo(() => groupPermissions(permissions), [permissions]);
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(h1RoleQueryFields, appliedQuery),
    [appliedQuery],
  );
  const busy = createMutation.isPending || updateMutation.isPending || deleteMutation.isPending || permissionsMutation.isPending || batchMutation.isPending || createUserMutation.isPending;

  React.useEffect(() => {
    const next = selectedRoleId && roles.some((role) => role.id === selectedRoleId) ? selectedRoleId : roles[0]?.id ?? null;
    if (next !== selectedRoleId) setSelectedRoleId(next);
  }, [roles, selectedRoleId]);

  // 仅当选中角色变化、或该角色服务端权限集版本变化（刷新 / 保存后）时才重同步矩阵，
  // 避免 roles 每次刷新拿到相同数据也把用户未保存的勾选清掉。
  const selectedRolePermissionSignature = selectedRole ? selectedRole.permission_codes.join("\n") : null;

  React.useEffect(() => {
    setSelectedRowKeys(selectedRoleId ? [selectedRoleId] : []);
    setPermissionCodes(
      selectedRolePermissionSignature === null
        ? []
        : selectedRolePermissionSignature.split("\n").filter(Boolean),
    );
  }, [selectedRoleId, selectedRolePermissionSignature]);

  function selectRole(role: Role) {
    setSelectedRoleId(role.id);
    setSelectedRowKeys([role.id]);
  }

  function openRoleDialog(role: Role | null) {
    openRoleFormWith(role ? roleFormFromRole(role) : emptyRoleForm());
  }

  async function saveRole() {
    const roleName = roleForm.roleName.trim();
    const roleCode = roleForm.roleCode.trim();
    if (!roleName || (!roleForm.id && !roleCode) || !roleForm.dataScope) {
      setNotice({ type: "error", text: "请填写角色编码、角色名称和数据范围" });
      return;
    }
    setNotice(null);
    try {
      const role = roleForm.id
        ? await updateMutation.mutateAsync({
            id: roleForm.id,
            body: { role_name: roleName, data_scope: roleForm.dataScope, parent_role_id: roleForm.parentRoleId || null },
          })
        : await createMutation.mutateAsync({
            role_code: roleCode,
            role_name: roleName,
            data_scope: roleForm.dataScope,
            parent_role_id: roleForm.parentRoleId || null,
          });
      setSelectedRoleId(role.id);
      setRoleDialogOpen(false);
      setNotice({ type: "success", text: `${role.role_name} 已${roleForm.id ? "修改" : "新增"}` });
    } catch (error) {
      setNotice({ type: "error", text: error instanceof Error ? error.message : "保存角色失败" });
    }
  }

  async function deleteSelectedRole() {
    if (!selectedRole) return;
    setNotice(null);
    try {
      await deleteMutation.mutateAsync(selectedRole.id);
      setDeleteDialogOpen(false);
      setSelectedRoleId(null);
      setNotice({ type: "success", text: `${selectedRole.role_name} 已删除` });
    } catch (error) {
      setNotice({ type: "error", text: error instanceof Error ? error.message : "删除角色失败" });
    }
  }

  async function savePermissions() {
    if (!selectedRole) return;
    setNotice(null);
    try {
      const role = await permissionsMutation.mutateAsync({ id: selectedRole.id, permissionCodes: permissionCodes.slice().sort() });
      setPermissionCodes(role.permission_codes);
      setNotice({ type: "success", text: `${role.role_name} 权限已原子保存` });
    } catch (error) {
      setNotice({ type: "error", text: error instanceof Error ? error.message : "保存权限矩阵失败" });
    }
  }

  function applyGridQueryState(state: unknown) {
    applyQuery(queryValueFromUnknown(state));
  }

  function clearGridQueryState() {
    resetQuery();
  }

  async function refreshAll() {
    const results = await Promise.all([rolesQuery.refetch(), permissionsQuery.refetch(), usersQuery.refetch()]);
    setNotice(results.some((result) => result.error) ? { type: "error", text: "刷新角色权限数据失败" } : { type: "success", text: "角色权限数据已刷新" });
  }

  function openBatchDialog() {
    setSelectedUserIds([]);
    setAssignRoleIds(selectedRole ? [selectedRole.id] : []);
    setBatchDialogOpen(true);
  }

  async function saveBatchAssignments() {
    if (selectedUserIds.length === 0 || assignRoleIds.length === 0) {
      setNotice({ type: "error", text: "请至少选择一名用户和一个角色" });
      return;
    }
    setNotice(null);
    try {
      await batchMutation.mutateAsync({ user_ids: selectedUserIds, role_ids: assignRoleIds });
      setBatchDialogOpen(false);
      setNotice({ type: "success", text: `已为 ${selectedUserIds.length} 名用户批量分配 ${assignRoleIds.length} 个角色` });
    } catch (error) {
      setNotice({ type: "error", text: error instanceof Error ? error.message : "批量分配角色失败" });
    }
  }

  async function saveUser(form: CreateUserRequest) {
    const username = form.username.trim();
    const displayName = form.display_name.trim();
    const phone = form.phone.trim();
    if (!username || !displayName || phone.length < 7 || form.password.length < 8 || form.role_ids.length === 0) {
      setNotice({ type: "error", text: "请填写账号、姓名、有效手机号、至少 8 位密码并选择角色" });
      return;
    }
    setNotice(null);
    try {
      const user = await createUserMutation.mutateAsync({ ...form, username, display_name: displayName, phone });
      setCreateUserDialogOpen(false);
      setNotice({ type: "success", text: `${user.display_name} 已新增` });
    } catch (error) {
      setNotice({ type: "error", text: error instanceof Error ? error.message : "新增用户失败" });
    }
  }

  const columns = React.useMemo<DataGridColumn<Role>[]>(
    () => roleColumns(roles),
    [roles],
  );
  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新角色、权限和用户数据",
    disabled: rolesQuery.isFetching || permissionsQuery.isFetching || usersQuery.isFetching,
    onClick: () => void refreshAll(),
  };
  const toolbarActions: DataGridToolbarAction[] = canManage
    ? [
        { key: "create-user", label: "新增", description: "新增用户并绑定角色", icon: <UsersRound className="size-4" aria-hidden />, onClick: () => setCreateUserDialogOpen(true) },
        { key: "batch-assign", label: "授权", description: "为多个用户分配多个角色", icon: <UsersRound className="size-4" aria-hidden />, onClick: openBatchDialog },
      ]
    : [];

  if (!canManage) {
    return (
      <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
        <PageHeader title="H1 角色权限" subtitle="角色、权限矩阵与用户批量授权" />
        <Card><CardContent className="p-6 text-sm text-muted-foreground" role="alert">当前账号没有 h1.roles.manage 权限，角色权限操作已隐藏。</CardContent></Card>
      </section>
    );
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader title="H1 角色权限" subtitle="角色、权限矩阵与用户批量授权；写入由 h1.roles.manage 控制" />
      {notice ? <NoticePanel notice={notice} /> : null}
      <QueryPanel
        fields={h1RoleQueryFields}
        defaultVisibleFieldKeys={h1RoleCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={(next) => setDraftQuery(normalizeRoleQuery(next))}
        onQuery={() => applyQuery(draftQuery)}
        onReset={resetQuery}
      />
      <RoleQueryError rolesQuery={rolesQuery} permissionsQuery={permissionsQuery} usersQuery={usersQuery} />
      <div className="grid gap-4 xl:grid-cols-[minmax(32rem,1.1fr)_minmax(28rem,0.9fr)]">
        <Card className="min-w-0 rounded-lg shadow-sm">
          <CardContent className="space-y-3 p-4">
            <div className="flex items-center justify-between gap-3"><h2 className="text-base font-semibold">角色列表</h2><span className="text-xs text-muted-foreground">默认角色 {roles.length}</span></div>
            <DataGrid
              columns={columns}
              data={filteredRoles}
              rowKey={(row) => row.id}
              selectedKey={selectedRoleId ?? undefined}
              selectedRowKeys={selectedRowKeys}
              onSelectedRowKeysChange={(keys) => { setSelectedRowKeys(keys); if (keys.at(-1)) setSelectedRoleId(keys.at(-1) ?? null); }}
              onRowClick={selectRole}
              selectable
              caption={rolesQuery.isPending ? "加载角色列表..." : undefined}
              emptyTitle={rolesQuery.isError ? "读取角色列表失败" : "暂无角色"}
              emptyDescription="调整查询条件或先新增角色。"
              storageKey="h1-role-permission-roles"
              exportFileBaseName="H1 角色权限"
              refreshAction={refreshAction}
              createAction={{ label: "新增", description: "新增角色", onClick: () => openRoleDialog(null) }}
              editAction={{ label: "修改", description: "修改选中角色", disabled: ({ selectedRowKeys: keys }) => keys.length !== 1 || busy, onClick: ({ selectedRowKeys: keys }) => { const role = roles.find((item) => item.id === keys[0]); if (role) openRoleDialog(role); } }}
              deleteAction={{ label: "删除", description: "删除选中角色", disabled: ({ selectedRowKeys: keys }) => keys.length !== 1 || busy, onClick: ({ selectedRowKeys: keys }) => { const role = roles.find((item) => item.id === keys[0]); if (role) { setSelectedRoleId(role.id); setDeleteDialogOpen(true); } } }}
              toolbarActions={toolbarActions}
              queryState={appliedQuery}
              querySummaryItems={querySummaryItems}
              onApplyQueryState={applyGridQueryState}
              onClearQueryState={clearGridQueryState}
              tableClassName="min-w-[760px]"
            />
          </CardContent>
        </Card>
        <PermissionMatrix
          role={selectedRole}
          groups={permissionGroups}
          permissionCodes={permissionCodes}
          loading={permissionsQuery.isPending}
          saving={permissionsMutation.isPending}
          onToggle={(code) => setPermissionCodes((current) => current.includes(code) ? current.filter((item) => item !== code) : [...current, code])}
          onToggleGroup={(codes) => setPermissionCodes((current) => toggleCodes(current, codes))}
          onSave={() => void savePermissions()}
        />
      </div>
      <RoleDialog open={roleDialogOpen} form={roleForm} roles={roles} saving={createMutation.isPending || updateMutation.isPending} onOpenChange={setRoleDialogOpen} onFormChange={setRoleForm} onSave={() => void saveRole()} />
      <DeleteRoleDialog open={deleteDialogOpen} role={selectedRole} deleting={deleteMutation.isPending} onOpenChange={setDeleteDialogOpen} onDelete={() => void deleteSelectedRole()} />
      <BatchAssignDialog open={batchDialogOpen} users={users} roles={roles} loading={usersQuery.isPending} selectedUserIds={selectedUserIds} selectedRoleIds={assignRoleIds} saving={batchMutation.isPending} onOpenChange={setBatchDialogOpen} onUsersChange={setSelectedUserIds} onRoleToggle={(id) => setAssignRoleIds((current) => current.includes(id) ? current.filter((item) => item !== id) : [...current, id])} onSave={() => void saveBatchAssignments()} />
      <CreateUserDialog open={createUserDialogOpen} roles={roles} saving={createUserMutation.isPending} onOpenChange={setCreateUserDialogOpen} onSave={(form) => void saveUser(form)} />
    </section>
  );
}

function NoticePanel({ notice }: { notice: Notice }) {
  if (!notice) return null;
  return <div className={notice.type === "error" ? "rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" : "rounded-md border border-wms-success/30 bg-wms-success/10 px-4 py-3 text-sm text-wms-success"} role={notice.type === "error" ? "alert" : "status"}>{notice.text}</div>;
}

function RoleQueryError({ rolesQuery, permissionsQuery, usersQuery }: { rolesQuery: { error: Error | null }; permissionsQuery: { error: Error | null }; usersQuery: { error: Error | null } }) {
  const error = rolesQuery.error ?? permissionsQuery.error ?? usersQuery.error;
  return error ? <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">{error.message}</div> : null;
}

interface PermissionGroup {
  key: string;
  label: string;
  items: Permission[];
}

function PermissionMatrix({
  role,
  groups,
  permissionCodes,
  loading,
  saving,
  onToggle,
  onToggleGroup,
  onSave,
}: {
  role: Role | null;
  groups: PermissionGroup[];
  permissionCodes: string[];
  loading: boolean;
  saving: boolean;
  onToggle: (code: string) => void;
  onToggleGroup: (codes: string[]) => void;
  onSave: () => void;
}) {
  const [openGroups, setOpenGroups] = React.useState<Record<string, boolean>>({});
  const selected = new Set(permissionCodes);
  return (
    <Card className="min-w-0 rounded-lg shadow-sm">
      <CardContent className="space-y-3 p-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 className="flex items-center gap-2 text-base font-semibold"><ShieldCheck className="size-4" aria-hidden />权限矩阵</h2>
            <p className="text-xs text-muted-foreground">{role ? `${role.role_name} · ${permissionCodes.length} 项；保存时完整替换当前角色权限集` : "请选择角色查看权限"}</p>
          </div>
          {role ? <Button type="button" size="sm" disabled={loading || saving} onClick={onSave}>{saving ? "保存中..." : "保存权限"}</Button> : null}
        </div>
        {loading ? <div className="rounded-md border border-dashed p-5 text-sm text-muted-foreground" role="status">加载权限目录...</div> : null}
        {!loading && groups.length === 0 ? <div className="rounded-md border border-dashed p-5 text-sm text-muted-foreground">暂无权限目录</div> : null}
        {!loading && groups.length > 0 ? (
          <ul className="max-h-[34rem] space-y-2 overflow-auto" role="tree" aria-label="权限树">
            {groups.map((group) => {
              const open = openGroups[group.key] ?? true;
              const groupSelected = group.items.filter((item) => selected.has(item.permission_code)).length;
              return (
                <li key={group.key} className="rounded-md border" role="treeitem" aria-expanded={open}>
                  <div className="flex items-center gap-2 bg-muted/30 px-3 py-2">
                    <button type="button" className="rounded p-1 hover:bg-muted" aria-label={`${open ? "收起" : "展开"}${group.label}`} onClick={() => setOpenGroups((current) => ({ ...current, [group.key]: !open }))}>
                      {open ? <ChevronDown className="size-4" aria-hidden /> : <ChevronRight className="size-4" aria-hidden />}
                    </button>
                    <span className="min-w-0 flex-1 text-sm font-medium">{group.label}</span>
                    <span className="text-xs text-muted-foreground">{groupSelected}/{group.items.length}</span>
                    <Button type="button" variant="ghost" size="sm" onClick={() => onToggleGroup(group.items.map((item) => item.permission_code))}>{groupSelected === group.items.length ? "清空" : "全选"}</Button>
                  </div>
                  {open ? <ul className="divide-y" role="group">{group.items.map((item) => <li key={item.id} className="flex items-center gap-3 px-3 py-2 text-sm hover:bg-muted/30"><Checkbox checked={selected.has(item.permission_code)} onCheckedChange={() => onToggle(item.permission_code)} aria-label={`权限 ${item.permission_name}`} /><code className="w-48 shrink-0 font-mono text-xs text-muted-foreground">{item.permission_code}</code><span className="min-w-0 truncate">{item.permission_name}</span></li>)}</ul> : null}
                </li>
              );
            })}
          </ul>
        ) : null}
      </CardContent>
    </Card>
  );
}

function RoleDialog({
  open,
  form,
  roles,
  saving,
  onOpenChange,
  onFormChange,
  onSave,
}: {
  open: boolean;
  form: RoleForm;
  roles: Role[];
  saving: boolean;
  onOpenChange: (open: boolean) => void;
  onFormChange: (form: RoleForm) => void;
  onSave: () => void;
}) {
  const parents = roles.filter((role) => role.id !== form.id);
  const editing = Boolean(form.id);
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>{editing ? "修改角色" : "新增角色"}</DialogTitle>
          <DialogDescription>{editing ? `修改 ${form.roleName || "当前角色"} 的父角色与数据范围。` : "填写角色编码、名称、父角色和数据范围。"}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-3 md:grid-cols-2">
          <Field label="角色编码"><Input value={form.roleCode} disabled={editing} onChange={(event) => onFormChange({ ...form, roleCode: event.target.value })} aria-label="角色编码" /></Field>
          <Field label="角色名称"><Input value={form.roleName} onChange={(event) => onFormChange({ ...form, roleName: event.target.value })} aria-label="角色名称" /></Field>
          <Field label="父角色"><select value={form.parentRoleId} onChange={(event) => onFormChange({ ...form, parentRoleId: event.target.value })} aria-label="父角色" className="h-9 rounded-md border border-input bg-background px-3 text-sm"><option value="">无父角色</option>{parents.map((role) => <option key={role.id} value={role.id}>{role.role_name}（{role.role_code}）</option>)}</select></Field>
          <Field label="数据范围"><select value={form.dataScope} onChange={(event) => onFormChange({ ...form, dataScope: event.target.value })} aria-label="数据范围" className="h-9 rounded-md border border-input bg-background px-3 text-sm"><option value="">请选择</option>{DATA_SCOPE_OPTIONS.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select></Field>
        </div>
        <DialogFooter><Button type="button" variant="outline" disabled={saving} onClick={() => onOpenChange(false)}>取消</Button><Button type="button" disabled={saving} onClick={onSave}>{saving ? "保存中..." : editing ? "保存修改" : "新增角色"}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function DeleteRoleDialog({
  open,
  role,
  deleting,
  onOpenChange,
  onDelete,
}: {
  open: boolean;
  role: Role | null;
  deleting: boolean;
  onOpenChange: (open: boolean) => void;
  onDelete: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader><DialogTitle>删除角色</DialogTitle><DialogDescription>{role ? `确认删除角色「${role.role_name}」？已绑定用户或作为父角色时后端会拒绝删除。` : "未选择角色。"}</DialogDescription></DialogHeader>
        <DialogFooter><Button type="button" variant="outline" disabled={deleting} onClick={() => onOpenChange(false)}>取消</Button><Button type="button" variant="destructive" disabled={!role || deleting} onClick={onDelete}>{deleting ? "删除中..." : "确认删除"}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function BatchAssignDialog({
  open,
  users,
  roles,
  loading,
  selectedUserIds,
  selectedRoleIds,
  saving,
  onOpenChange,
  onUsersChange,
  onRoleToggle,
  onSave,
}: {
  open: boolean;
  users: RoleUser[];
  roles: Role[];
  loading: boolean;
  selectedUserIds: string[];
  selectedRoleIds: string[];
  saving: boolean;
  onOpenChange: (open: boolean) => void;
  onUsersChange: (ids: string[]) => void;
  onRoleToggle: (id: string) => void;
  onSave: () => void;
}) {
  const columns = React.useMemo<DataGridColumn<RoleUser>[]>(() => [
    { key: "username", header: "账号", width: 180, sortable: true, copyable: true },
    { key: "display_name", header: "姓名", width: 160, sortable: true, copyable: true },
    { key: "role_count", header: "当前角色数", width: 130, sortable: true, sortValue: (row) => row.role_ids.length, render: (row) => `${row.role_ids.length} 个` },
  ], []);
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] max-w-5xl overflow-y-auto">
        <DialogHeader><DialogTitle>批量授权</DialogTitle><DialogDescription>先选择用户，再勾选多个角色；提交后会原子替换这些用户在当前货主下的角色集合。</DialogDescription></DialogHeader>
        <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_18rem]">
          <div className="space-y-2"><h3 className="text-sm font-semibold">用户（已选 {selectedUserIds.length}）</h3>{loading ? <div className="rounded-md border border-dashed p-5 text-sm text-muted-foreground" role="status">加载用户列表...</div> : <DataGrid columns={columns} data={users} rowKey={(row) => row.user_id} selectedRowKeys={selectedUserIds} onSelectedRowKeysChange={onUsersChange} selectable storageKey="h1-role-permission-users" defaultPageSize={10} emptyTitle="暂无可授权用户" emptyDescription="当前货主没有可用用户。" tableClassName="min-w-[470px]" />}</div>
          <div className="space-y-2"><h3 className="text-sm font-semibold">角色（已选 {selectedRoleIds.length}）</h3><div className="space-y-2 rounded-md border p-3">{roles.length === 0 ? <p className="text-sm text-muted-foreground">暂无角色</p> : roles.map((role) => <label key={role.id} className="flex items-start gap-2 text-sm"><Checkbox checked={selectedRoleIds.includes(role.id)} onCheckedChange={() => onRoleToggle(role.id)} /><span className="min-w-0"><span className="block truncate">{role.role_name}</span><span className="block font-mono text-xs text-muted-foreground">{role.role_code}</span></span></label>)}</div></div>
        </div>
        <DialogFooter><Button type="button" variant="outline" disabled={saving} onClick={() => onOpenChange(false)}>取消</Button><Button type="button" disabled={saving || selectedUserIds.length === 0 || selectedRoleIds.length === 0} onClick={onSave}>{saving ? "提交中..." : "确认批量授权"}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="grid gap-1 text-sm"><span className="text-xs text-muted-foreground">{label}</span>{children}</label>;
}

function roleColumns(roles: Role[]): DataGridColumn<Role>[] {
  const names = new Map(roles.map((role) => [role.id, role.role_name]));
  return [
    { key: "role_name", header: "角色名称", width: 180, sortable: true, copyable: true },
    { key: "role_code", header: "角色编码", width: 190, sortable: true, copyable: true, mono: true },
    { key: "data_scope", header: "数据范围", width: 130, sortable: true, sortValue: (row) => dataScopeLabel(row.data_scope), filterValue: (row) => row.data_scope, filter: { type: "multiSelect", options: DATA_SCOPE_OPTIONS }, render: (row) => <StatusBadge status="completed" label={dataScopeLabel(row.data_scope)} size="sm" /> },
    { key: "parent_role_id", header: "父角色", width: 170, sortable: true, sortValue: (row) => names.get(row.parent_role_id ?? "") ?? "", render: (row) => row.parent_role_id ? names.get(row.parent_role_id) ?? "未知角色" : "无" },
    { key: "permission_count", header: "权限数", width: 110, align: "right", sortable: true, sortValue: (row) => row.permission_codes.length, render: (row) => `${row.permission_codes.length} 项` },
    { key: "created_at", header: "创建时间", width: 170, sortable: true, copyable: true, render: (row) => formatDateTime(row.created_at) },
  ];
}

function groupPermissions(permissions: Permission[]): PermissionGroup[] {
  const groups = new Map<string, Permission[]>();
  for (const permission of permissions) {
    const key = permission.permission_code.split(".")[0] || "other";
    groups.set(key, [...(groups.get(key) ?? []), permission]);
  }
  return Array.from(groups, ([key, items]) => ({ key, label: permissionGroupLabel(key), items })).sort((left, right) => left.key.localeCompare(right.key));
}

function permissionGroupLabel(key: string) {
  const labels: Record<string, string> = { h1: "H1 权限租户", h2: "H2 审计", h3: "H3 契约", h4: "H4 企业微信", h5: "H5 快递", h9: "H9 打印", m1: "M1 基础档案", m2: "M2 入库", m3: "M3 库内", m4: "M4 出库" };
  return labels[key] ?? key.toUpperCase();
}

function toggleCodes(current: string[], codes: string[]) {
  const set = new Set(current);
  const allSelected = codes.every((code) => set.has(code));
  codes.forEach((code) => allSelected ? set.delete(code) : set.add(code));
  return Array.from(set);
}

function defaultRoleQuery(): RoleQuery {
  return { keyword: "", dataScope: "" };
}

function normalizeRoleQuery(value: QueryPanelValue): RoleQuery {
  return { keyword: queryString(value.keyword), dataScope: queryString(value.dataScope) };
}

function filterRoles(roles: Role[], query: RoleQuery) {
  const keyword = query.keyword.trim().toLowerCase();
  return roles.filter((role) => (!keyword || `${role.role_code} ${role.role_name}`.toLowerCase().includes(keyword)) && (!query.dataScope || role.data_scope === query.dataScope));
}

function emptyRoleForm(): RoleForm {
  return { id: null, roleCode: "", roleName: "", dataScope: "owner", parentRoleId: "" };
}

function roleFormFromRole(role: Role): RoleForm {
  return { id: role.id, roleCode: role.role_code, roleName: role.role_name, dataScope: role.data_scope, parentRoleId: role.parent_role_id ?? "" };
}

function dataScopeLabel(value: string) {
  return DATA_SCOPE_OPTIONS.find((option) => option.value === value)?.label ?? value;
}

function formatDateTime(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { hour12: false });
}
