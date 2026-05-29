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
import { M3Inventory } from "./pages/m3-inventory";
import { M3Stocktake } from "./pages/m3-stocktake";
import { M5ColdMonitor } from "./pages/m5-cold-monitor";
import { M6Purchase } from "./pages/m6-purchase";
import { M6Sales } from "./pages/m6-sales";
import { M6Inventory } from "./pages/m6-inventory";
import { M6Cold } from "./pages/m6-cold";
import { M6Expiry } from "./pages/m6-expiry";
import { M6Custom } from "./pages/m6-custom";
import { M6Special } from "./pages/m6-special";
import { M6Subscriptions } from "./pages/m6-subscriptions";
import { M2Asn } from "./pages/m2-asn";
import { M2Hours } from "./pages/m2-hours";
import { M8StorageFeeRules } from "./pages/m8-storage-fee";
import { M10InTransitTemp } from "./pages/m10-in-transit-temp";
import { FULL_MATRIX_SPECS } from "./prototype-kit/full-matrix-specs";
import type { MatrixPrototypeSpec } from "./prototype-kit/types";
import { UniversalPrototypePage } from "./prototype-kit/UniversalPrototypePage";

/**
 * tabs.tsx — 原型 tab 注册表（数据驱动）
 *
 * 加新 tab 只需追加一项；App.tsx 自动渲染左侧 sidebar 分组
 * 与 governance/visual-baselines/manifest.toml 的 tab 列表一一对应
 */

export type Device = "pc" | "pda" | "pad" | "h5" | "shared";
export type Group =
  | "组件"
  | "H1 权限审计"
  | "H2/H3 治理"
  | "H4/H5 协同"
  | "H-AL 告警"
  | "H-DOCK 月台"
  | "H-Driver 司机端"
  | "H-Store 门店端"
  | "M1 基础数据"
  | "M2 采购入库"
  | "M3 库存核心"
  | "M4 销售出库"
  | "M5 冷链监控"
  | "M6 GSP 报表"
  | "M8 连锁"
  | "M9 计费"
  | "M10 TMS"
  | "M-TE 任务引擎"
  | "M-RP 补货"
  | "M-PK 包装站"
  | "M-VR 规则"
  | "M-QL 质量联系单"
  | "M-SA 报损报溢"
  | "M-RC 对账"
  | "M-DI 药检单"
  | "M-BA 批号调整"
  | "M-TC 追溯码"
  | "M-PM 参数对照"
  | "M8 计费 / M10 TMS";

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

const HAND_BUILT_TABS: TabDef[] = [
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
  { value: "m2-asn", label: "ASN 接收", group: "M2 采购入库", device: ["pc"],
    render: () => wrap(<M2Asn />) },
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
  { value: "m2-hours", label: "工时统计", group: "M2 采购入库", device: ["pc"],
    render: () => wrap(<M2Hours />) },

  // M3 库存核心
  { value: "m3-inventory", label: "库存查询", group: "M3 库存核心", device: ["pc"],
    render: () => wrap(<M3Inventory />) },
  { value: "m3-stocktake", label: "PDA 盘点", group: "M3 库存核心", device: ["pda"],
    render: () => wrap(<M3Stocktake />) },

  // M4 销售出库
  { value: "m4-picking", label: "PDA 拣货", group: "M4 销售出库", device: ["pda"],
    render: () => wrap(<M4Picking />) },
  { value: "m4-review", label: "PDA 复核", group: "M4 销售出库", device: ["pda"],
    render: () => wrap(<M4Review />) },
  { value: "m4-manifest", label: "随货同行单", group: "M4 销售出库", device: ["pc"],
    render: () => wrap(<M4Manifest />) },
  { value: "m4-exception", label: "PDA 异常拣货", group: "M4 销售出库", device: ["pda"],
    render: () => wrap(<M4Exception />) },

  // M5 冷链监控
  { value: "m5-cold", label: "冷链监控", group: "M5 冷链监控", device: ["pc"],
    render: () => wrap(<M5ColdMonitor />) },

  // M6 GSP 报表（5 张独立报表）
  { value: "m6-purchase", label: "采购入库月报", group: "M6 GSP 报表", device: ["pc"],
    render: () => wrap(<M6Purchase />) },
  { value: "m6-sales", label: "销售出库月报", group: "M6 GSP 报表", device: ["pc"],
    render: () => wrap(<M6Sales />) },
  { value: "m6-inventory", label: "库存盘点月报", group: "M6 GSP 报表", device: ["pc"],
    render: () => wrap(<M6Inventory />) },
  { value: "m6-cold", label: "冷链温度月报", group: "M6 GSP 报表", device: ["pc"],
    render: () => wrap(<M6Cold />) },
  { value: "m6-expiry", label: "近效期/不合格月报", group: "M6 GSP 报表", device: ["pc"],
    render: () => wrap(<M6Expiry />) },
  { value: "m6-custom", label: "业务报表（自定义）", group: "M6 GSP 报表", device: ["pc"],
    render: () => wrap(<M6Custom />) },
  { value: "m6-special", label: "特殊药品台账", group: "M6 GSP 报表", device: ["pc"],
    render: () => wrap(<M6Special />) },
  { value: "m6-subscriptions", label: "报表订阅", group: "M6 GSP 报表", device: ["pc"],
    render: () => wrap(<M6Subscriptions />) },

  // M8 计费 / M10 TMS
  { value: "m8-storage-fee", label: "仓储费规则", group: "M8 计费 / M10 TMS", device: ["pc"],
    render: () => wrap(<M8StorageFeeRules />) },
  { value: "m10-in-transit", label: "在途温控", group: "M8 计费 / M10 TMS", device: ["pc"],
    render: () => wrap(<M10InTransitTemp />) },
];

