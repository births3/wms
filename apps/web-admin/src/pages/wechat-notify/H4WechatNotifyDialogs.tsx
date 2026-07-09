import * as React from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
} from "@wms/ui";

import type { H4NotificationRecord } from "@/features/wechat-notify/wechat-notify-queries";

export type ConfigFormState = {
  eventType: string;
  enabled: boolean;
  channelsText: string;
  usersText: string;
  rolesText: string;
  template: string;
};

export type SendFormState = {
  eventType: string;
  recipientsText: string;
  dedupeKey: string;
  payloadText: string;
};

export type SettingsFormState = {
  corpId: string;
  agentId: string;
  secretAlias: string;
  callbackTokenAlias: string;
  aesKeyAlias: string;
  callbackUrl: string;
  approvalCallbackPath: string;
  enabled: boolean;
  retryMaxAttempts: string;
  retryIntervalSeconds: string;
};

export type Notice = { type: "success" | "error"; text: string } | null;

export function SettingsDialog(props: {
  open: boolean;
  form: SettingsFormState;
  saving: boolean;
  onFormChange: (form: SettingsFormState) => void;
  onOpenChange: (open: boolean) => void;
  onSave: () => void;
}) {
  const { open, form, saving, onFormChange, onOpenChange, onSave } = props;
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl">
        <DialogHeader>
          <DialogTitle>企业微信参数设置</DialogTitle>
          <DialogDescription>维护企业微信应用、回调地址、密钥别名和重试参数；真实密钥不在 WMS 明文保存。</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 md:grid-cols-2">
          <Field label="企业 ID">
            <Input value={form.corpId} onChange={(event) => onFormChange({ ...form, corpId: event.target.value })} />
          </Field>
          <Field label="Agent ID">
            <Input value={form.agentId} onChange={(event) => onFormChange({ ...form, agentId: event.target.value })} />
          </Field>
          <Field label="Secret 别名">
            <Input value={form.secretAlias} onChange={(event) => onFormChange({ ...form, secretAlias: event.target.value })} />
          </Field>
          <Field label="Token 别名">
            <Input value={form.callbackTokenAlias} onChange={(event) => onFormChange({ ...form, callbackTokenAlias: event.target.value })} />
          </Field>
          <Field label="AES Key 别名">
            <Input value={form.aesKeyAlias} onChange={(event) => onFormChange({ ...form, aesKeyAlias: event.target.value })} />
          </Field>
          <Field label="审批回调路径">
            <Input value={form.approvalCallbackPath} onChange={(event) => onFormChange({ ...form, approvalCallbackPath: event.target.value })} />
          </Field>
          <Field label="最大重试次数">
            <Input type="number" min={0} max={10} value={form.retryMaxAttempts} onChange={(event) => onFormChange({ ...form, retryMaxAttempts: event.target.value })} />
          </Field>
          <Field label="重试间隔秒">
            <Input type="number" min={1} max={3600} value={form.retryIntervalSeconds} onChange={(event) => onFormChange({ ...form, retryIntervalSeconds: event.target.value })} />
          </Field>
          <Field label="回调地址" className="md:col-span-2">
            <Input value={form.callbackUrl} onChange={(event) => onFormChange({ ...form, callbackUrl: event.target.value })} />
          </Field>
          <label className="flex items-center gap-2 text-sm text-muted-foreground">
            <input type="checkbox" checked={form.enabled} onChange={(event) => onFormChange({ ...form, enabled: event.target.checked })} />
            启用企业微信通道
          </label>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button type="button" disabled={saving} onClick={onSave}>保存</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function ConfigDialog(props: {
  open: boolean;
  form: ConfigFormState;
  saving: boolean;
  onFormChange: (form: ConfigFormState) => void;
  onOpenChange: (open: boolean) => void;
  onSave: () => void;
}) {
  const { open, form, saving, onFormChange, onOpenChange, onSave } = props;
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>通知配置</DialogTitle>
          <DialogDescription>维护事件类型、模板、企业微信接收人和启停状态。</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 md:grid-cols-2">
          <Field label="事件类型">
            <Input value={form.eventType} onChange={(event) => onFormChange({ ...form, eventType: event.target.value })} />
          </Field>
          <Field label="通知方式">
            <Input value={form.channelsText} onChange={(event) => onFormChange({ ...form, channelsText: event.target.value })} />
          </Field>
          <Field label="接收用户">
            <Input value={form.usersText} onChange={(event) => onFormChange({ ...form, usersText: event.target.value })} />
          </Field>
          <Field label="接收角色">
            <Input value={form.rolesText} onChange={(event) => onFormChange({ ...form, rolesText: event.target.value })} />
          </Field>
          <label className="flex items-center gap-2 text-sm text-muted-foreground">
            <input type="checkbox" checked={form.enabled} onChange={(event) => onFormChange({ ...form, enabled: event.target.checked })} />
            启用配置
          </label>
          <Field label="通知模板" className="md:col-span-2">
            <textarea
              value={form.template}
              onChange={(event) => onFormChange({ ...form, template: event.target.value })}
              className="min-h-28 w-full rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
          </Field>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button type="button" disabled={saving} onClick={onSave}>保存</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function SendDialog(props: {
  open: boolean;
  form: SendFormState;
  sending: boolean;
  onFormChange: (form: SendFormState) => void;
  onOpenChange: (open: boolean) => void;
  onSend: () => void;
}) {
  const { open, form, sending, onFormChange, onOpenChange, onSend } = props;
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>试发通知</DialogTitle>
          <DialogDescription>使用模板变量 payload 渲染内容，并写入企业微信发送记录。</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 md:grid-cols-2">
          <Field label="事件类型">
            <Input value={form.eventType} onChange={(event) => onFormChange({ ...form, eventType: event.target.value })} />
          </Field>
          <Field label="去重键">
            <Input value={form.dedupeKey} onChange={(event) => onFormChange({ ...form, dedupeKey: event.target.value })} />
          </Field>
          <Field label="接收人" className="md:col-span-2">
            <Input value={form.recipientsText} onChange={(event) => onFormChange({ ...form, recipientsText: event.target.value })} />
          </Field>
          <Field label="变量 JSON" className="md:col-span-2">
            <textarea
              value={form.payloadText}
              onChange={(event) => onFormChange({ ...form, payloadText: event.target.value })}
              className="min-h-36 w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
          </Field>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button type="button" disabled={sending} onClick={onSend}>发送</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function RecordDetailDialog({ record, onOpenChange }: { record: H4NotificationRecord | null; onOpenChange: (open: boolean) => void }) {
  return (
    <Dialog open={Boolean(record)} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>发送记录详情</DialogTitle>
          <DialogDescription>查看企业微信通知的发送状态、接收人、去重键和失败原因。</DialogDescription>
        </DialogHeader>
        {record && (
          <div className="grid gap-4">
            <InfoGrid items={[
              ["事件类型", record.event_type],
              ["接收人", record.recipient],
              ["通知方式", record.channel],
              ["状态", statusLabel(record.status)],
              ["重试次数", String(record.retry_count)],
              ["创建时间", formatDateTime(record.created_at)],
              ["发送时间", record.sent_at ? formatDateTime(record.sent_at) : "-"],
              ["去重键", record.dedupe_key],
            ]} />
            <Field label="内容摘要">
              <div className="rounded-md border bg-muted/30 p-3 text-sm">{record.content_summary}</div>
            </Field>
            <Field label="失败原因">
              <div className="rounded-md border bg-muted/30 p-3 text-sm">{record.failure_reason || "-"}</div>
            </Field>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}

export function NoticePanel({ notice }: { notice: Notice }) {
  if (!notice) return null;
  const className = notice.type === "success"
    ? "border-primary/20 bg-primary/5 text-primary"
    : "border-destructive/30 bg-destructive/10 text-destructive";
  return <div className={`rounded-md border px-4 py-3 text-sm ${className}`} role="status">{notice.text}</div>;
}

export function ErrorPanel({ message }: { message: string }) {
  return <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">{message}</div>;
}

function Field({ label, className, children }: { label: string; className?: string; children: React.ReactNode }) {
  return (
    <label className={`grid gap-1.5 text-sm ${className ?? ""}`}>
      <span className="text-xs text-muted-foreground">{label}</span>
      {children}
    </label>
  );
}

function InfoGrid({ items }: { items: Array<[string, string]> }) {
  return (
    <div className="grid gap-3 rounded-md border p-4 md:grid-cols-2">
      {items.map(([label, value]) => (
        <div key={label} className="min-w-0">
          <div className="text-xs text-muted-foreground">{label}</div>
          <div className="truncate text-sm font-medium">{value}</div>
        </div>
      ))}
    </div>
  );
}

function statusLabel(status: string) {
  if (status === "success") return "成功";
  if (status === "failed") return "失败";
  if (status === "retrying") return "重试中";
  return status;
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value || "-";
  return date.toLocaleString("zh-CN", { hour12: false });
}
