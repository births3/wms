import type { IncomingMessage, ServerResponse } from "node:http";

const devUserId = "00000000-0000-0000-0000-000000000101";

interface DevAdminMenuButtonPermission {
  action_key: string;
  action_label: string;
  action_kind: string;
  enabled: boolean;
  sort_order: number;
}

interface DevAdminMenuNode {
  id: string;
  parent_id: string | null;
  code: string;
  title: string;
  level: number;
  path: string;
  view_id: string | null;
  icon_key: string;
  permission_key: string;
  sort_order: number;
  enabled: boolean;
  button_permissions: DevAdminMenuButtonPermission[];
  children: DevAdminMenuNode[];
  created_at: string;
  updated_at: string;
}

let devAdminMenuNodes = devAdminMenuSeed();
let devAdminMenuVersionNo = 1;

export async function handleAdminMenuDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
) {
  if (req.method === "GET" && (pathname === "/api/v1/admin/menus/published" || pathname === "/api/v1/admin/menus/draft")) {
    sendJson(res, 200, devAdminMenuResponse());
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/admin/menus/draft/nodes") {
    const body = await readJsonBody(req);
    const parentId = asNullableString(body.parent_id);
    const parent = parentId ? findDevAdminMenuNode(parentId)?.node : null;
    if (!parent || parent.level >= 3) {
      sendJson(res, 400, { code: "DEV_MOCK_INVALID_MENU_PARENT", message: "Invalid menu parent", trace_id: "dev-mock" });
      return;
    }
    const now = new Date().toISOString();
    const code = asString(body.code, `custom.${Date.now()}`);
    const node: DevAdminMenuNode = {
      id: crypto.randomUUID(),
      parent_id: parent.id,
      code,
      title: asString(body.title, "新菜单"),
      level: parent.level + 1,
      path: `${parent.path}/${code}`,
      view_id: asNullableString(body.view_id),
      icon_key: asString(body.icon_key, "ShieldCheck"),
      permission_key: asString(body.permission_key, `menu.${code}`),
      sort_order: asNumber(body.sort_order, parent.children.length * 10 + 10),
      enabled: asBoolean(body.enabled, true),
      button_permissions: devAdminMenuButtonsFromUnknown(body.button_permissions),
      children: [],
      created_at: now,
      updated_at: now,
    };
    parent.children.push(node);
    sortDevAdminMenuTree(devAdminMenuNodes);
    sendJson(res, 200, node);
    return;
  }

  const nodeDetail = pathname.match(/^\/api\/v1\/admin\/menus\/draft\/nodes\/([^/]+)$/);
  if (req.method === "PATCH" && nodeDetail) {
    const found = findDevAdminMenuNode(nodeDetail[1]);
    if (!found) {
      sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Menu node not found", trace_id: "dev-mock" });
      return;
    }
    const body = await readJsonBody(req);
    const node = found.node;
    if (hasOwn(body, "parent_id")) {
      const nextParentId = asNullableString(body.parent_id);
      const nextParent = nextParentId ? findDevAdminMenuNode(nextParentId)?.node : null;
      if (nextParent && nextParent.level < 3 && nextParent.id !== node.id) {
        found.siblings.splice(found.index, 1);
        nextParent.children.push(node);
        updateDevAdminMenuPath(node, nextParent);
      }
    }
    if (typeof body.title === "string") node.title = body.title;
    if (hasOwn(body, "view_id")) node.view_id = asNullableString(body.view_id);
    if (typeof body.icon_key === "string") node.icon_key = body.icon_key;
    if (typeof body.permission_key === "string") node.permission_key = body.permission_key;
    if (typeof body.sort_order === "number") node.sort_order = body.sort_order;
    if (typeof body.enabled === "boolean") node.enabled = body.enabled;
    if (Array.isArray(body.button_permissions)) node.button_permissions = devAdminMenuButtonsFromUnknown(body.button_permissions);
    node.updated_at = new Date().toISOString();
    sortDevAdminMenuTree(devAdminMenuNodes);
    sendJson(res, 200, node);
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/admin/menus/draft/batch-enable") {
    const body = await readJsonBody(req);
    const ids = Array.isArray(body.ids) ? body.ids.filter((item): item is string => typeof item === "string") : [];
    const enabled = asBoolean(body.enabled, false);
    const updated = ids.flatMap((id) => {
      const found = findDevAdminMenuNode(id);
      if (!found) return [];
      found.node.enabled = enabled;
      found.node.updated_at = new Date().toISOString();
      return [found.node];
    });
    sendJson(res, 200, updated);
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/admin/menus/publish") {
    devAdminMenuVersionNo += 1;
    sendJson(res, 200, devAdminMenuVersion());
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/admin/menus/rollback") {
    sendJson(res, 200, devAdminMenuVersion());
    return;
  }

  sendJson(res, 404, {
    code: "DEV_MOCK_NOT_FOUND",
    message: "Admin menu dev mock route not found",
    trace_id: "dev-mock",
  });
}

