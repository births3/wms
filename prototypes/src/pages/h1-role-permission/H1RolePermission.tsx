import { useState } from "react";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { StatusBadge } from "@/components/business";
import { Plus, Copy, Search, Users, ChevronDown, ChevronRight } from "lucide-react";

interface Role {
  id: string;
  name: string;
  scope: string;
  members: number;
  builtin?: boolean;
}

const ROLES: Role[] = [
  { id: "r1", name: "系统管理员", scope: "全局", members: 2, builtin: true },
  { id: "r2", name: "仓库主管", scope: "本仓", members: 5 },
  { id: "r3", name: "收货员（验收岗）", scope: "本仓 · 入库", members: 12 },
  { id: "r4", name: "养护员", scope: "本仓 · 库存", members: 4 },
  { id: "r5", name: "保管员", scope: "本仓 · 出库", members: 8 },
  { id: "r6", name: "质量负责人", scope: "全仓", members: 1 },
  { id: "r7", name: "门店用户", scope: "门店级", members: 23 },
];

interface PermGroup {
  module: string;
  description: string;
  perms: { code: string; label: string }[];
}

const PERM_GROUPS: PermGroup[] = [
  {
    module: "M2 入库",
    description: "ASN / 收货 / 验收 / 上架",
    perms: [
      { code: "M2.asn.read", label: "查看 ASN" },
      { code: "M2.asn.write", label: "创建 / 编辑 ASN" },
      { code: "M2.receive.write", label: "PDA 收货" },
      { code: "M2.verify.write", label: "PDA 验收" },
      { code: "M2.dual_sign.approve", label: "双人复核签字" },
      { code: "M2.putaway.write", label: "上架" },
    ],
  },
  {
    module: "M3 库存",
    description: "查询 / 移库 / 状态变更",
    perms: [
      { code: "M3.inventory.read", label: "实时库存查询" },
      { code: "M3.move.write", label: "库内移库" },
      { code: "M3.status.approve", label: "库存状态变更" },
      { code: "M3.stocktake.write", label: "盘点作业" },
      { code: "M3.maintenance.write", label: "在库养护" },
    ],
  },
  {
    module: "M4 出库",
    description: "订单 / 拣选 / 复核",
    perms: [
      { code: "M4.order.read", label: "查看订单" },
      { code: "M4.order.write", label: "创建订单" },
      { code: "M4.pick.write", label: "PDA 拣选" },
      { code: "M4.verify.approve", label: "出库复核" },
    ],
  },
  {
    module: "H2 审计",
    description: "审计追踪查询（敏感）",
    perms: [
      { code: "H2.events.read.self", label: "查看自己的操作" },
      { code: "H2.events.read.warehouse", label: "查看本仓所有" },
      { code: "H2.events.read.global", label: "查看全部（系统管理员）" },
      { code: "H2.export.write", label: "导出审计 CSV/PDF" },
    ],
  },
];

// 角色 → 权限映射（Mock）
const ROLE_PERMS: Record<string, Set<string>> = {
  r1: new Set(PERM_GROUPS.flatMap((g) => g.perms.map((p) => p.code))), // 系统管理员：全部
  r2: new Set([
    "M2.asn.read", "M2.dual_sign.approve",
    "M3.inventory.read", "M3.status.approve",
    "M4.order.read", "M4.verify.approve",
    "H2.events.read.warehouse", "H2.export.write",
  ]),
  r3: new Set(["M2.asn.read", "M2.receive.write", "M2.verify.write"]),
  r4: new Set(["M3.inventory.read", "M3.maintenance.write"]),
  r5: new Set(["M3.inventory.read", "M3.move.write", "M2.putaway.write", "M4.order.read", "M4.pick.write"]),
  r6: new Set(["M2.dual_sign.approve", "M3.status.approve", "M4.verify.approve", "H2.events.read.warehouse"]),
  r7: new Set(["M4.order.read"]),
};

interface Member {
  user: string;
  warehouse: string;
  joinedAt: string;
  status: "active" | "expired";
}

const MEMBERS_BY_ROLE: Record<string, Member[]> = {
  r3: [
    { user: "张三 (u001)", warehouse: "天河仓", joinedAt: "2025-08-01", status: "active" },
    { user: "李四 (u002)", warehouse: "天河仓", joinedAt: "2025-09-15", status: "active" },
    { user: "王五 (u003)", warehouse: "白云仓", joinedAt: "2026-01-10", status: "active" },
    { user: "赵六 (u004)", warehouse: "天河仓", joinedAt: "2026-03-22", status: "expired" },
  ],
};

/**
 * H1RolePermission — 角色与权限管理（矩阵）
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-H1-002 / US-M1-006（角色矩阵 + 权限码 + 用户分配）
 * Wave：Wave 0.5（P0 必交付）
 * 业务约束：内置角色（系统管理员）不可删除；权限码层级化（模块.对象.动作）
 *
 * @example
 *   <H1RolePermission />
 */
