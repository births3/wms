import { useState, type FormEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, ShieldCheck, UserRound } from "lucide-react";
import { Button, Checkbox, Input, Label, StatusBadge } from "@wms/ui";

import { createUser, listAddresses, listUsers, updateUser } from "./api";
import type { LoginResponse, PortalUser } from "./types";

export function UsersPage(props: { session: LoginResponse }) {
  const token = props.session.access_token;
  const userId = props.session.user.id;
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const users = useQuery({
    queryKey: ["portal-users", userId],
    queryFn: () => listUsers(token),
  });
  const addresses = useQuery({
    queryKey: ["portal-addresses", userId],
    queryFn: () => listAddresses(token),
  });
  const create = useMutation({
    mutationFn: createUser.bind(null, token),
    onSuccess: async () => {
      setOpen(false);
      await queryClient.invalidateQueries({ queryKey: ["portal-users", userId] });
    },
  });
  const update = useMutation({
    mutationFn: (input: {
      user: PortalUser;
      status?: "active" | "disabled";
      canViewHistory?: boolean;
    }) => updateUser(token, input.user.id, {
      display_name: input.user.display_name,
      role: input.user.role,
      status: input.status ?? (input.user.status === "active" ? "active" : "disabled"),
      can_view_report_history:
        input.canViewHistory ?? input.user.can_view_report_history,
      address_ids: input.user.address_ids,
    }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["portal-users", userId] });
    },
  });

  return (
    <div className="portal-page" data-testid="portal-users-page">
      <section className="portal-page-header">
        <div>
          <div className="portal-eyebrow">客户多账号</div>
          <h1 className="portal-page-title">客户账号</h1>
          <p className="portal-page-description">
            普通账号按一个或多个稳定客户地址授权；未授权地址不返回数据。
          </p>
        </div>
        <Button type="button" onClick={() => setOpen((value) => !value)}>
          <Plus className="mr-2 size-4" />
          新建账号
        </Button>
      </section>

      {open && addresses.isPending ? (
        <div className="portal-form-panel portal-empty-state">正在读取可授权地址…</div>
      ) : open && addresses.error ? (
        <div role="alert" className="portal-alert portal-alert-error">
          {addresses.error.message}
        </div>
      ) : open ? (
        <CreateUserForm
          addresses={addresses.data ?? []}
          pending={create.isPending}
          error={create.error}
          onSubmit={(request) => create.mutate(request)}
          onCancel={() => setOpen(false)}
        />
      ) : null}
      {update.error ? (
        <div role="alert" className="portal-alert portal-alert-error">
          {update.error.message}
        </div>
      ) : null}

      <section className="portal-table-shell">
        <div className="portal-table-toolbar">
          <div>
            <div className="portal-table-title">账号与授权范围</div>
            <div className="portal-table-subtitle">
              共 {users.data?.length ?? 0} 个账号
            </div>
          </div>
        </div>
        {users.isPending ? (
          <div className="portal-empty-state">正在读取客户账号…</div>
        ) : users.error ? (
          <div role="alert" className="portal-alert portal-alert-error">
            {users.error.message}
          </div>
        ) : users.data?.length ? (
          <div className="overflow-x-auto">
          <table className="portal-table portal-responsive-table">
            <thead>
              <tr>
                <th>账号</th>
                <th>显示名称</th>
                <th>角色</th>
                <th>地址范围</th>
                <th>历史版本</th>
                <th>状态</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              {users.data?.map((user) => (
                <tr key={user.id}>
                  <td data-label="账号" className="font-medium">{user.username}</td>
                  <td data-label="显示名称">{user.display_name}</td>
                  <td data-label="角色">
                    <span className="flex items-center gap-2">
                      {user.role === "customer_admin" ? (
                        <ShieldCheck className="size-4 text-emerald-700" />
                      ) : (
                        <UserRound className="size-4 text-slate-500" />
                      )}
                      {user.role === "customer_admin" ? "客户管理员" : "普通账号"}
                    </span>
                  </td>
                  <td data-label="地址范围">
                    {user.role === "customer_admin"
                      ? "全部客户地址"
                      : `${user.address_ids.length} 个地址`}
                  </td>
                  <td data-label="历史版本">
                    <StatusBadge
                      status={user.can_view_report_history ? "completed" : "isolated"}
                      size="sm"
                      label={user.can_view_report_history ? "可查看" : "仅当前版本"}
                    />
                  </td>
                  <td data-label="状态">
                    <StatusBadge
                      status={user.status === "active" ? "completed" : "pending"}
                      size="sm"
                      label={user.status === "active" ? "启用" : user.status === "locked" ? "已锁定" : "停用"}
                    />
                  </td>
                  <td data-mobile="action">
                    <div className="flex flex-wrap gap-2">
                      <Button
                        type="button"
                        size="sm"
                        variant="outline"
                        disabled={update.isPending || user.id === props.session.user.id || user.status === "locked"}
                        onClick={() => update.mutate({
                          user,
                          status: user.status === "active" ? "disabled" : "active",
                        })}
                      >
                        {user.status === "active" ? "停用" : "启用"}
                      </Button>
                      <Button
                        type="button"
                        size="sm"
                        variant="outline"
                        disabled={update.isPending}
                        onClick={() => update.mutate({
                          user,
                          canViewHistory: !user.can_view_report_history,
                        })}
                      >
                        {user.can_view_report_history ? "关闭历史" : "开启历史"}
                      </Button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          </div>
        ) : (
          <div className="portal-empty-state">暂无客户账号</div>
        )}
      </section>
    </div>
  );
}

function CreateUserForm(props: {
  addresses: { id: string; address_code: string; address_name: string }[];
  pending: boolean;
  error: Error | null;
  onSubmit: (request: {
    username: string;
    display_name: string;
    password: string;
    role: "customer_admin" | "customer_user";
    can_view_report_history: boolean;
    address_ids: string[];
  }) => void;
  onCancel: () => void;
}) {
  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<"customer_admin" | "customer_user">("customer_user");
  const [history, setHistory] = useState(false);
  const [addressIds, setAddressIds] = useState<string[]>([]);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    props.onSubmit({
      username,
      display_name: displayName,
      password,
      role,
      can_view_report_history: history,
      address_ids: role === "customer_admin" ? [] : addressIds,
    });
  };
  return (
    <form
      className="portal-form-panel"
      onSubmit={submit}
    >
      <div className="portal-form-heading">
        <div>
          <div className="portal-table-title">创建客户账号</div>
          <div className="portal-table-subtitle">
            账号权限仅在所选地址范围内生效
          </div>
        </div>
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        <Field label="用户名">
          <Input value={username} onChange={(event) => setUsername(event.target.value)} required />
        </Field>
        <Field label="显示名称">
          <Input value={displayName} onChange={(event) => setDisplayName(event.target.value)} required />
        </Field>
        <Field label="初始密码">
          <Input
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            placeholder="至少12位，含大小写字母和数字"
            required
          />
        </Field>
        <Field label="角色">
          <select
            className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm"
            value={role}
            onChange={(event) =>
              setRole(event.target.value as "customer_admin" | "customer_user")
            }
          >
            <option value="customer_user">普通账号</option>
            <option value="customer_admin">客户管理员</option>
          </select>
        </Field>
      </div>
      {role === "customer_user" ? (
        <div className="mt-4">
          <Label>授权客户地址（至少一个）</Label>
          <div className="mt-2 grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
            {props.addresses.map((address) => (
              <label
                key={address.id}
                className="portal-choice-card"
              >
                <Checkbox
                  checked={addressIds.includes(address.id)}
                  onCheckedChange={(value) =>
                    setAddressIds((current) =>
                      value === true
                        ? [...current, address.id]
                        : current.filter((id) => id !== address.id),
                    )
                  }
                />
                {address.address_code} · {address.address_name}
              </label>
            ))}
          </div>
        </div>
      ) : null}
      <label className="mt-4 flex items-center gap-2 text-sm">
        <Checkbox checked={history} onCheckedChange={(value) => setHistory(value === true)} />
        允许查看药检单历史版本和更正记录
      </label>
      {props.error ? (
        <div role="alert" className="portal-alert portal-alert-error mt-4">
          {props.error.message}
        </div>
      ) : null}
      <div className="mt-5 flex justify-end gap-2">
        <Button type="button" variant="outline" onClick={props.onCancel}>
          取消
        </Button>
        <Button
          type="submit"
          disabled={
            props.pending ||
            !username.trim() ||
            !displayName.trim() ||
            !password ||
            (role === "customer_user" && !addressIds.length)
          }
        >
          {props.pending ? "保存中…" : "保存账号"}
        </Button>
      </div>
    </form>
  );
}

function Field(props: { label: string; children: React.ReactNode }) {
  return (
    <Label className="grid space-y-2">
      <span>{props.label}</span>
      {props.children}
    </Label>
  );
}