function devAdminMenuResponse() {
  return {
    data: devAdminMenuNodes,
    page: { count: devAdminMenuNodes.length, next_cursor: null },
    version_no: devAdminMenuVersionNo,
  };
}

function devAdminMenuVersion() {
  return {
    id: crypto.randomUUID(),
    version_no: devAdminMenuVersionNo,
    published_by: devUserId,
    published_at: new Date().toISOString(),
    note: "dev mock",
  };
}

function devAdminMenuSeed(): DevAdminMenuNode[] {
  const now = "2026-06-29T00:00:00.000Z";
  let sequence = 7000;
  const nextId = () => `00000000-0000-0000-0000-${String(sequence++).padStart(12, "0")}`;
  const pages = (
    parent: DevAdminMenuNode,
    values: Array<[string, string, string]>,
  ) => values.map(([viewId, title, iconKey], index) => devAdminMenuNode({
    id: nextId(),
    parent,
    code: viewId,
    title,
    level: 3,
    viewId,
    iconKey,
    sortOrder: (index + 1) * 10,
    now,
  }));
  const group = (
    parent: DevAdminMenuNode,
    code: string,
    title: string,
    iconKey: string,
    sortOrder: number,
    items: Array<[string, string, string]>,
  ) => {
    const node = devAdminMenuNode({ id: nextId(), parent, code, title, level: 2, viewId: null, iconKey, sortOrder, now });
    node.children = pages(node, items);
    return node;
  };
  const section = (code: string, title: string, iconKey: string, sortOrder: number, groups: (parent: DevAdminMenuNode) => DevAdminMenuNode[]) => {
    const node = devAdminMenuNode({ id: nextId(), parent: null, code, title, level: 1, viewId: null, iconKey, sortOrder, now });
    node.children = groups(node);
    return node;
  };

  return [
    section("dashboard", "工作台", "Activity", 10, (parent) => [
      group(parent, "overview", "工作台概览", "Activity", 10, [["dashboard", "运营总览", "Activity"]]),
    ]),
    section("master-data", "基础档案", "BookOpen", 20, (parent) => [
      group(parent, "master", "主数据", "PackageCheck", 10, [
        ["m1-products", "M1 商品档案", "PackageCheck"],
        ["m1-business-partners", "M1 客商档案", "Users"],
      ]),
      group(parent, "warehouse", "仓储资料", "Warehouse", 20, [
        ["m1-warehouses", "M1 仓库管理", "Warehouse"],
        ["m1-zones", "M1 库区管理", "MapPinned"],
        ["m1-locations", "M1 库位管理", "MapPinned"],
        ["dock-management", "M1 月台管理", "MapPinned"],
        ["m1-lpn-containers", "M1 容器管理", "PackageCheck"],
      ]),
      group(parent, "system", "系统配置", "BookOpen", 30, [
        ["m1-system-dictionary", "M1 系统字典", "BookOpen"],
        ["m1-feature-flags", "M1 Feature Flag", "KeyRound"],
      ]),
    ]),
    section("inbound", "入库业务", "CheckCircle2", 30, (parent) => [
      group(parent, "inbound-work", "入库作业", "CheckCircle2", 10, [
        ["m2-receiving", "M2 收货管理", "CheckCircle2"],
        ["m2-inspecting", "M2 验收管理", "ClipboardList"],
        ["m2-putaway", "M2 上架管理", "PackageCheck"],
        ["m2-putaway-strategy", "M2 上架策略", "ClipboardList"],
      ]),
      group(parent, "inbound-documents", "入库资料", "ClipboardList", 20, [
        ["m2-inbound-documents", "入库资料录入", "ClipboardList"],
        ["m-di-review", "药检单审核", "ShieldCheck"],
        ["m-di-platforms", "M-DI 药检平台", "KeyRound"],
        ["m-di-stamp", "药检图章配置", "Stamp"],
      ]),
    ]),
    section("outbound", "出库业务", "ClipboardList", 40, (parent) => [
      group(parent, "outbound-work", "出库作业", "ClipboardList", 10, [
        ["m4-orders", "M4 出库订单管理", "ClipboardList"],
        ["m4-waves", "M4 波次规划", "PackageCheck"],
        ["m4-review", "M4 复核发货", "CheckCircle2"],
        ["m4-returns", "M4 采购退货出库", "ClipboardList"],
      ]),
    ]),
    section("inventory", "库内业务", "Layers", 50, (parent) => [
      group(parent, "inventory-work", "库存管理", "Layers", 10, [
        ["m3-batches", "M3 批号管理", "Layers"],
        ["m3-location-history", "M3 库位历史", "Layers"],
        ["m3-status-config", "M3 状态规则", "ClipboardList"],
        ["m3-replenishment-strategies", "M3 补货策略", "ClipboardList"],
        ["m3-replenishment-tasks", "M3 补货任务", "ClipboardList"],
        ["m3-counts", "M3 库存盘点", "ClipboardList"],
        ["m3-maintenance", "M3 在库养护", "ClipboardList"],
        ["m3-relocations", "M3 库内移库", "Layers"],
        ["mrc-reconciliation", "M-RC 库存对账", "ClipboardList"],
        ["mte-task-dispatch", "M-TE 任务调度", "ClipboardList"],
        ["mte-task-groups", "M-TE 任务组资格", "Users"],
        ["mte-task-types", "M-TE 任务类型配置", "ClipboardList"],
      ]),
    ]),
    section("value-added", "增值业务", "ClipboardList", 55, (parent) => [
      group(parent, "value-added-work", "增值作业", "ClipboardList", 10, [
        ["m9-billing-rules", "M9 计费规则", "ClipboardList"],
        ["m10-route-plans", "M10 路径规划接收", "Truck"],
      ]),
    ]),
    section("platform", "基础能力", "ShieldCheck", 60, (parent) => [
      group(parent, "h1-auth", "H1 权限租户", "ShieldCheck", 10, [
        ["h1-menu-management", "H1 菜单管理", "ShieldCheck"],
        ["h1-role-permission", "H1 角色权限", "ShieldCheck"],
        ["h1-session-management", "H1 登录会话", "ShieldCheck"],
        ["h1-api-keys", "H1 API Key 管理", "KeyRound"],
      ]),
      group(parent, "h2-audit", "H2 审计能力", "ClipboardList", 20, [["h2-audit-trail", "H2 审计追踪", "ClipboardList"]]),
      group(parent, "h3-contract", "H3 契约能力", "KeyRound", 30, [["h3-api-contract", "H3 OpenAPI", "KeyRound"]]),
      group(parent, "h4-wechat", "H4 企业微信", "Bell", 40, [
        ["h4-wechat-settings", "H4 参数设置", "KeyRound"],
        ["h4-notify-configs", "H4 通知配置", "Bell"],
        ["h4-notify-records", "H4 发送记录", "History"],
      ]),
      group(parent, "h5-express", "H5 快递能力", "Truck", 50, [["h5-express", "H5 快递对接", "Truck"]]),
      group(parent, "hal-alert", "H-AL 告警能力", "Bell", 60, [
        ["hal-alert-dashboard", "H-AL 告警看板", "Activity"],
        ["hal-alert-definitions", "H-AL 告警定义", "Bell"],
        ["hal-alert-escalations", "H-AL 升级规则", "History"],
      ]),
      group(parent, "h8-integration", "H8 集成中心", "KeyRound", 55, [
        ["h8-erp-connectors", "H8 ERP 连接", "KeyRound"],
        ["h8-erp-messages", "H8 ERP 消息", "Inbox"],
        ["h8-erp-interface-tables", "H8 接口表探查", "Database"],
      ]),
      group(parent, "h9-print", "H9 打印能力", "Printer", 90, [
        ["h9-delivery-note-aggregation", "作业·随货同行单归集", "Printer"],
        ["h9-print-devices", "设备·Print Agent 管理", "Printer"],
        ["h9-print-templates", "H9 打印模板", "Printer"],
      ]),
      group(parent, "mcg-numbering", "M-CG 编码能力", "KeyRound", 100, [["mcg-numbering", "M-CG 单据号规则", "KeyRound"]]),
    ]),
  ];
}

