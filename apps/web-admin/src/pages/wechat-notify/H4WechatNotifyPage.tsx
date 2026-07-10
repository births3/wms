import * as React from "react";
import {
  DataGrid,
  PageHeader,
  QueryPanel,
  StatusBadge,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type QueryPanelField,
  type QueryPanelRangeValue,
  type QueryPanelValue,
  type StatusKey,
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
import { settingsColumns } from "./H4WechatSettingsColumns";

export type H4WechatNotifyMode = "settings" | "configs" | "records";

interface H4WechatNotifyPageProps {
  mode: H4WechatNotifyMode;
}

const h4NotificationConfigQueryFields: QueryPanelField[] = [
  { key: "eventType", label: "事件类型", type: "text", placeholder: "例如 asn_arrived" },
  {
    key: "enabled",
    label: "启停状态",
    type: "multiSelect",
    options: [{ label: "启用", value: "true" }, { label: "停用", value: "false" }],
  },
];
const h4NotificationConfigCoreQueryFieldKeys = ["eventType", "enabled"];

const h4NotificationRecordQueryFields: QueryPanelField[] = [
  { key: "eventType", label: "事件类型", type: "text", placeholder: "例如 asn_arrived" },
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
  { key: "createdAt", label: "创建时间", type: "dateRange" },
];
const h4NotificationRecordCoreQueryFieldKeys = ["eventType", "recipient", "status"];

const configColumns: DataGridColumn<H4NotificationConfig>[] = [
  {
    key: "event_type",
    header: "事件类型",
    width: 220,
    minWidth: 160,
    mono: true,
    sortable: true,
    sortValue: (row) => row.event_type,
    filterValue: (row) => row.event_type,
    copyValue: (row) => row.event_type,
    filter: { type: "text" },
  },
  {
    key: "enabled",
    header: "状态",
    width: 120,
    minWidth: 100,
    sortable: true,
    sortValue: (row) => row.enabled ? 1 : 0,
    filterValue: (row) => row.enabled ? "true" : "false",
    copyValue: (row) => row.enabled ? "启用" : "停用",
    filter: { type: "multiSelect", options: [{ label: "启用", value: "true" }, { label: "停用", value: "false" }] },
    render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.enabled ? "启用" : "停用"} size="sm" />,
  },
  {
    key: "channels",
    header: "通知方式",
    width: 160,
    minWidth: 120,
    copyValue: (row) => row.channels.join("、"),
    render: (row) => row.channels.join("、"),
  },
  {
    key: "recipient_rule",
    header: "接收规则",
    width: 260,
    minWidth: 180,
    filterValue: (row) => recipientRuleText(row.recipient_rule),
    copyValue: (row) => recipientRuleText(row.recipient_rule),
    filter: { type: "text" },
    render: (row) => <span className="line-clamp-2">{recipientRuleText(row.recipient_rule)}</span>,
  },
  {
    key: "template",
    header: "通知模板",
    width: 360,
    minWidth: 220,
    filterValue: (row) => row.template,
    copyValue: (row) => row.template,
    filter: { type: "text" },
    render: (row) => <span className="line-clamp-2">{row.template}</span>,
  },
  {
    key: "version",
    header: "版本",
    width: 100,
    minWidth: 80,
    sortable: true,
    sortValue: (row) => row.version,
    render: (row) => `v${row.version}`,
  },
  {
    key: "updated_at",
    header: "更新时间",
    width: 190,
    minWidth: 160,
    sortable: true,
    sortValue: (row) => row.updated_at,
    filterValue: (row) => row.updated_at,
    copyValue: (row) => formatDateTime(row.updated_at),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.updated_at),
  },
];