const HAND_BUILT_BY_VALUE = new Map(HAND_BUILT_TABS.map((tab) => [tab.value, tab]));

const GROUP_BY_MODULE: Record<string, Group> = {
  H1: "H1 权限审计",
  H2: "H2/H3 治理",
  H3: "H2/H3 治理",
  H4: "H4/H5 协同",
  H5: "H4/H5 协同",
  AL: "H-AL 告警",
  DOCK: "H-DOCK 月台",
  DR: "H-Driver 司机端",
  ST: "H-Store 门店端",
  M1: "M1 基础数据",
  M2: "M2 采购入库",
  M3: "M3 库存核心",
  M4: "M4 销售出库",
  M5: "M5 冷链监控",
  M6: "M6 GSP 报表",
  M8: "M8 连锁",
  M9: "M9 计费",
  M10: "M10 TMS",
  TE: "M-TE 任务引擎",
  RP: "M-RP 补货",
  PK: "M-PK 包装站",
  VR: "M-VR 规则",
  QL: "M-QL 质量联系单",
  SA: "M-SA 报损报溢",
  RC: "M-RC 对账",
  DI: "M-DI 药检单",
  BA: "M-BA 批号调整",
  TC: "M-TC 追溯码",
  MPM: "M-PM 参数对照",
};

const LEGACY_SLUGS: Record<string, string> = {
  "pc-us-h1-001": "h1-login-pc",
  "pda-us-h1-001": "h1-login-pda",
  "pc-us-h1-002": "h1-role",
  "pc-us-h1-005": "h1-token",
  "pc-us-h1-006": "h1-apikey",
  "pc-us-h2-002": "h2-audit",
  "pc-us-h2-004": "h2-archive",
  "pc-us-h2-006": "h2-archive",
  "pc-us-h3-004": "h3-swagger",
  "pc-us-m1-001": "m1-items",
  "pc-us-m1-002": "m1-suppliers",
  "pc-us-m1-004": "m1-locations",
  "pc-us-m1-006": "h1-role",
  "pc-us-m2-001": "m2-asn",
  "pda-us-m2-002": "m2-tasks",
  "pda-us-m2-003": "m2-accept",
  "pda-us-m2-004": "m2-dual-sign",
  "pda-us-m2-005": "m2-putaway",
  "pda-us-m2-006": "m2-reject",
  "pc-us-m2-008": "m2-kanban",
  "pad-us-m2-008": "m2-kanban",
  "pc-us-m2-009": "m2-hours",
  "pc-us-m3-001": "m3-inventory",
  "pda-us-m3-005": "m3-stocktake",
  "pda-us-m4-003": "m4-picking",
  "pda-us-m4-004": "m4-review",
  "pc-us-m4-005": "m4-manifest",
  "pda-us-m4-008": "m4-exception",
  "pc-us-m5-002": "m5-cold",
  "pc-us-m6-001": "m6-purchase",
  "pc-us-m6-002": "m6-custom",
  "pc-us-m6-003": "m6-custom",
  "pc-us-m6-004": "m6-special",
  "pc-us-m8-001": "m8-storage-fee",
  "pc-us-m10-002": "m10-in-transit",
};