function devAdminMenuNode({
  id,
  parent,
  code,
  title,
  level,
  viewId,
  iconKey,
  sortOrder,
  now,
}: {
  id: string;
  parent: DevAdminMenuNode | null;
  code: string;
  title: string;
  level: number;
  viewId: string | null;
  iconKey: string;
  sortOrder: number;
  now: string;
}): DevAdminMenuNode {
  const path = parent ? `${parent.path}/${code}` : code;
  return {
    id,
    parent_id: parent?.id ?? null,
    code,
    title,
    level,
    path,
    view_id: viewId,
    icon_key: iconKey,
    permission_key: viewId ? devViewPermissionKey(viewId) : `menu.${code.replace(/-/g, ".")}`,
    sort_order: sortOrder,
    enabled: true,
    button_permissions: viewId ? devStandardAdminMenuButtons() : [],
    children: [],
    created_at: now,
    updated_at: now,
  };
}

function devStandardAdminMenuButtons(): DevAdminMenuButtonPermission[] {
  return ["detail", "create", "update", "disable", "refresh", "export", "print"].map((actionKey, index) => ({
    action_key: actionKey,
    action_label: devActionLabel(actionKey),
    action_kind: "standard",
    enabled: true,
    sort_order: (index + 1) * 10,
  }));
}

