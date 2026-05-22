import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import "./globals.css";
import { Button } from "./components/ui";
import { H1LoginPda } from "./pages/h1-login-pda";
import { H1LoginPc } from "./pages/h1-login-pc";
import { H1TokenLogout } from "./pages/h1-token-logout";
import { H1ApiKey } from "./pages/h1-api-key";
import { H1RolePermission } from "./pages/h1-role-permission";
import { H2AuditQuery } from "./pages/h2-audit-query";
import { H2Archive } from "./pages/h2-archive";
import { H3SwaggerUi } from "./pages/h3-swagger";
import { ComponentsGallery } from "./pages/components-gallery";
import { M2DualSign } from "./pages/m2-dual-sign";
import { M2InboundKanban } from "./pages/m2-inbound-kanban";
import { M2InboundTasks } from "./pages/m2-inbound-tasks";
import { M2InboundAccept } from "./pages/m2-inbound-accept";
import { M2Putaway } from "./pages/m2-putaway";
import { M2Reject } from "./pages/m2-reject";

type Tab =
  | "gallery"
  | "h1-login-pda"
  | "h1-login-pc"
  | "h1-token"
  | "h1-apikey"
  | "h1-role"
  | "h2-audit"
  | "h2-archive"
  | "h3-swagger"
  | "m2-dual-sign"
  | "m2-kanban"
  | "m2-tasks"
  | "m2-accept"
  | "m2-putaway"
  | "m2-reject";

const TABS: { value: Tab; label: string; group: "组件" | "H1" | "H2" | "H3" | "M2" }[] = [
  { value: "gallery", label: "组件库", group: "组件" },
  { value: "h1-login-pda", label: "PDA 登录", group: "H1" },
  { value: "h1-login-pc", label: "PC 登录", group: "H1" },
  { value: "h1-token", label: "Token & 登出", group: "H1" },
  { value: "h1-apikey", label: "API Key", group: "H1" },
  { value: "h1-role", label: "角色权限", group: "H1" },
  { value: "h2-audit", label: "审计查询", group: "H2" },
  { value: "h2-archive", label: "数据归档", group: "H2" },
  { value: "h3-swagger", label: "API 文档", group: "H3" },
  { value: "m2-tasks", label: "M2 任务列表", group: "M2" },
  { value: "m2-accept", label: "M2 14步验收", group: "M2" },
  { value: "m2-putaway", label: "M2 上架", group: "M2" },
  { value: "m2-reject", label: "M2 拒收", group: "M2" },
  { value: "m2-dual-sign", label: "M2 双人签字", group: "M2" },
  { value: "m2-kanban", label: "M2 收货看板", group: "M2" },
];

function App() {
  const initial = (window.location.hash.replace("#", "") as Tab) || "gallery";
  const [tab, setTab] = useState<Tab>(
    TABS.some((t) => t.value === initial) ? initial : "gallery"
  );

  return (
    <div className="min-h-screen bg-muted/40 p-6 font-sans">
      <div className="flex items-center gap-4 mb-6 max-w-[1400px] mx-auto flex-wrap">
        <h1 className="text-xl font-semibold whitespace-nowrap">WMS 原型预览</h1>
        <span className="text-xs text-muted-foreground whitespace-nowrap">P0 + 组件库 · ADR-0021/0022</span>
        <div className="ml-auto flex gap-1.5 flex-wrap">
          {TABS.map((t) => (
            <Button
              key={t.value}
              variant={tab === t.value ? "default" : "outline"}
              size="sm"
              onClick={() => {
                setTab(t.value);
                window.location.hash = t.value;
              }}
            >
              {t.label}
            </Button>
          ))}
        </div>
      </div>

      {tab === "gallery" && <ComponentsGallery />}
      {tab === "h1-login-pda" && (
        <div className="flex justify-center gap-8 flex-wrap">
          <div>
            <p className="text-sm text-muted-foreground mb-2">在线 + 工牌扫码</p>
            <H1LoginPda />
          </div>
          <div>
            <p className="text-sm text-muted-foreground mb-2">离线 + 工牌错误</p>
            <H1LoginPda offlineMode errorState />
          </div>
        </div>
      )}
      {tab === "h1-login-pc" && (
        <div className="flex flex-col items-center gap-6">
          <div>
            <p className="text-sm text-muted-foreground mb-2">常规状态</p>
            <H1LoginPc />
          </div>
          <div>
            <p className="text-sm text-muted-foreground mb-2">已连续失败 3 次（带验证码）</p>
            <H1LoginPc withCaptcha />
          </div>
        </div>
      )}
      {tab === "h1-token" && (
        <div className="flex flex-col items-center gap-6">
          <div>
            <p className="text-sm text-muted-foreground mb-2">活跃会话列表</p>
            <H1TokenLogout />
          </div>
          <div>
            <p className="text-sm text-muted-foreground mb-2">Token 过期弹窗（叠加在列表上）</p>
            <H1TokenLogout showExpireDialog />
          </div>
        </div>
      )}
      {tab === "h1-apikey" && (
        <div className="flex flex-col items-center gap-6">
          <div>
            <p className="text-sm text-muted-foreground mb-2">列表</p>
            <H1ApiKey />
          </div>
          <div>
            <p className="text-sm text-muted-foreground mb-2">刚创建（仅一次性展示密钥）</p>
            <H1ApiKey showCreated />
          </div>
        </div>
      )}
      {tab === "h1-role" && (
        <div className="flex justify-center">
          <H1RolePermission />
        </div>
      )}
      {tab === "h2-audit" && (
        <div className="flex justify-center">
          <H2AuditQuery />
        </div>
      )}
      {tab === "h2-archive" && (
        <div className="flex justify-center">
          <H2Archive />
        </div>
      )}
      {tab === "h3-swagger" && (
        <div className="flex justify-center">
          <H3SwaggerUi />
        </div>
      )}
      {tab === "m2-dual-sign" && (
        <div className="flex justify-center">
          <M2DualSign />
        </div>
      )}
      {tab === "m2-kanban" && (
        <div className="flex justify-center">
          <M2InboundKanban />
        </div>
      )}
      {tab === "m2-tasks" && (
        <div className="flex justify-center">
          <M2InboundTasks />
        </div>
      )}
      {tab === "m2-accept" && (
        <div className="flex justify-center">
          <M2InboundAccept />
        </div>
      )}
      {tab === "m2-putaway" && (
        <div className="flex justify-center">
          <M2Putaway />
        </div>
      )}
      {tab === "m2-reject" && (
        <div className="flex justify-center">
          <M2Reject />
        </div>
      )}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