function legacyKeyFor(spec: MatrixPrototypeSpec) {
  return `${spec.end}-${spec.storyId.toLowerCase()}`;
}

function MergedPrototypeNotice(props: {
  spec: MatrixPrototypeSpec;
  reusedSlug: string;
  reusedLabel: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-3">
      <div className="rounded-md border bg-primary/5 px-4 py-3 flex items-center gap-4">
        <div className="min-w-0 flex-1">
          <div className="text-xs font-mono text-primary">
            {props.spec.storyId} · {props.spec.moduleCode} · GSP · merged prototype
          </div>
          <div className="text-sm font-medium truncate">
            复用 {props.reusedLabel}
          </div>
          <div className="text-xs text-muted-foreground truncate">
            矩阵备注：{props.spec.reason} · #{props.spec.slug} → #{props.reusedSlug}
          </div>
        </div>
      </div>
      {props.children}
    </div>
  );
}

function renderGeneratedTab(spec: MatrixPrototypeSpec) {
  const reusedSlug = LEGACY_SLUGS[legacyKeyFor(spec)];
  const reusedTab = reusedSlug ? HAND_BUILT_BY_VALUE.get(reusedSlug) : undefined;

  if (reusedSlug && reusedTab) {
    return () => wrap(
      <MergedPrototypeNotice spec={spec} reusedSlug={reusedSlug} reusedLabel={reusedTab.label}>
        {reusedTab.render()}
      </MergedPrototypeNotice>
    );
  }

  return () => wrap(<UniversalPrototypePage spec={spec} />);
}

const generatedTabs: TabDef[] = FULL_MATRIX_SPECS
  .map((spec) => ({
    value: spec.slug,
    label: `${spec.storyId.replace("US-", "")} ${spec.title}`,
    group: GROUP_BY_MODULE[spec.moduleCode] ?? "M-PM 参数对照",
    device: [spec.end],
    render: renderGeneratedTab(spec),
  }));

export const TABS: TabDef[] = [...HAND_BUILT_TABS, ...generatedTabs];

export const DEVICE_META: Record<Device, { label: string; color: string }> = {
  pc: { label: "PC", color: "bg-primary/10 text-primary border-primary/20" },
  pda: { label: "PDA", color: "bg-wms-warning/10 text-wms-warning border-wms-warning/30" },
  pad: { label: "PAD", color: "bg-wms-cold/10 text-wms-cold border-wms-cold/30" },
  h5: { label: "H5", color: "bg-wms-success/10 text-wms-success border-wms-success/30" },
  shared: { label: "通用", color: "bg-muted text-muted-foreground border-input" },
};

export const GROUP_ORDER: Group[] = [
  "组件",
  "H1 权限审计",
  "H2/H3 治理",
  "H4/H5 协同",
  "H-AL 告警",
  "H-DOCK 月台",
  "H-Driver 司机端",
  "H-Store 门店端",
  "M1 基础数据",
  "M2 采购入库",
  "M3 库存核心",
  "M4 销售出库",
  "M5 冷链监控",
  "M6 GSP 报表",
  "M8 连锁",
  "M9 计费",
  "M10 TMS",
  "M-TE 任务引擎",
  "M-RP 补货",
  "M-PK 包装站",
  "M-VR 规则",
  "M-QL 质量联系单",
  "M-SA 报损报溢",
  "M-RC 对账",
  "M-DI 药检单",
  "M-BA 批号调整",
  "M-TC 追溯码",
  "M-PM 参数对照",
  "M8 计费 / M10 TMS",
];