function devViewPermissionKey(viewId: string) {
  const permissions: Record<string, string> = {
    dashboard: "h1.auth.me",
    "m1-products": "m1.master_data.read",
    "m1-business-partners": "m1.master_data.read",
    "m1-warehouses": "m1.master_data.read",
    "m1-zones": "m1.master_data.read",
    "m1-locations": "m1.master_data.read",
    "m1-system-dictionary": "m1.system_dictionary.read",
    "m1-feature-flags": "m1.config.write",
    "m2-receiving": "m2.write",
    "m2-inspecting": "m2.write",
    "m2-putaway": "m2.write",
    "m2-putaway-strategy": "m2.putaway.write",
    "m3-batches": "m3.read",
    "m3-location-history": "m3.read",
    "m3-status-config": "m3.read",
    "m3-replenishment-strategies": "m3.replenishment.manage",
    "m3-replenishment-tasks": "m3.replenishment.manage",
    "m3-counts": "m3.read",
    "m3-maintenance": "m3.read",
    "m3-relocations": "m3.read",
    "mrc-reconciliation": "rc.reconciliation.read",
    "mte-task-dispatch": "mte.task.read_all",
    "mte-task-groups": "mte.task_group.write",
    "mte-task-types": "mte.task_type.read",
    "m9-billing-rules": "m9.billing.read",
    "m10-route-plans": "m10.tms.read",
    "m4-orders": "m4.read",
    "m4-waves": "m4.read",
    "m4-review": "m4.read",
    "m4-returns": "m4.read",
    "h1-menu-management": "h1.menu.read",
    "h1-role-permission": "h1.roles.manage",
    "h1-session-management": "h1.sessions.manage",
    "h1-api-keys": "h1.api_keys.manage",
    "h2-audit-trail": "audit.read",
    "h3-api-contract": "h3.contract.read",
    "h4-wechat-settings": "h4.notify.read",
    "h4-notify-configs": "h4.notify.read",
    "h4-notify-records": "h4.notify.read",
    "h5-express": "h5.express.read",
    "hal-alert-dashboard": "hal.alert.read",
    "hal-alert-definitions": "hal.alert-definition.read",
    "hal-alert-escalations": "hal.escalation.read",
    "h8-erp-connectors": "h8.erp_connector.read",
    "h8-erp-messages": "h8.erp_connector.read",
    "h8-erp-interface-tables": "h8.erp_interface_table.read",
    "h9-delivery-note-aggregation": "h9.print_orchestration.read",
    "h9-print-devices": "h9.print_device.read",
    "h9-print-templates": "h9.print_template.read",
    "mcg-numbering": "mcg.document_numbering.read",
  };
  return permissions[viewId] ?? `menu.${viewId.replace(/-/g, ".")}`;
}

