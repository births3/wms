import * as React from "react";
import {
  DataGrid,
  PageHeader,
  QueryPanel,
  buildQueryPanelSummaryItems,
  type QueryPanelField,
  type QueryPanelRangeValue,
  type QueryPanelValue,
} from "@wms/ui";
import { RotateCw, Send } from "lucide-react";

import {
  useH4NotificationConfigsQuery,
  useH4NotificationRecordsQuery,
  useH4WechatSettingsQuery,
  useResendH4NotificationRecordMutation,
  useSendH4NotificationMutation,
  useTestH4WechatSettingsMutation,
  useUpsertH4NotificationConfigMutation,
  useUpsertH4WechatSettingsMutation,
  type H4NotificationConfig,
  type H4NotificationRecord,
  type H4WechatSettings,
} from "@/features/wechat-notify/wechat-notify-queries";

import {
  ConfigDialog,
  ErrorPanel,
  NoticePanel,
  RecordDetailDialog,
  SendDialog,
  SettingsDialog,
  type ConfigFormState,
  type Notice,
  type SendFormState,
  type SettingsFormState,
} from "./H4WechatNotifyDialogs";
import { configColumns, recordColumns } from "./H4WechatNotifyColumns";
import { settingsColumns } from "./H4WechatSettingsColumns";
import { stringArray } from "./wechat-notify-helpers";
import { errorText as libErrorText } from "@/lib/error-text";
import { queryRange, queryString, queryStringArray, queryValueFromUnknown } from "@/lib/query-value";
import {
  BUTTON_ADD, BUTTON_REFRESH, COLUMN_CREATED_AT, COLUMN_EVENT_TYPE, FIELD_KEYWORD,
  STATUS_DISABLED, STATUS_ENABLED,
} from "@/lib/ui-strings";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";

export type H4WechatNotifyMode = "settings" | "configs" | "records";

interface H4WechatNotifyPageProps {
  mode: H4WechatNotifyMode;
}

const h4WechatSettingsQueryFields: QueryPanelField[] = [
  { key: "keyword", label: FIELD_KEYWORD, type: "text", placeholder: "企业 ID / Agent / 别名" },
  {
    key: "enabled",
    label: "启停状态",
    type: "multiSelect",
    options: [{ label: STATUS_ENABLED, value: "true" }, { label: STATUS_DISABLED, value: "false" }],
  },
];
const h4WechatSettingsCoreQueryFieldKeys = ["keyword", "enabled"];

const h4NotificationConfigQueryFields: QueryPanelField[] = [
  { key: "eventType", label: COLUMN_EVENT_TYPE, type: "text", placeholder: "例如 asn_arrived" },
  {
    key: "enabled",
    label: "启停状态",
    type: "multiSelect",
    options: [{ label: STATUS_ENABLED, value: "true" }, { label: STATUS_DISABLED, value: "false" }],
  },
];
const h4NotificationConfigCoreQueryFieldKeys = ["eventType", "enabled"];

const h4NotificationRecordQueryFields: QueryPanelField[] = [
  { key: "eventType", label: COLUMN_EVENT_TYPE, type: "text", placeholder: "例如 asn_arrived" },
  { key: "recipient", label: "接收人", type: "text", placeholder: "用户 ID / 企业微信账号" },
  {
    key: "status",
    label: "发送状态",
    type: "multiSelect",
    options: [
      { label: "成功", value: "success" },
      { label: "失败", value: "failed" },
      { label: "重试中", value: "retrying" },
    ],
  },
  { key: "createdAt", label: COLUMN_CREATED_AT, type: "dateRange" },
];
const h4NotificationRecordCoreQueryFieldKeys = ["eventType", "recipient", "status"];

