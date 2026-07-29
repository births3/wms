import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  DataGrid,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  PageHeader,
  QueryPanel,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridDisableAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { Ban, LogOut, UserX } from "lucide-react";

import {
  useAuthSessionsQuery,
  useKickAuthUserMutation,
  useRevokeAuthSessionMutation,
  useRevokeOtherAuthSessionsMutation,
  type AuthSession,
  type CurrentUser,
} from "@/features/auth/auth-queries";
import { usePageQueryState } from "@/lib/use-page-query-state";

const sessionQueryFields: QueryPanelField[] = [
  {
    key: "targetUserId",
    label: "目标用户 ID",
    type: "text",
    placeholder: "仅会话管理员可查询其他用户",
    ariaLabel: "目标用户 ID",
  },
];
const sessionCoreQueryFieldKeys = ["targetUserId"];

type SessionAction = "revoke" | "revokeOthers" | "kick" | null;

interface H1SessionPageProps {
  currentUser: CurrentUser;
}

export function H1SessionPage({ currentUser }: H1SessionPageProps) {
  const canManage = currentUser.permissions.includes("h1.sessions.manage");
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(() => ({ targetUserId: "" }));
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [dialogAction, setDialogAction] = React.useState<SessionAction>(null);
  const [actionError, setActionError] = React.useState<string | null>(null);
  const isDisableDialog = dialogAction === "revoke";
  const targetUserId = canManage ? queryText(appliedQuery.targetUserId) : "";
  const sessionsQuery = useAuthSessionsQuery(targetUserId || undefined);
  const revokeSessionMutation = useRevokeAuthSessionMutation();
  const revokeOthersMutation = useRevokeOtherAuthSessionsMutation();
  const kickUserMutation = useKickAuthUserMutation();
  const sessions = sessionsQuery.data ?? [];
  const selectedSession = sessions.find((session) => session.session_id === selectedRowKeys[0]);
  const busy =
    revokeSessionMutation.isPending || revokeOthersMutation.isPending || kickUserMutation.isPending;

  const revokeAction: DataGridDisableAction = {
    label: "失效设备",
    description: "撤销选中的单设备登录会话",
    disabled: (context) =>
      Boolean(targetUserId) ||
      context.selectedRowKeys.length !== 1 ||
      sessions.find((session) => session.session_id === context.selectedRowKeys[0])?.is_current === true,
    onClick: (context) => {
      setSelectedRowKeys(context.selectedRowKeys);
      openDialog("revoke");
    },
  };

  /** 打开确认弹窗时清空上一次遗留的错误文案。 */
  function openDialog(action: SessionAction) {
    setActionError(null);
    setDialogAction(action);
  }

  async function confirmAction() {
    setActionError(null);
    try {
      if (isDisableDialog && selectedSession) {
        await revokeSessionMutation.mutateAsync(selectedSession.session_id);
      } else if (dialogAction === "revokeOthers") {
        await revokeOthersMutation.mutateAsync();
      } else if (dialogAction === "kick" && targetUserId) {
        await kickUserMutation.mutateAsync(targetUserId);
      }
      setDialogAction(null);
      setSelectedRowKeys([]);
      void sessionsQuery.refetch();
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "会话失效失败");
    }
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="H1 登录会话"
        subtitle="Token 失效、设备会话与管理员强制踢人 · Redis 黑名单直至 token 到期"
      />

      <Card className="rounded-lg border-primary/20 bg-primary/5 shadow-sm">
        <CardContent className="flex flex-wrap items-center justify-between gap-4 p-5">
          <div>
            <p className="text-sm font-semibold">当前登录用户</p>
            <p className="mt-1 text-sm text-muted-foreground">
              {currentUser.display_name}（{currentUser.username}）· 当前货主 {currentUser.owner_code}
            </p>
          </div>
          <Button type="button" variant="outline" onClick={() => openDialog("revokeOthers")} disabled={busy}>
            <LogOut className="size-4" aria-hidden />
            登出其他设备
          </Button>
        </CardContent>
      </Card>

      <QueryPanel
        fields={sessionQueryFields}
        defaultVisibleFieldKeys={sessionCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => {
          applyQuery(draftQuery);
          setSelectedRowKeys([]);
        }}
        onReset={() => {
          resetQuery();
          setSelectedRowKeys([]);
        }}
        resetLabel="重置"
        actions={
          canManage && targetUserId ? (
            <Button type="button" variant="destructive" onClick={() => openDialog("kick")} disabled={busy}>
              <UserX className="size-4" aria-hidden />
              踢出目标用户
            </Button>
          ) : undefined
        }
      />

      <DataGrid
        storageKey="h1.auth-sessions"
        columns={sessionColumns}
        data={sessions}
        rowKey={(row) => row.session_id}
        selectable
        selectedRowKeys={selectedRowKeys}
        onSelectedRowKeysChange={setSelectedRowKeys}
        disableAction={!targetUserId ? revokeAction : undefined}
        refreshAction={{
          label: "刷新",
          description: "重新加载活跃登录会话",
          disabled: sessionsQuery.isFetching,
          onClick: () => void sessionsQuery.refetch(),
        }}
        queryState={appliedQuery}
        querySummaryItems={buildQueryPanelSummaryItems(sessionQueryFields, appliedQuery)}
        caption={sessionsQuery.isPending ? "加载会话..." : `当前 ${sessions.length} 个活跃会话`}
        emptyTitle={sessionsQuery.isError ? "读取会话失败" : "暂无活跃登录会话"}
        emptyDescription={sessionsQuery.isError ? sessionsQuery.error.message : "登录后产生的会话会显示在这里"}
        exportFileBaseName="H1 登录会话"
        tableClassName="min-w-[980px]"
      />

      <p className="text-xs text-muted-foreground">
        会话数据来自真实 API；管理员只能查看当前货主内用户，单设备失效仅允许用户操作自己的非当前会话。
      </p>

      <Dialog open={dialogAction !== null} onOpenChange={(open) => !open && setDialogAction(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{dialogTitle(dialogAction)}</DialogTitle>
            <DialogDescription>{dialogDescription(dialogAction, selectedSession, targetUserId)}</DialogDescription>
          </DialogHeader>
          {actionError && <p role="alert" className="text-sm text-destructive">{actionError}</p>}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline" disabled={busy}>取消</Button>
            </DialogClose>
            <Button type="button" variant="destructive" onClick={() => void confirmAction()} disabled={busy}>
              <Ban className="size-4" aria-hidden />
              确认失效
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}

const sessionColumns: DataGridColumn<AuthSession>[] = [
  {
    key: "device",
    header: "设备 / 客户端",
    width: 230,
    minWidth: 180,
    sortable: true,
    sortValue: (row) => row.device_name,
    filterValue: (row) => row.device_name,
    copyValue: (row) => row.device_name,
    filter: { type: "text" },
    render: (row) => (
      <div className="min-w-0">
        <div className="truncate font-medium">{row.device_name}</div>
        {row.is_current && <div className="text-xs text-primary">当前设备</div>}
      </div>
    ),
  },
  {
    key: "user",
    header: "用户 ID",
    width: 250,
    minWidth: 210,
    mono: true,
    filterValue: (row) => row.user_id,
    copyValue: (row) => row.user_id,
    filter: { type: "text" },
    render: (row) => row.user_id,
  },
  {
    key: "ip",
    header: "IP",
    width: 150,
    minWidth: 120,
    mono: true,
    render: (row) => row.ip ?? "—",
  },
  {
    key: "created_at",
    header: "创建时间",
    width: 190,
    minWidth: 170,
    sortable: true,
    sortValue: (row) => row.logged_in_at,
    render: (row) => formatDateTime(row.logged_in_at),
  },
  {
    key: "expiresAt",
    header: "过期时间",
    width: 190,
    minWidth: 170,
    sortable: true,
    sortValue: (row) => row.expires_at,
    render: (row) => formatDateTime(row.expires_at),
  },
];

function queryText(value: unknown) {
  return typeof value === "string" ? value.trim() : "";
}

function formatDateTime(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { hour12: false });
}

function dialogTitle(action: SessionAction) {
  if (action === "revoke") return "确认失效此设备";
  if (action === "kick") return "确认踢出目标用户";
  return "确认登出其他设备";
}

function dialogDescription(action: SessionAction, session: AuthSession | undefined, targetUserId: string) {
  if (action === "revoke") return `设备「${session?.device_name ?? "未知设备"}」将立即失效。`;
  if (action === "kick") return `用户 ${targetUserId} 的所有活跃 token 将立即失效，并写入审计。`;
  return "当前用户除本设备外的所有活跃 token 将立即失效，并写入审计。";
}