export function H1RolePermission() {
  const [selectedRole, setSelectedRole] = useState<Role>(ROLES[2]);
  const [openGroups, setOpenGroups] = useState<Record<string, boolean>>(
    Object.fromEntries(PERM_GROUPS.map((g) => [g.module, true]))
  );

  const rolePerms = ROLE_PERMS[selectedRole.id] ?? new Set<string>();
  const members = MEMBERS_BY_ROLE[selectedRole.id] ?? [];

  return (
    <div className="w-[1280px] min-h-[800px] bg-muted/40 border rounded-xl p-6 font-sans">
      <header className="flex justify-between items-center mb-6">
        <div>
          <h1 className="text-xl font-semibold">角色与权限管理</h1>
          <p className="text-sm text-muted-foreground mt-1">H1-002 / M1-006 · 权限码层级 · 内置角色不可删</p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">
            <Copy className="size-4" />
            复制为新角色
          </Button>
          <Button size="sm">
            <Plus className="size-4" />
            新建角色
          </Button>
        </div>
      </header>

      <div className="grid grid-cols-[260px_1fr_300px] gap-4">
        {/* 左：角色列表 */}
        <Card className="p-0 overflow-hidden self-start">
          <div className="p-3 border-b">
            <div className="relative">
              <Search className="size-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
              <Input className="h-8 text-xs pl-8" placeholder="搜索角色" />
            </div>
          </div>
          <ul className="py-1 max-h-[600px] overflow-auto">
            {ROLES.map((r) => {
              const active = r.id === selectedRole.id;
              return (
                <li
                  key={r.id}
                  onClick={() => setSelectedRole(r)}
                  className={`px-3 py-2.5 cursor-pointer border-l-2 ${
                    active
                      ? "bg-primary/10 border-primary"
                      : "border-transparent hover:bg-accent/40 hover:border-muted-foreground/20"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">{r.name}</span>
                    {r.builtin && <span className="text-[10px] px-1.5 py-0.5 bg-muted rounded text-muted-foreground">内置</span>}
                  </div>
                  <div className="flex items-center justify-between mt-1 text-xs text-muted-foreground">
                    <span>{r.scope}</span>
                    <span className="inline-flex items-center gap-1">
                      <Users className="size-3" />
                      {r.members}
                    </span>
                  </div>
                </li>
              );
            })}
          </ul>
        </Card>

        {/* 中：权限矩阵 */}
        <Card className="p-0 overflow-hidden">
          <div className="px-4 py-3 border-b bg-muted/30 flex items-center justify-between">
            <div>
              <h3 className="text-sm font-semibold">{selectedRole.name} · 权限</h3>
              <p className="text-xs text-muted-foreground mt-0.5">
                共勾选 {rolePerms.size} / {PERM_GROUPS.flatMap((g) => g.perms).length} 项
                {selectedRole.builtin && "（内置角色，仅可查看）"}
              </p>
            </div>
            <div className="flex gap-2">
              <Button variant="outline" size="sm" disabled={selectedRole.builtin}>全选当前组</Button>
              <Button size="sm" disabled={selectedRole.builtin}>保存</Button>
            </div>
          </div>
          <div className="max-h-[600px] overflow-auto">
            {PERM_GROUPS.map((g) => {
              const open = openGroups[g.module] ?? true;
              const groupSelected = g.perms.filter((p) => rolePerms.has(p.code)).length;
              return (
                <div key={g.module} className="border-b last:border-b-0">
                  <button
                    onClick={() => setOpenGroups({ ...openGroups, [g.module]: !open })}
                    className="w-full px-4 py-2.5 bg-muted/20 hover:bg-muted/40 flex items-center justify-between text-left"
                  >
                    <div className="flex items-center gap-2">
                      {open ? <ChevronDown className="size-3.5" /> : <ChevronRight className="size-3.5" />}
                      <span className="text-sm font-medium">{g.module}</span>
                      <span className="text-xs text-muted-foreground">— {g.description}</span>
                    </div>
                    <span className="text-xs text-muted-foreground">{groupSelected}/{g.perms.length}</span>
                  </button>
                  {open && (
                    <ul className="divide-y">
                      {g.perms.map((p) => {
                        const checked = rolePerms.has(p.code);
                        return (
                          <li key={p.code} className="px-4 py-2 flex items-center gap-3 hover:bg-accent/30">
                            <Checkbox checked={checked} disabled={selectedRole.builtin} />
                            <code className="font-mono text-xs text-muted-foreground w-44 shrink-0">{p.code}</code>
                            <span className="text-sm">{p.label}</span>
                            {p.code.endsWith(".approve") && (
                              <StatusBadge status="pending" size="sm" label="审批级" />
                            )}
                          </li>
                        );
                      })}
                    </ul>
                  )}
                </div>
              );
            })}
          </div>
        </Card>

        {/* 右：成员列表 */}
        <Card className="p-0 overflow-hidden self-start">
          <div className="px-4 py-3 border-b bg-muted/30 flex items-center justify-between">
            <h3 className="text-sm font-semibold">成员（{selectedRole.members}）</h3>
            <Button variant="outline" size="sm" className="h-7 text-xs">
              <Plus className="size-3" />
              添加
            </Button>
          </div>
          <ul className="divide-y max-h-[600px] overflow-auto">
            {members.length === 0 ? (
              <li className="px-4 py-8 text-center text-sm text-muted-foreground">
                Mock 数据仅含「{ROLES[2].name}」角色成员
              </li>
            ) : (
              members.map((m, i) => (
                <li key={i} className="px-4 py-2.5 hover:bg-accent/30">
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">{m.user}</span>
                    <StatusBadge
                      status={m.status === "active" ? "qualified" : "expired"}
                      size="sm"
                      label={m.status === "active" ? "在职" : "已离职"}
                    />
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    {m.warehouse} · 加入 {m.joinedAt}
                  </div>
                </li>
              ))
            )}
          </ul>
        </Card>
      </div>
    </div>
  );
}