export function H4WechatNotifyPage({ mode }: H4WechatNotifyPageProps) {
  const {
    draftQuery: settingsQuery,
    setDraftQuery: setSettingsQueryState,
    appliedQuery: appliedSettingsQuery,
    applyQuery: applySettingsQuery,
    resetQuery: resetSettingsQuery,
  } = usePageQueryState<QueryPanelValue>(defaultSettingsQuery, normalizeSettingsQuery);
  const {
    draftQuery: configQuery,
    setDraftQuery: setConfigQuery,
    appliedQuery: appliedConfigQuery,
    applyQuery: applyConfigQuery,
    resetQuery: resetConfigQuery,
  } = usePageQueryState<QueryPanelValue>(defaultConfigQuery, normalizeConfigQuery);
  const {
    draftQuery: recordQuery,
    setDraftQuery: setRecordQuery,
    appliedQuery: appliedRecordQuery,
    applyQuery: applyRecordQuery,
    resetQuery: resetRecordQuery,
  } = usePageQueryState<QueryPanelValue>(defaultRecordQuery, normalizeRecordQuery);
  const [selectedConfigKeys, setSelectedConfigKeys] = React.useState<string[]>([]);
  const [selectedRecordKeys, setSelectedRecordKeys] = React.useState<string[]>([]);
  const [selectedSettingsKeys, setSelectedSettingsKeys] = React.useState<string[]>([]);
  const settingsDialog = useDialogState<SettingsFormState>();
  const configDialog = useDialogState<ConfigFormState>();
  const sendDialog = useDialogState<SendFormState>();
  const [detailRecord, setDetailRecord] = React.useState<H4NotificationRecord | null>(null);
  const [notice, setNotice] = React.useState<Notice>(null);

  const settingsParams = normalizeSettingsQuery(appliedSettingsQuery);
  const configParams = normalizeConfigQuery(appliedConfigQuery);
  const recordParams = normalizeRecordQuery(appliedRecordQuery);
  const settingsQueryResult = useH4WechatSettingsQuery();
  const configsQuery = useH4NotificationConfigsQuery(queryString(configParams.eventType));
  const recordsQuery = useH4NotificationRecordsQuery({
    eventType: queryString(recordParams.eventType),
    recipient: queryString(recordParams.recipient),
    status: queryStringArray(recordParams.status).length === 1 ? queryStringArray(recordParams.status)[0] : undefined,
    from: queryRange(recordParams.createdAt).from,
    to: queryRange(recordParams.createdAt).to,
  });
  const upsertSettingsMutation = useUpsertH4WechatSettingsMutation();
  const testSettingsMutation = useTestH4WechatSettingsMutation();
  const upsertMutation = useUpsertH4NotificationConfigMutation();
  const sendMutation = useSendH4NotificationMutation();
  const resendMutation = useResendH4NotificationRecordMutation();

  const settingsRows = React.useMemo(
    () => filterSettings(settingsQueryResult.data ? [settingsQueryResult.data] : [], settingsParams),
    [settingsQueryResult.data, settingsParams],
  );
  const configs = React.useMemo(
    () => filterConfigs(configsQuery.data ?? [], configParams),
    [configsQuery.data, configParams],
  );
  const records = React.useMemo(
    () => filterRecords(recordsQuery.data ?? [], recordParams),
    [recordsQuery.data, recordParams],
  );
  const configById = React.useMemo(() => new Map((configsQuery.data ?? []).map((row) => [row.id, row])), [configsQuery.data]);
  const recordById = React.useMemo(() => new Map((recordsQuery.data ?? []).map((row) => [row.id, row])), [recordsQuery.data]);
  const settingsById = React.useMemo(
    () => new Map((settingsQueryResult.data ? [settingsQueryResult.data] : []).map((row) => [row.id, row])),
    [settingsQueryResult.data],
  );

  const selectedSettings = selectedSettingsKeys.length === 1 ? settingsById.get(selectedSettingsKeys[0]) ?? null : null;
  const selectedConfig = selectedConfigKeys.length === 1 ? configById.get(selectedConfigKeys[0]) ?? null : null;
  const selectedRecord = selectedRecordKeys.length === 1 ? recordById.get(selectedRecordKeys[0]) ?? null : null;

  if (mode === "settings") {
    return (
      <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
        <PageHeader title="H4 参数设置" subtitle="企业微信应用、回调、密钥别名和重试参数" />
        <NoticePanel notice={settingsDialog.open ? null : notice} />
        <QueryPanel
          fields={h4WechatSettingsQueryFields}
          defaultVisibleFieldKeys={h4WechatSettingsCoreQueryFieldKeys}
          value={settingsQuery}
          onValueChange={(next) => setSettingsQueryState(normalizeSettingsQuery(next))}
          onQuery={() => applySettingsQuery(settingsQuery)}
          onReset={resetSettingsQuery}
        />
        {settingsQueryResult.error && <ErrorPanel message={settingsQueryResult.error.message} />}
        <DataGrid
          storageKey="h4.wechat-notify.settings"
          columns={settingsColumns}
          data={settingsRows}
          rowKey={(row) => row.id}
          selectable
          selectedRowKeys={selectedSettingsKeys}
          onSelectedRowKeysChange={setSelectedSettingsKeys}
          caption={settingsQueryResult.isPending ? "加载企业微信参数..." : undefined}
          emptyTitle="暂无企业微信参数设置"
          exportFileBaseName="H4 参数设置"
          tableClassName="min-w-[2180px]"
          refreshAction={{
            label: BUTTON_REFRESH,
            description: "刷新企业微信参数",
            disabled: settingsQueryResult.isFetching,
            onClick: () => void refreshSettings(),
          }}
          createAction={{
            label: BUTTON_ADD,
            description: "首次新增企业微信参数",
            disabled: () => Boolean(settingsQueryResult.data),
            onClick: () => openSettingsDialog(null),
          }}
          editAction={{
            label: "修改",
            description: "修改选中的企业微信参数",
            disabled: () => selectedSettingsKeys.length !== 1,
            onClick: () => openSettingsDialog(selectedSettings),
          }}
          queryState={appliedSettingsQuery}
          querySummaryItems={buildQueryPanelSummaryItems(h4WechatSettingsQueryFields, appliedSettingsQuery)}
          onApplyQueryState={applySettingsGridQueryState}
          onClearQueryState={clearSettingsGridQueryState}
        />
        <SettingsDialog
          open={settingsDialog.open}
          form={settingsDialog.target ?? emptySettingsForm()}
          notice={notice}
          saving={upsertSettingsMutation.isPending}
          testing={testSettingsMutation.isPending}
          onFormChange={settingsDialog.setTarget}
          onOpenChange={settingsDialog.setOpen}
          onSave={saveSettings}
          onTest={testSettings}
        />
      </section>
    );
  }

  if (mode === "records") {
    return (
      <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
        <PageHeader title="H4 发送记录" subtitle="企业微信通知发送、失败排查与手动重发" />
        <NoticePanel notice={notice} />
        <QueryPanel
          fields={h4NotificationRecordQueryFields}
          defaultVisibleFieldKeys={h4NotificationRecordCoreQueryFieldKeys}
          value={recordQuery}
          onValueChange={(next) => setRecordQuery(normalizeRecordQuery(next))}
          onQuery={() => applyRecordQuery(recordQuery)}
          onReset={resetRecordQuery}
        />
        {recordsQuery.error && <ErrorPanel message={recordsQuery.error.message} />}
        <DataGrid
          storageKey="h4.wechat-notify.records"
          columns={recordColumns}
          data={records}
          rowKey={(row) => row.id}
          selectable
          selectedRowKeys={selectedRecordKeys}
          onSelectedRowKeysChange={setSelectedRecordKeys}
          caption={recordsQuery.isPending ? "加载发送记录..." : undefined}
          emptyTitle="暂无发送记录"
          exportFileBaseName="H4 发送记录"
          tableClassName="min-w-[1660px]"
          refreshAction={{
            label: BUTTON_REFRESH,
            description: "刷新发送记录",
            disabled: recordsQuery.isFetching,
            onClick: () => void refreshRecords(),
          }}
          detailAction={{
            label: "详情",
            description: "查看通知发送详情",
            onClick: (context) => setDetailRecord(recordById.get(context.selectedRowKeys[0]) ?? null),
          }}
          toolbarActions={[
            {
              key: "resend",
              label: "重发",
              description: "重发失败或重试中的企业微信通知",
              icon: <RotateCw className="size-4" aria-hidden />,
              disabled: () => !selectedRecord || !["failed", "retrying"].includes(selectedRecord.status) || resendMutation.isPending,
              onClick: () => selectedRecord && void resendRecord(selectedRecord.id),
            },
          ]}
          queryState={appliedRecordQuery}
          querySummaryItems={buildQueryPanelSummaryItems(h4NotificationRecordQueryFields, appliedRecordQuery)}
          onApplyQueryState={applyRecordGridQueryState}
          onClearQueryState={clearRecordGridQueryState}
        />
        <RecordDetailDialog record={detailRecord} onOpenChange={(open) => !open && setDetailRecord(null)} />
      </section>
    );
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader title="H4 通知配置" subtitle="配置企业微信事件、模板、接收人和启停状态" />
      <NoticePanel notice={configDialog.open || sendDialog.open ? null : notice} />
      <QueryPanel
        fields={h4NotificationConfigQueryFields}
        defaultVisibleFieldKeys={h4NotificationConfigCoreQueryFieldKeys}
        value={configQuery}
        onValueChange={(next) => setConfigQuery(normalizeConfigQuery(next))}
        onQuery={() => applyConfigQuery(configQuery)}
        onReset={resetConfigQuery}
      />
      {configsQuery.error && <ErrorPanel message={configsQuery.error.message} />}
      <DataGrid
        storageKey="h4.wechat-notify.configs"
        columns={configColumns}
        data={configs}
        rowKey={(row) => row.id}
        selectable
        selectedRowKeys={selectedConfigKeys}
        onSelectedRowKeysChange={setSelectedConfigKeys}
        caption={configsQuery.isPending ? "加载通知配置..." : undefined}
        emptyTitle="暂无通知配置"
        exportFileBaseName="H4 通知配置"
        tableClassName="min-w-[1410px]"
        refreshAction={{
          label: BUTTON_REFRESH,
          description: "刷新通知配置",
          disabled: configsQuery.isFetching,
          onClick: () => void refreshConfigs(),
        }}
        createAction={{
          label: BUTTON_ADD,
          description: "新增企业微信通知配置",
          onClick: () => openConfigDialog(null),
        }}
        editAction={{
          label: "修改",
          description: "修改选中的通知配置",
          disabled: () => selectedConfigKeys.length !== 1,
          onClick: () => openConfigDialog(selectedConfig),
        }}
        toolbarActions={[
          {
            key: "test-send",
            label: "试发",
            description: "按选中配置发送一次测试通知",
            icon: <Send className="size-4" aria-hidden />,
            disabled: () => !selectedConfig || !selectedConfig.enabled,
            onClick: () => selectedConfig && openSendDialog(selectedConfig),
          },
        ]}
        queryState={appliedConfigQuery}
        querySummaryItems={buildQueryPanelSummaryItems(h4NotificationConfigQueryFields, appliedConfigQuery)}
        onApplyQueryState={applyConfigGridQueryState}
        onClearQueryState={clearConfigGridQueryState}
      />
      <ConfigDialog
        open={configDialog.open}
        form={configDialog.target ?? emptyConfigForm()}
        notice={notice}
        saving={upsertMutation.isPending}
        onFormChange={configDialog.setTarget}
        onOpenChange={configDialog.setOpen}
        onSave={saveConfig}
      />
      <SendDialog
        open={sendDialog.open}
        form={sendDialog.target ?? emptySendForm()}
        notice={notice}
        sending={sendMutation.isPending}
        onFormChange={sendDialog.setTarget}
        onOpenChange={sendDialog.setOpen}
        onSend={sendNotification}
      />
    </section>
  );

  async function refreshSettings() {
    const result = await settingsQueryResult.refetch();
    setNotice(result.error ? { type: "error", text: result.error.message } : { type: "success", text: "企业微信参数已刷新" });
  }

  async function refreshConfigs() {
    const result = await configsQuery.refetch();
    setNotice(result.error ? { type: "error", text: result.error.message } : { type: "success", text: "通知配置已刷新" });
  }

  async function refreshRecords() {
    const result = await recordsQuery.refetch();
    setNotice(result.error ? { type: "error", text: result.error.message } : { type: "success", text: "发送记录已刷新" });
  }

  function applySettingsGridQueryState(queryState: unknown) {
    applySettingsQuery(queryValueFromUnknown(queryState));
  }

  function clearSettingsGridQueryState() {
    resetSettingsQuery();
  }

  function applyConfigGridQueryState(queryState: unknown) {
    applyConfigQuery(queryValueFromUnknown(queryState));
  }

  function clearConfigGridQueryState() {
    resetConfigQuery();
  }

  function applyRecordGridQueryState(queryState: unknown) {
    applyRecordQuery(queryValueFromUnknown(queryState));
  }

  function clearRecordGridQueryState() {
    resetRecordQuery();
  }

  function openSettingsDialog(settings: H4WechatSettings | null) {
    setNotice(null);
    settingsDialog.openWith(settings ? formFromSettings(settings) : emptySettingsForm());
  }

  function openConfigDialog(config: H4NotificationConfig | null) {
    setNotice(null);
    configDialog.openWith(config ? formFromConfig(config) : emptyConfigForm());
  }

  function openSendDialog(config: H4NotificationConfig) {
    setNotice(null);
    sendDialog.openWith({
      eventType: config.event_type,
      recipientsText: usersFromRule(config.recipient_rule).join(", "),
      dedupeKey: `web-test-${Date.now()}`,
      payloadText: JSON.stringify({ asn_no: "ASN-DEMO", recall_id: "RC-DEMO", message: "企业微信通知测试" }, null, 2),
    });
  }

  async function saveSettings() {
    if (!settingsDialog.target) return;
    try {
      const saved = await upsertSettingsMutation.mutateAsync(settingsRequest(settingsDialog.target));
      settingsDialog.close();
      setSelectedSettingsKeys([saved.id]);
      setNotice({ type: "success", text: `${saved.corp_id} 已保存` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "保存企业微信参数失败") });
    }
  }

  async function testSettings() {
    if (!settingsDialog.target) return;
    let saved: H4WechatSettings;
    try {
      saved = await upsertSettingsMutation.mutateAsync(settingsRequest(settingsDialog.target));
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "保存企业微信参数失败") });
      return;
    }

    setSelectedSettingsKeys([saved.id]);
    try {
      const result = await testSettingsMutation.mutateAsync();
      settingsDialog.close();
      setNotice({ type: result.status === "success" ? "success" : "warning", text: result.message });
    } catch (errorValue) {
      settingsDialog.close();
      setNotice({ type: "error", text: errorText(errorValue, "测试企业微信参数失败") });
    }
  }

  async function saveConfig() {
    const configForm = configDialog.target;
    if (!configForm) return;
    try {
      const request = {
        event_type: configForm.eventType.trim(),
        enabled: configForm.enabled,
        channels: ensureWechat(splitTokens(configForm.channelsText)),
        template: configForm.template.trim(),
        recipient_rule: {
          users: splitTokens(configForm.usersText),
          roles: splitTokens(configForm.rolesText),
        },
      };
      const saved = await upsertMutation.mutateAsync(request);
      configDialog.close();
      setNotice({ type: "success", text: `${saved.event_type} 已保存` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "保存通知配置失败") });
    }
  }

  async function sendNotification() {
    const sendForm = sendDialog.target;
    if (!sendForm) return;
    try {
      const sent = await sendMutation.mutateAsync({
        event_type: sendForm.eventType.trim(),
        recipients: splitTokens(sendForm.recipientsText),
        dedupe_key: sendForm.dedupeKey.trim(),
        payload: JSON.parse(sendForm.payloadText) as Record<string, unknown>,
      });
      sendDialog.close();
      setNotice({ type: "success", text: `已生成 ${sent.length} 条发送记录` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "发送企业微信通知失败") });
    }
  }

  async function resendRecord(recordId: string) {
    if (!window.confirm("确认重发这条企业微信通知？")) return;
    try {
      const resent = await resendMutation.mutateAsync(recordId);
      if (resent.status === "success") {
        setNotice({ type: "success", text: `${resent.recipient} 已重发` });
      } else {
        setNotice({
          type: "warning",
          text: `重发失败：${resent.failure_reason || "企业微信未返回成功状态"}`,
        });
      }
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "重发企业微信通知失败") });
    }
  }
}

