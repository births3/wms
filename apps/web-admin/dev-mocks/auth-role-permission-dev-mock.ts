import type { IncomingMessage, ServerResponse } from "node:http";

import { asNullableString, asString, readJsonBody, sendError, sendJson } from "./web-admin-dev-mock-core-common";

interface DevRole {
  id: string;
  role_code: string;
  role_name: string;
  data_scope: string;
  parent_role_id: string | null;
  permission_codes: string[];
  created_at: string;
}

interface DevPermission {
  id: string;
  permission_code: string;
  permission_name: string;
}

interface DevRoleUser {
  user_id: string;
  username: string;
  display_name: string;
  role_ids: string[];
}

const permissions: DevPermission[] = [
  ["h1.roles.manage", "H1 角色权限维护"],
  ["h1.menu.read", "H1 菜单读取"],
  ["h1.menu.write", "H1 菜单维护"],
  ["h1.menu.publish", "H1 菜单发布"],
  ["m1.master_data.read", "基础档案读取"],
  ["m1.master_data.write", "基础档案写入"],
  ["m2.write", "入库作业写入"],
  ["m3.read", "库存读取"],
  ["m4.read", "出库读取"],
  ["audit.read", "审计查询"],
  ["h4.notify.read", "H4 通知读取"],
  ["h5.express.read", "H5 快递读取"],
  ["mcg.document_numbering.read", "M-CG 单据号读取"],
  ["mcg.document_numbering.write", "M-CG 单据号维护"],
].map(([permission_code, permission_name], index) => ({
  id: `00000000-0000-0000-0000-${String(9100 + index).padStart(12, "0")}`,
  permission_code,
  permission_name,
}));

const roleIds = Array.from({ length: 8 }, (_, index) => `00000000-0000-0000-0000-${String(9000 + index).padStart(12, "0")}`);
const roles: DevRole[] = [
  ["system_admin", "系统管理员", "all"],
  ["warehouse_manager", "仓库主管", "warehouse"],
  ["receiving_clerk", "收货员", "warehouse"],
  ["maintenance_clerk", "养护员", "warehouse"],
  ["custodian", "保管员", "warehouse"],
  ["owner_user", "货主", "owner"],
  ["store_user", "门店用户", "self"],
  ["driver", "司机", "self"],
].map(([role_code, role_name, data_scope], index) => ({
  id: roleIds[index],
  role_code,
  role_name,
  data_scope,
  parent_role_id: null,
  permission_codes: index === 0 ? permissions.map((item) => item.permission_code) : rolePermissionSeed(role_code),
  created_at: `2026-07-12T00:${String(index).padStart(2, "0")}:00.000Z`,
}));

const users: DevRoleUser[] = ([
  ["admin", "系统管理员", [roleIds[0]]],
  ["warehouse.manager", "仓库主管", [roleIds[1]]],
  ["receiving.clerk", "收货员", [roleIds[2]]],
  ["owner.user", "货主用户", [roleIds[5]]],
] as Array<[string, string, string[]]>).map(([username, display_name, role_ids], index) => ({
  user_id: `00000000-0000-0000-0000-${String(9200 + index).padStart(12, "0")}`,
  username,
  display_name,
  role_ids,
}));

export async function handleRolePermissionDevMock(req: IncomingMessage, res: ServerResponse, pathname: string) {
  if (req.method === "GET" && pathname === "/api/v1/auth/roles") {
    sendJson(res, 200, { items: roles });
    return;
  }
  if (req.method === "GET" && pathname === "/api/v1/auth/permissions") {
    sendJson(res, 200, { items: permissions });
    return;
  }
  if (req.method === "GET" && pathname === "/api/v1/auth/users") {
    sendJson(res, 200, { items: users });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/auth/roles") {
    await createRole(req, res);
    return;
  }
  const permissionPath = pathname.match(/^\/api\/v1\/auth\/roles\/([^/]+)\/permissions$/);
  if (req.method === "PUT" && permissionPath) {
    await replacePermissions(req, res, permissionPath[1]);
    return;
  }
  const rolePath = pathname.match(/^\/api\/v1\/auth\/roles\/([^/]+)$/);
  if (rolePath && req.method === "PUT") {
    await updateRole(req, res, rolePath[1]);
    return;
  }
  if (rolePath && req.method === "DELETE") {
    deleteRole(res, rolePath[1]);
    return;
  }
  if (req.method === "PUT" && pathname === "/api/v1/auth/user-roles/batch") {
    await batchAssignRoles(req, res);
    return;
  }
  sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Role permission dev mock route not found");
}

