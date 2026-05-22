import type { ReactNode } from "react";
import { ComponentsGallery } from "./pages/components-gallery";
import { H1LoginPda } from "./pages/h1-login-pda";
import { H1LoginPc } from "./pages/h1-login-pc";
import { H1TokenLogout } from "./pages/h1-token-logout";
import { H1ApiKey } from "./pages/h1-api-key";
import { H1RolePermission } from "./pages/h1-role-permission";
import { H2AuditQuery } from "./pages/h2-audit-query";
import { H2Archive } from "./pages/h2-archive";
import { H3SwaggerUi } from "./pages/h3-swagger";
import { M2DualSign } from "./pages/m2-dual-sign";
import { M2InboundKanban } from "./pages/m2-inbound-kanban";
import { M2InboundTasks } from "./pages/m2-inbound-tasks";
import { M2InboundAccept } from "./pages/m2-inbound-accept";
import { M2Putaway } from "./pages/m2-putaway";
import { M2Reject } from "./pages/m2-reject";
import { M1Items } from "./pages/m1-items";
import { M1Suppliers } from "./pages/m1-suppliers";
import { M1Locations } from "./pages/m1-locations";
import { M4Picking } from "./pages/m4-picking";
import { M4Review } from "./pages/m4-review";
import { M4Manifest } from "./pages/m4-manifest";
import { M4Exception } from "./pages/m4-exception";

/**
 * tabs.tsx — 原型 tab 注册表（数据驱动）
 *
 * 加新 tab 只需追加一项；App.tsx 自动渲染左侧 sidebar 分组
 * 与 governance/visual-baselines/manifest.toml 的 tab 列表一一对应
 */

export type Device = "pc" | "pda" | "pad" | "shared";
export type Group = "组件" | "H1 权限审计" | "H2/H3 治理" | "M1 基础数据" | "M2 采购入库" | "M4 销售出库";

export interface TabDef {
  value: string;
  label: string;
  group: Group;
  device: Device[];
  render: () => ReactNode;
}

const wrap = (children: ReactNode) => <div>{children}</div>;

const dual = (left: ReactNode, leftLabel: string, right: ReactNode, rightLabel: string) => (
  <div className="flex gap-6 flex-wrap items-start">
    <div>
      <p className="text-sm text-muted-foreground mb-2">{leftLabel}</p>
      {left}
    </div>
    <div>
      <p className="text-sm text-muted-foreground mb-2">{rightLabel}</p>
      {right}
    </div>
  </div>
);

const stack = (top: ReactNode, topLabel: string, bottom: ReactNode, bottomLabel: string) => (
  <div className="flex flex-col gap-6 items-start">
    <div>
      <p className="text-sm text-muted-foreground mb-2">{topLabel}</p>
      {top}
    </div>
    <div>
      <p className="text-sm text-muted-foreground mb-2">{bottomLabel}</p>
      {bottom}
    </div>
  </div>
);

export const TABS: TabDef[] = [
  // 组件
  { value: "gallery", label: "组件库", group: "组件", device: ["shared"],
    render: () => <ComponentsGallery /> },

  // H1 权限审计
  { value: "h1-login-pda", label: "PDA 登录", group: "H1 权限审计", device: ["pda"],
    render: () => dual(<H1LoginPda />, "在线 + 工牌扫码", <H1LoginPda offlineMode errorState />, "离线 + 工牌错误") },
  { value: "h1-login-pc", label: "PC 登录", group: "H1 权限审计", device: ["pc"],
    render: () => stack(<H1LoginPc />, "常规状态", <H1LoginPc withCaptcha />, "已连续失败 3 次（带验证码）") },
  { value: "h1-token", label: "Token & 登出", group: "H1 权限审计", device: ["pc"],
    render: () => stack(<H1TokenLogout />, "活跃会话列表", <H1TokenLogout showExpireDialog />, "Token 过期弹窗") },
  { value: "h1-apikey", label: "API Key", group: "H1 权限审计", device: ["pc"],
    render: () => stack(<H1ApiKey />, "列表", <H1ApiKey showCreated />, "刚创建（仅一次性展示密钥）") },
  { value: "h1-role", label: "角色权限", group: "H1 权限审计", device: ["pc"],
    render: () => wrap(<H1RolePermission />) },

  // H2/H3 治理
  { value: "h2-audit", label: "审计查询", group: "H2/H3 治理", device: ["pc"],
    render: () => wrap(<H2AuditQuery />) },
  { value: "h2-archive", label: "数据归档", group: "H2/H3 治理", device: ["pc"],
    render: () => wrap(<H2Archive />) },
  { value: "h3-swagger", label: "API 文档", group: "H2/H3 治理", device: ["pc"],
    render: () => wrap(<H3SwaggerUi />) },

  // M1 基础数据
  { value: "m1-items", label: "商品档案", group: "M1 基础数据", device: ["pc"],
    render: () => wrap(<M1Items />) },
  { value: "m1-suppliers", label: "供应商资质", group: "M1 基础数据", device: ["pc"],
    render: () => wrap(<M1Suppliers />) },
  { value: "m1-locations", label: "仓库与库位", group: "M1 基础数据", device: ["pc"],
    render: () => wrap(<M1Locations />) },

  // M2 采购入库
  { value: "m2-tasks", label: "PDA 任务列表", group: "M2 采购入库", device: ["pda"],
    render: () => wrap(<M2InboundTasks />) },
  { value: "m2-accept", label: "PDA 14 步验收", group: "M2 采购入库", device: ["pda"],
    render: () => wrap(<M2InboundAccept />) },
  { value: "m2-putaway", label: "PDA 上架", group: "M2 采购入库", device: ["pda"],
    render: () => wrap(<M2Putaway />) },
  { value: "m2-reject", label: "PDA 拒收", group: "M2 采购入库", device: ["pda"],
    render: () => wrap(<M2Reject />) },
  { value: "m2-dual-sign", label: "PDA 双人签字", group: "M2 采购入库", device: ["pda"],
    render: () => wrap(<M2DualSign />) },
  { value: "m2-kanban", label: "收货看板", group: "M2 采购入库", device: ["pc", "pad"],
    render: () => wrap(<M2InboundKanban />) },

  // M4 销售出库
  { value: "m4-picking", label: "PDA 拣货", group: "M4 销售出库", device: ["pda"],
    render: () => wrap(<M4Picking />) },
  { value: "m4-review", label: "PDA 复核", group: "M4 销售出库", device: ["pda"],
    render: () => wrap(<M4Review />) },
  { value: "m4-manifest", label: "随货同行单", group: "M4 销售出库", device: ["pc"],
    render: () => wrap(<M4Manifest />) },
  { value: "m4-exception", label: "PDA 异常拣货", group: "M4 销售出库", device: ["pda"],
    render: () => wrap(<M4Exception />) },
];

export const DEVICE_META: Record<Device, { label: string; color: string }> = {
  pc: { label: "PC", color: "bg-primary/10 text-primary border-primary/20" },
  pda: { label: "PDA", color: "bg-wms-warning/10 text-wms-warning border-wms-warning/30" },
  pad: { label: "PAD", color: "bg-wms-cold/10 text-wms-cold border-wms-cold/30" },
  shared: { label: "通用", color: "bg-muted text-muted-foreground border-input" },
};

export const GROUP_ORDER: Group[] = [
  "组件",
  "H1 权限审计",
  "H2/H3 治理",
  "M1 基础数据",
  "M2 采购入库",
  "M4 销售出库",
];