function defaultSettingsQuery(): QueryPanelValue {
  return { keyword: "", enabled: [] };
}

function defaultConfigQuery(): QueryPanelValue {
  return { eventType: "", enabled: [] };
}

function defaultRecordQuery(): QueryPanelValue {
  return { eventType: "", recipient: "", status: [], createdAt: { from: "", to: "" } };
}

function normalizeSettingsQuery(value: QueryPanelValue): QueryPanelValue {
  return { keyword: queryString(value.keyword), enabled: queryStringArray(value.enabled) };
}

function normalizeConfigQuery(value: QueryPanelValue): QueryPanelValue {
  return { eventType: queryString(value.eventType), enabled: queryStringArray(value.enabled) };
}

function normalizeRecordQuery(value: QueryPanelValue): QueryPanelValue {
  return {
    eventType: queryString(value.eventType),
    recipient: queryString(value.recipient),
    status: queryStringArray(value.status),
    createdAt: queryRange(value.createdAt),
  };
}

function filterSettings(rows: H4WechatSettings[], query: QueryPanelValue) {
  const keyword = queryString(query.keyword).trim().toLowerCase();
  const enabled = new Set(queryStringArray(query.enabled));
  return rows.filter((row) => {
    const enabledValue = row.enabled ? "true" : "false";
    const haystack = [
      row.corp_id,
      row.agent_id,
      row.secret_alias,
      row.callback_token_alias,
      row.aes_key_alias,
    ].join(" ").toLowerCase();
    return (!keyword || haystack.includes(keyword)) && (!enabled.size || enabled.has(enabledValue));
  });
}

