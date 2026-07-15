import * as React from "react";
import {
  Button, Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter,
  DialogHeader, DialogTitle, Input,
} from "@wms/ui";

import type { AlertDefinition, AlertDefinitionDraft } from "@/features/alert-engine/alert-definition-queries";
import type { AlertEscalationRule } from "@/features/alert-engine/alert-runtime-queries";

const roleOptions = [
  ["warehouse_manager", "仓库经理"],
  ["maintenance_operator", "养护员"],
  ["system_admin", "系统管理员"],
  ["owner_contact", "货主联系人"],
] as const;

export interface AlertDefinitionForm {
  alertCode: string;
  name: string;
  eventType: string;
  conditionExpression: string;
  severity: AlertDefinitionDraft["default_severity"];
  recipientRoles: string[];
  escalationRef: string;
  silenceMinutes: string;
  disableAllowed: boolean;
  messageTemplate: string;
  messageTemplateEn: string;
}

interface Props {
  open: boolean;
  editing: AlertDefinition | null;
  form: AlertDefinitionForm;
  pending: boolean;
  errorMessage?: string;
  escalationRules: AlertEscalationRule[];
  onOpenChange: (open: boolean) => void;
  onFormChange: (form: AlertDefinitionForm) => void;
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
}

export function AlertDefinitionFormDialog(props: Props) {
  const { open, editing, form, pending, errorMessage, escalationRules, onOpenChange, onFormChange, onSubmit } = props;
  const set = (key: keyof AlertDefinitionForm, value: string | boolean | string[]) => onFormChange({ ...form, [key]: value });
  const toggleRole = (role: string, checked: boolean) => set("recipientRoles", checked ? [...form.recipientRoles, role] : form.recipientRoles.filter((item) => item !== role));

  return <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}><DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl"><form className="grid gap-4" onSubmit={onSubmit}>
    <DialogHeader><DialogTitle>{editing ? "编辑告警定义" : "新增告警定义"}</DialogTitle><DialogDescription>保存只提交质量联系单，审批通过后才会变更生效；GSP 强制告警不能停用或删除。</DialogDescription></DialogHeader>
    <div className="grid gap-3 sm:grid-cols-2"><label className="grid gap-1 text-sm">告警编码<Input required minLength={2} maxLength={64} pattern="[A-Za-z0-9][A-Za-z0-9_.-]{1,63}" readOnly={Boolean(editing)} value={form.alertCode} onChange={(event) => set("alertCode", event.target.value)} /></label><label className="grid gap-1 text-sm">名称<Input required maxLength={128} value={form.name} onChange={(event) => set("name", event.target.value)} /></label></div>
    <div className="grid gap-3 sm:grid-cols-2"><label className="grid gap-1 text-sm">事件类型<Input required minLength={2} maxLength={128} pattern="[A-Za-z0-9][A-Za-z0-9_.-]{1,127}" value={form.eventType} onChange={(event) => set("eventType", event.target.value)} /></label><label className="grid gap-1 text-sm">默认级别<select aria-label="默认级别" className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={form.severity} onChange={(event) => set("severity", event.target.value)}><option value="info">提示</option><option value="warning">警告</option><option value="critical">严重</option></select></label></div>
    <label className="grid gap-1 text-sm">触发条件（可选 JSON）<textarea rows={4} className="rounded-md border border-input bg-background px-3 py-2 font-mono text-sm" value={form.conditionExpression} onChange={(event) => set("conditionExpression", event.target.value)} /></label>
    <fieldset className="grid gap-2"><legend className="text-sm">接收角色</legend><div className="grid gap-2 sm:grid-cols-2">{roleOptions.map(([value, label]) => <label key={value} className="flex min-h-10 items-center gap-2 text-sm"><input type="checkbox" checked={form.recipientRoles.includes(value)} onChange={(event) => toggleRole(value, event.target.checked)} />{label}</label>)}</div></fieldset>
    <div className="grid gap-3 sm:grid-cols-2"><label className="grid gap-1 text-sm">升级策略引用<select aria-label="升级策略引用" className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={form.escalationRef} onChange={(event) => set("escalationRef", event.target.value)}><option value="">不升级</option>{escalationRules.map((rule) => <option key={rule.id} value={rule.rule_code}>{rule.rule_name}（{rule.rule_code}）</option>)}{form.escalationRef && !escalationRules.some((rule) => rule.rule_code === form.escalationRef) && <option value={form.escalationRef}>{form.escalationRef}（不可用）</option>}</select></label><label className="grid gap-1 text-sm">静默期（分钟）<Input required type="number" min="0" max="525600" step="1" value={form.silenceMinutes} onChange={(event) => set("silenceMinutes", event.target.value)} /></label></div>
    <label className="grid gap-1 text-sm">中文消息模板<textarea required rows={3} maxLength={2000} className="rounded-md border border-input bg-background px-3 py-2 text-sm" value={form.messageTemplate} onChange={(event) => set("messageTemplate", event.target.value)} /></label>
    <label className="grid gap-1 text-sm">英文消息模板（可选）<textarea rows={3} maxLength={2000} className="rounded-md border border-input bg-background px-3 py-2 text-sm" value={form.messageTemplateEn} onChange={(event) => set("messageTemplateEn", event.target.value)} /></label>
    <label className="flex min-h-10 items-center gap-2 text-sm"><input type="checkbox" checked={form.disableAllowed} onChange={(event) => set("disableAllowed", event.target.checked)} />允许审批停用</label>
    {errorMessage && <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">{errorMessage}</div>}
    <DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose><Button type="submit" disabled={pending}>{pending ? "提交中..." : "提交审批"}</Button></DialogFooter>
  </form></DialogContent></Dialog>;
}

