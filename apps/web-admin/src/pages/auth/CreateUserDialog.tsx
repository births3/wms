import * as React from "react";
import {
  Button,
  Checkbox,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
} from "@wms/ui";

import type { Role, CreateUserRequest } from "@/features/auth/role-permission-queries";

type CreateUserForm = CreateUserRequest;

export function CreateUserDialog({
  open,
  roles,
  saving,
  onOpenChange,
  onSave,
}: {
  open: boolean;
  roles: Role[];
  saving: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (form: CreateUserForm) => void;
}) {
  const [form, setForm] = React.useState<CreateUserForm>(emptyForm);

  React.useEffect(() => {
    if (open) setForm(emptyForm);
  }, [open]);

  function toggleRole(roleId: string) {
    setForm((current) => ({
      ...current,
      role_ids: current.role_ids.includes(roleId)
        ? current.role_ids.filter((id) => id !== roleId)
        : [...current.role_ids, roleId],
    }));
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>新增用户</DialogTitle>
          <DialogDescription>填写账号、联系方式、密码并绑定至少一个角色。</DialogDescription>
        </DialogHeader>
        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="登录账号"><Input value={form.username} onChange={(event) => setForm({ ...form, username: event.target.value })} aria-label="登录账号" /></Field>
          <Field label="姓名"><Input value={form.display_name} onChange={(event) => setForm({ ...form, display_name: event.target.value })} aria-label="姓名" /></Field>
          <Field label="手机号"><Input value={form.phone} onChange={(event) => setForm({ ...form, phone: event.target.value })} aria-label="手机号" /></Field>
          <Field label="初始密码"><Input type="password" value={form.password} onChange={(event) => setForm({ ...form, password: event.target.value })} aria-label="初始密码" /></Field>
        </div>
        <fieldset className="grid gap-2">
          <legend className="text-sm font-medium">角色</legend>
          {roles.length === 0 ? <p className="text-sm text-muted-foreground">暂无可绑定角色。</p> : <div className="grid gap-2 sm:grid-cols-2">{roles.map((role) => <label key={role.id} className="flex items-start gap-2 rounded-md border px-3 py-2 text-sm"><Checkbox checked={form.role_ids.includes(role.id)} onCheckedChange={() => toggleRole(role.id)} aria-label={`绑定角色 ${role.role_name}`} /><span><span className="block">{role.role_name}</span><span className="font-mono text-xs text-muted-foreground">{role.role_code}</span></span></label>)}</div>}
        </fieldset>
        <DialogFooter>
          <Button type="button" variant="outline" disabled={saving} onClick={() => onOpenChange(false)}>取消</Button>
          <Button type="button" disabled={saving || roles.length === 0} onClick={() => onSave(form)}>{saving ? "新增中..." : "新增用户"}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

const emptyForm: CreateUserForm = { username: "", display_name: "", phone: "", password: "", role_ids: [] };

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="grid gap-1 text-sm"><span className="text-xs text-muted-foreground">{label}</span>{children}</label>;
}