function filterConfigs(rows: H4NotificationConfig[], query: QueryPanelValue) {
  const eventType = queryString(query.eventType).trim().toLowerCase();
  const enabled = new Set(queryStringArray(query.enabled));
  return rows.filter((row) => {
    const enabledValue = row.enabled ? "true" : "false";
    return (!eventType || row.event_type.toLowerCase().includes(eventType)) && (!enabled.size || enabled.has(enabledValue));
  });
}

function filterRecords(rows: H4NotificationRecord[], query: QueryPanelValue) {
  const eventType = queryString(query.eventType).trim().toLowerCase();
  const recipient = queryString(query.recipient).trim().toLowerCase();
  const statuses = new Set(queryStringArray(query.status));
  const createdAt = queryRange(query.createdAt);
  return rows.filter((row) => (
    (!eventType || row.event_type.toLowerCase().includes(eventType)) &&
    (!recipient || row.recipient.toLowerCase().includes(recipient)) &&
    (!statuses.size || statuses.has(row.status)) &&
    dateInRange(row.created_at, createdAt)
  ));
}

function emptyConfigForm(): ConfigFormState {
  return { eventType: "", enabled: true, channelsText: "wechat", usersText: "", rolesText: "", template: "{{message}}" };
}

function emptySettingsForm(): SettingsFormState {
  return {
    corpId: "",
    agentId: "",
    secretAlias: "",
    callbackTokenAlias: "",
    aesKeyAlias: "",
    callbackUrl: "",
    approvalCallbackPath: "/api/v1/wechat-notify/approvals/{approval_id}/callback",
    enabled: true,
    retryMaxAttempts: "3",
    retryIntervalSeconds: "60",
  };
}