async function createRole(req: IncomingMessage, res: ServerResponse) {
  const body = await readJsonBody(req);
  const roleCode = asNullableString(body.role_code);
  const roleName = asNullableString(body.role_name);
  const dataScope = asNullableString(body.data_scope);
  if (!roleCode || !roleName || !dataScope || !["self", "warehouse", "owner", "all"].includes(dataScope)) {
    sendError(res, 422, "DEV_MOCK_ROLE_INVALID", "角色参数不完整");
    return;
  }
  if (roles.some((role) => role.role_code.toLowerCase() === roleCode.toLowerCase())) {
    sendError(res, 409, "H1_ROLE_DUPLICATE", "角色编码已存在");
    return;
  }
  const role: DevRole = {
    id: crypto.randomUUID(),
    role_code: roleCode,
    role_name: roleName,
    data_scope: dataScope,
    parent_role_id: asNullableString(body.parent_role_id),
    permission_codes: [],
    created_at: new Date().toISOString(),
  };
  roles.unshift(role);
  sendJson(res, 200, role);
}

async function updateRole(req: IncomingMessage, res: ServerResponse, id: string) {
  const role = roles.find((item) => item.id === id);
  if (!role) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Role not found");
    return;
  }
  const body = await readJsonBody(req);
  const dataScope = asString(body.data_scope, role.data_scope);
  if (!["self", "warehouse", "owner", "all"].includes(dataScope)) {
    sendError(res, 422, "DEV_MOCK_ROLE_INVALID", "数据范围非法");
    return;
  }
  role.role_name = asString(body.role_name, role.role_name);
  role.data_scope = dataScope;
  role.parent_role_id = asNullableString(body.parent_role_id);
  sendJson(res, 200, role);
}

function deleteRole(res: ServerResponse, id: string) {
  const index = roles.findIndex((role) => role.id === id);
  if (index < 0) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Role not found");
    return;
  }
  if (users.some((user) => user.role_ids.includes(id)) || roles.some((role) => role.parent_role_id === id)) {
    sendError(res, 409, "H1_ROLE_IN_USE", "角色正在使用中");
    return;
  }
  roles.splice(index, 1);
  sendJson(res, 200, { id });
}

async function replacePermissions(req: IncomingMessage, res: ServerResponse, id: string) {
  const role = roles.find((item) => item.id === id);
  if (!role) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Role not found");
    return;
  }
  const body = await readJsonBody(req);
  const codes = stringArray(body.permission_codes);
  if (codes.some((code) => !permissions.some((permission) => permission.permission_code === code))) {
    sendError(res, 422, "H1_PERMISSION_NOT_FOUND", "权限码不存在");
    return;
  }
  role.permission_codes = Array.from(new Set(codes)).sort();
  sendJson(res, 200, role);
}

async function batchAssignRoles(req: IncomingMessage, res: ServerResponse) {
  const body = await readJsonBody(req);
  const userIds = stringArray(body.user_ids);
  const roleIds = stringArray(body.role_ids);
  if (!userIds.length || !roleIds.length || userIds.some((id) => !users.some((user) => user.user_id === id)) || roleIds.some((id) => !roles.some((role) => role.id === id))) {
    sendError(res, 422, "DEV_MOCK_ROLE_ASSIGN_INVALID", "用户或角色选择无效");
    return;
  }
  users.filter((user) => userIds.includes(user.user_id)).forEach((user) => { user.role_ids = Array.from(new Set(roleIds)); });
  sendJson(res, 200, { user_ids: userIds, role_ids: roleIds });
}

function rolePermissionSeed(roleCode: string) {
  const seeds: Record<string, string[]> = {
    warehouse_manager: ["m1.master_data.read", "m2.write", "m3.read", "m4.read", "audit.read"],
    receiving_clerk: ["m1.master_data.read", "m2.write"],
    maintenance_clerk: ["m1.master_data.read", "m3.read"],
    custodian: ["m3.read", "m4.read"],
    owner_user: ["m1.master_data.read", "m3.read", "m4.read"],
    store_user: ["m4.read"],
    driver: ["m4.read", "h5.express.read"],
  };
  return seeds[roleCode] ?? [];
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string" && item.trim().length > 0) : [];
}