const recordColumns: DataGridColumn<H4NotificationRecord>[] = [
  {
    key: "event_type",
    header: "事件类型",
    width: 220,
    minWidth: 160,
    mono: true,
    sortable: true,
    sortValue: (row) => row.event_type,
    filterValue: (row) => row.event_type,
    copyValue: (row) => row.event_type,
    filter: { type: "text" },
  },
  {
    key: "created_at",
    header: "时间",
    width: 190,
    minWidth: 160,
    sortable: true,
    sortValue: (row) => row.created_at,
    filterValue: (row) => row.created_at,
    copyValue: (row) => formatDateTime(row.created_at),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.created_at),
  },
  {
    key: "recipient",
    header: "接收人",
    width: 180,
    minWidth: 140,
    filterValue: (row) => row.recipient,
    copyValue: (row) => row.recipient,
    filter: { type: "text" },
  },
  {
    key: "content_summary",
    header: "内容摘要",
    width: 360,
    minWidth: 220,
    filterValue: (row) => row.content_summary,
    copyValue: (row) => row.content_summary,
    filter: { type: "text" },
    render: (row) => <span className="line-clamp-2">{row.content_summary}</span>,
  },
  {
    key: "status",
    header: "状态",
    width: 130,
    minWidth: 110,
    sortable: true,
    sortValue: (row) => row.status,
    filterValue: (row) => row.status,
    copyValue: (row) => statusLabel(row.status),
    filter: { type: "multiSelect", options: [{ label: "成功", value: "success" }, { label: "失败", value: "failed" }, { label: "重试中", value: "retrying" }] },
    render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" />,
  },
  {
    key: "retry_count",
    header: "重试次数",
    width: 120,
    minWidth: 100,
    sortable: true,
    sortValue: (row) => row.retry_count,
    filterValue: (row) => row.retry_count,
    filter: { type: "numberRange" },
  },
  {
    key: "failure_reason",
    header: "失败原因",
    width: 240,
    minWidth: 160,
    filterValue: (row) => row.failure_reason ?? "",
    copyValue: (row) => row.failure_reason ?? "",
    filter: { type: "text" },
    render: (row) => row.failure_reason || "-",
  },
  {
    key: "dedupe_key",
    header: "去重键",
    width: 220,
    minWidth: 160,
    mono: true,
    filterValue: (row) => row.dedupe_key,
    copyValue: (row) => row.dedupe_key,
    filter: { type: "text" },
  },
];