export function emptyAlertDefinitionForm(): AlertDefinitionForm {
  return { alertCode: "", name: "", eventType: "", conditionExpression: "{}", severity: "warning", recipientRoles: ["warehouse_manager"], escalationRef: "", silenceMinutes: "5", disableAllowed: true, messageTemplate: "", messageTemplateEn: "" };
}

export function alertDefinitionFormFor(row: AlertDefinition): AlertDefinitionForm {
  return { alertCode: row.alert_code, name: row.name, eventType: row.event_type, conditionExpression: row.condition_expression, severity: row.default_severity, recipientRoles: row.recipient_roles, escalationRef: row.escalation_ref ?? "", silenceMinutes: String(row.silence_period_seconds / 60), disableAllowed: row.is_disable_allowed, messageTemplate: row.message_template, messageTemplateEn: row.message_templates["en-US"] ?? "" };
}

export function validateAlertDefinitionForm(form: AlertDefinitionForm): string | null {
  if (!/^[A-Za-z0-9][A-Za-z0-9_.-]{1,63}$/.test(form.alertCode.trim())) return "告警编码必须为 2 到 64 位受控标识符";
  if (!/^[A-Za-z0-9][A-Za-z0-9_.-]{1,127}$/.test(form.eventType.trim())) return "事件类型必须为 2 到 128 位受控标识符";
  if (!form.name.trim() || !form.messageTemplate.trim()) return "名称和消息模板不能为空";
  try { if (form.conditionExpression.trim()) JSON.parse(form.conditionExpression); } catch { return "触发条件必须是合法 JSON"; }
  if (form.recipientRoles.length === 0) return "至少选择一个接收角色";
  const silenceMinutes = Number(form.silenceMinutes);
  if (!Number.isInteger(silenceMinutes) || silenceMinutes < 0 || silenceMinutes > 525600) return "静默期必须是 0 到 525600 分钟的整数";
  return null;
}

export function toAlertDefinitionDraft(form: AlertDefinitionForm): AlertDefinitionDraft {
  const zh = form.messageTemplate.trim();
  const en = form.messageTemplateEn.trim();
  return { alert_code: form.alertCode.trim(), name: form.name.trim(), event_type: form.eventType.trim(), condition_expression: form.conditionExpression.trim(), default_severity: form.severity, recipient_roles: form.recipientRoles, escalation_ref: form.escalationRef.trim() || null, silence_period_seconds: Number(form.silenceMinutes) * 60, is_disable_allowed: form.disableAllowed, message_template: zh, message_templates: { "zh-CN": zh, ...(en ? { "en-US": en } : {}) } };
}