function devActionLabel(actionKey: string) {
  const labels: Record<string, string> = {
    detail: "详情",
    create: "新增",
    update: "更新",
    disable: "停用",
    refresh: "刷新",
    export: "导出",
    print: "打印",
    waybill: "下单",
    tracking: "轨迹",
    cancel: "取消",
  };
  return labels[actionKey] ?? actionKey;
}

function devAdminMenuButtonsFromUnknown(value: unknown): DevAdminMenuButtonPermission[] {
  if (!Array.isArray(value)) return [];
  return value.map((item, index) => {
    const record = asRecord(item);
    return {
      action_key: asString(record.action_key, `action_${index + 1}`),
      action_label: asString(record.action_label, `动作${index + 1}`),
      action_kind: asString(record.action_kind, "private"),
      enabled: asBoolean(record.enabled, true),
      sort_order: asNumber(record.sort_order, (index + 1) * 10),
    };
  });
}

function findDevAdminMenuNode(id: string, nodes = devAdminMenuNodes, parent: DevAdminMenuNode | null = null): {
  node: DevAdminMenuNode;
  parent: DevAdminMenuNode | null;
  siblings: DevAdminMenuNode[];
  index: number;
} | null {
  for (let index = 0; index < nodes.length; index += 1) {
    const node = nodes[index];
    if (node.id === id) return { node, parent, siblings: nodes, index };
    const child = findDevAdminMenuNode(id, node.children, node);
    if (child) return child;
  }
  return null;
}

function updateDevAdminMenuPath(node: DevAdminMenuNode, parent: DevAdminMenuNode) {
  node.parent_id = parent.id;
  node.level = parent.level + 1;
  node.path = `${parent.path}/${node.code}`;
  node.children.forEach((child) => updateDevAdminMenuPath(child, node));
}

function sortDevAdminMenuTree(nodes: DevAdminMenuNode[]) {
  nodes.sort((left, right) => left.sort_order - right.sort_order || left.title.localeCompare(right.title));
  nodes.forEach((node) => sortDevAdminMenuTree(node.children));
}

async function readJsonBody(req: IncomingMessage): Promise<Record<string, unknown>> {
  let raw = "";
  for await (const chunk of req) {
    raw += String(chunk);
  }
  if (!raw) return {};
  const parsed: unknown = JSON.parse(raw);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
  return Object.fromEntries(Object.entries(parsed));
}

function asRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  return Object.fromEntries(Object.entries(value));
}

function hasOwn(record: Record<string, unknown>, key: string) {
  return Object.prototype.hasOwnProperty.call(record, key);
}

function sendJson(res: ServerResponse, statusCode: number, body: unknown) {
  res.statusCode = statusCode;
  res.setHeader("content-type", "application/json; charset=utf-8");
  res.setHeader("cache-control", "no-store");
  res.end(JSON.stringify(body));
}

function asNumber(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function asString(value: unknown, fallback: string) {
  return typeof value === "string" && value.trim() ? value.trim() : fallback;
}

function asNullableString(value: unknown) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function asBoolean(value: unknown, fallback: boolean) {
  return typeof value === "boolean" ? value : fallback;
}