export function H4WechatNotifyPage({ mode }: H4WechatNotifyPageProps) {
  const [configQuery, setConfigQuery] = React.useState<QueryPanelValue>(() => defaultConfigQuery());
  const [appliedConfigQuery, setAppliedConfigQuery] = React.useState<QueryPanelValue>(() => defaultConfigQuery());
  const [recordQuery, setRecordQuery] = React.useState<QueryPanelValue>(() => defaultRecordQuery());
  const [appliedRecordQuery, setAppliedRecordQuery] = React.useState<QueryPanelValue>(() => defaultRecordQuery());
  const [selectedConfigKeys, setSelectedConfigKeys] = React.useState<string[]>([]);
  const [selectedRecordKeys, setSelectedRecordKeys] = React.useState<string[]>([]);
  const [selectedSettingsKeys, setSelectedSettingsKeys] = React.useState<string[]>([]);
  const [settingsDialogOpen, setSettingsDialogOpen] = React.useState(false);
  const [configDialogOpen, setConfigDialogOpen] = React.useState(false);
  const [sendDialogOpen, setSendDialogOpen] = React.useState(false);
  const [detailRecord, setDetailRecord] = React.useState<H4NotificationRecord | null>(null);
  const [settingsForm, setSettingsForm] = React.useState<SettingsFormState>(() => emptySettingsForm());
  const [configForm, setConfigForm] = React.useState<ConfigFormState>(() => emptyConfigForm());
  const [sendForm, setSendForm] = React.useState<SendFormState>(() => emptySendForm());
  const [notice, setNotice] = React.useState<Notice>(null);

  const configParams = normalizeConfigQuery(appliedConfigQuery);
  const recordParams = normalizeRecordQuery(appliedRecordQuery);
  const settingsQuery = useH4WechatSettingsQuery();
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

  const settingsRows = React.useMemo(() => settingsQuery.data ? [settingsQuery.data] : [], [settingsQuery.data]);
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
  const settingsById = React.useMemo(() => new Map(settingsRows.map((row) => [row.id, row])), [settingsRows]);

  const selectedSettings = selectedSettingsKeys.length === 1 ? settingsById.get(selectedSettingsKeys[0]) ?? null : null;
  const selectedConfig = selectedConfigKeys.length === 1 ? configById.get(selectedConfigKeys[0]) ?? null : null;
  const selectedRecord = selectedRecordKeys.length === 1 ? recordById.get(selectedRecordKeys[0]) ?? null : null;

  if (mode === "settings") {
    return (
      <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
        <PageHeader title="H4 参数设置" subtitle="企业微信应用、回调、密钥别名和重试参数" />
        <NoticePanel notice={settingsDialogOpen ? null : notice} />
        {settingsQuery.error && <ErrorPanel message={settingsQuery.error.message} />}
        <DataGrid
          storageKey="h4.wechat-notify.settings"
          columns={settingsColumns}
          data={settingsRows}
          rowKey={(row) => row.id}
          selectable
          selectedRowKeys={selectedSettingsKeys}
          onSelectedRowKeysChange={setSelectedSettingsKeys}
          caption={settingsQuery.isPending ? "加载企业微信参数..." : undefined}
          emptyTitle="暂无企业微信参数设置"
          exportFileBaseName="H4 参数设置"
          tableClassName="min-w-[2180px]"
          refreshAction={{
            label: "刷新",
            description: "刷新企业微信参数",
            disabled: settingsQuery.isFetching,
            onClick: () => void refreshSettings(),
          }}
          createAction={{
            label: "新增",
            description: "首次新增企业微信参数",
            disabled: () => Boolean(settingsQuery.data),
            onClick: () => openSettingsDialog(null),
          }}
          editAction={{
            label: "修改",
            description: "修改选中的企业微信参数",
            disabled: () => selectedSettingsKeys.length !== 1,
            onClick: () => openSettingsDialog(selectedSettings),
          }}
        />
        <SettingsDialog
          open={settingsDialogOpen}
          form={settingsForm}
          notice={notice}
          saving={upsertSettingsMutation.isPending}
          testing={testSettingsMutation.isPending}
          onFormChange={setSettingsForm}
          onOpenChange={setSettingsDialogOpen}
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
          onQuery={() => setAppliedRecordQuery(normalizeRecordQuery(recordQuery))}
          onReset={() => {
            const next = defaultRecordQuery();
            setRecordQuery(next);
            setAppliedRecordQuery(next);
          }}
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
            label: "刷新",
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
          onApplyQueryState={(queryState) => {
            const next = normalizeRecordQuery(queryValueFromUnknown(queryState));
            setRecordQuery(next);
            setAppliedRecordQuery(next);
          }}
          onClearQueryState={() => {
            const next = defaultRecordQuery();
            setRecordQuery(next);
            setAppliedRecordQuery(next);
          }}
        />
        <RecordDetailDialog record={detailRecord} onOpenChange={(open) => !open && setDetailRecord(null)} />
      </section>
    );
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader title="H4 通知配置" subtitle="配置企业微信事件、模板、接收人和启停状态" />
      <NoticePanel notice={notice} />
      <QueryPanel
        fields={h4NotificationConfigQueryFields}
        defaultVisibleFieldKeys={h4NotificationConfigCoreQueryFieldKeys}
        value={configQuery}
        onValueChange={(next) => setConfigQuery(normalizeConfigQuery(next))}
        onQuery={() => setAppliedConfigQuery(normalizeConfigQuery(configQuery))}
        onReset={() => {
          const next = defaultConfigQuery();
          setConfigQuery(next);
          setAppliedConfigQuery(next);
        }}
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
          label: "刷新",
          description: "刷新通知配置",
          disabled: configsQuery.isFetching,
          onClick: () => void refreshConfigs(),
        }}
        createAction={{
          label: "新增",
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
        onApplyQueryState={(queryState) => {
          const next = normalizeConfigQuery(queryValueFromUnknown(queryState));
          setConfigQuery(next);
          setAppliedConfigQuery(next);
        }}
        onClearQueryState={() => {
          const next = defaultConfigQuery();
          setConfigQuery(next);
          setAppliedConfigQuery(next);
        }}
      />
      <ConfigDialog
        open={configDialogOpen}
        form={configForm}
        saving={upsertMutation.isPending}
        onFormChange={setConfigForm}
        onOpenChange={setConfigDialogOpen}
        onSave={saveConfig}
      />
      <SendDialog
        open={sendDialogOpen}
        form={sendForm}
        sending={sendMutation.isPending}
        onFormChange={setSendForm}
        onOpenChange={setSendDialogOpen}
        onSend={sendNotification}
      />
    </section>
  );

  async function refreshSettings() {
    const result = await settingsQuery.refetch();
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

  function openSettingsDialog(settings: H4WechatSettings | null) {
    setSettingsForm(settings ? formFromSettings(settings) : emptySettingsForm());
    setNotice(null);
    setSettingsDialogOpen(true);
  }

  function openConfigDialog(config: H4NotificationConfig | null) {
    setConfigForm(config ? formFromConfig(config) : emptyConfigForm());
    setConfigDialogOpen(true);
  }

  function openSendDialog(config: H4NotificationConfig) {
    setSendForm({
      eventType: config.event_type,
      recipientsText: usersFromRule(config.recipient_rule).join(", "),
      dedupeKey: `web-test-${Date.now()}`,
      payloadText: JSON.stringify({ asn_no: "ASN-DEMO", recall_id: "RC-DEMO", message: "企业微信通知测试" }, null, 2),
    });
    setSendDialogOpen(true);
  }

  async function saveSettings() {
    try {
      const saved = await upsertSettingsMutation.mutateAsync(settingsRequest(settingsForm));
      setSettingsDialogOpen(false);
      setSelectedSettingsKeys([saved.id]);
      setNotice({ type: "success", text: `${saved.corp_id} 已保存` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "保存企业微信参数失败") });
    }
  }

  async function testSettings() {
    let saved: H4WechatSettings;
    try {
      saved = await upsertSettingsMutation.mutateAsync(settingsRequest(settingsForm));
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "保存企业微信参数失败") });
      return;
    }

    setSelectedSettingsKeys([saved.id]);
    try {
      const result = await testSettingsMutation.mutateAsync();
      setSettingsDialogOpen(false);
      setNotice({ type: result.status === "success" ? "success" : "warning", text: result.message });
    } catch (errorValue) {
      setSettingsDialogOpen(false);
      setNotice({ type: "error", text: errorText(errorValue, "测试企业微信参数失败") });
    }
  }

  async function saveConfig() {
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
      setConfigDialogOpen(false);
      setNotice({ type: "success", text: `${saved.event_type} 已保存` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "保存通知配置失败") });
    }
  }

  async function sendNotification() {
    try {
      const sent = await sendMutation.mutateAsync({
        event_type: sendForm.eventType.trim(),
        recipients: splitTokens(sendForm.recipientsText),
        dedupe_key: sendForm.dedupeKey.trim(),
        payload: JSON.parse(sendForm.payloadText) as Record<string, unknown>,
      });
      setSendDialogOpen(false);
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

function defaultConfigQuery(): QueryPanelValue {
  return { eventType: "", enabled: [] };
}

function defaultRecordQuery(): QueryPanelValue {
  return { eventType: "", recipient: "", status: [], createdAt: { from: "", to: "" } };
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

function recipientRuleText(rule: Record<string, unknown>) {
  const users = stringArray(rule.users);
  const roles = stringArray(rule.roles);
  return [`用户 ${users.join("、") || "-"}`, `角色 ${roles.join("、") || "-"}`].join("；");
}

function usersFromRule(rule: Record<string, unknown>) {
  return stringArray(rule.users);
}

function stringArray(value: unknown) {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string" && item.trim().length > 0) : [];
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

function statusLabel(status: string) {
  if (status === "success") return "成功";
  if (status === "failed") return "失败";
  if (status === "retrying") return "重试中";
  return status;
}

function statusKey(status: string): StatusKey {
  if (status === "success") return "completed";
  if (status === "failed") return "unqualified";
  return "pending";
}

function queryString(value: QueryPanelValue[string]) {
  return typeof value === "string" ? value : "";
}

function queryStringArray(value: QueryPanelValue[string]) {
  return Array.isArray(value) ? value.filter((item) => typeof item === "string") : [];
}

function queryRange(value: QueryPanelValue[string]): QueryPanelRangeValue {
  if (!value || typeof value !== "object" || Array.isArray(value)) return { from: "", to: "" };
  return { from: typeof value.from === "string" ? value.from : "", to: typeof value.to === "string" ? value.to : "" };
}

function queryValueFromUnknown(value: unknown): QueryPanelValue {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as QueryPanelValue) : {};
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

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value || "-";
  return date.toLocaleString("zh-CN", { hour12: false });
}

function errorText(errorValue: unknown, fallback: string) {
  if (errorValue instanceof SyntaxError) return "变量 JSON 格式不正确";
  return errorValue instanceof Error ? errorValue.message : fallback;
}