function settingsRequest(form: SettingsFormState) {
  return {
    corp_id: form.corpId.trim(),
    agent_id: form.agentId.trim(),
    secret_alias: form.secretAlias.trim(),
    callback_token_alias: form.callbackTokenAlias.trim(),
    aes_key_alias: form.aesKeyAlias.trim(),
    callback_url: form.callbackUrl.trim(),
    approval_callback_path: form.approvalCallbackPath.trim(),
    enabled: form.enabled,
    retry_max_attempts: intFromText(form.retryMaxAttempts, 3),
    retry_interval_seconds: intFromText(form.retryIntervalSeconds, 60),
  };
}

function emptySendForm(): SendFormState {
  return { eventType: "", recipientsText: "", dedupeKey: "", payloadText: "{}" };
}

function formFromSettings(settings: H4WechatSettings): SettingsFormState {
  return {
    corpId: settings.corp_id,
    agentId: settings.agent_id,
    secretAlias: settings.secret_alias,
    callbackTokenAlias: settings.callback_token_alias,
    aesKeyAlias: settings.aes_key_alias,
    callbackUrl: settings.callback_url,
    approvalCallbackPath: settings.approval_callback_path,
    enabled: settings.enabled,
    retryMaxAttempts: String(settings.retry_max_attempts),
    retryIntervalSeconds: String(settings.retry_interval_seconds),
  };
}

function formFromConfig(config: H4NotificationConfig): ConfigFormState {
  return {
    eventType: config.event_type,
    enabled: config.enabled,
    channelsText: config.channels.join(", "),
    usersText: usersFromRule(config.recipient_rule).join(", "),
    rolesText: stringArray(config.recipient_rule.roles).join(", "),
    template: config.template,
  };
}

function usersFromRule(rule: Record<string, unknown>) {
  return stringArray(rule.users);
}

function splitTokens(value: string) {
  return value.split(/[,，\s]+/).map((item) => item.trim()).filter(Boolean);
}

function ensureWechat(channels: string[]) {
  return channels.includes("wechat") ? channels : ["wechat", ...channels];
}

function intFromText(value: string, fallback: number) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function dateInRange(value: string, range: QueryPanelRangeValue) {
  const date = localDateKey(value);
  return (!range.from || date >= range.from) && (!range.to || date <= range.to);
}

function localDateKey(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value.slice(0, 10);
  const pad = (part: number) => String(part).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function errorText(errorValue: unknown, fallback: string) {
  if (errorValue instanceof SyntaxError) return "变量 JSON 格式不正确";
  return libErrorText(errorValue, fallback);
}
