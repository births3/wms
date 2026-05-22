import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import "./globals.css";
import { Button } from "./components/ui";
import { H1LoginPda } from "./pages/h1-login-pda";
import { H1LoginPc } from "./pages/h1-login-pc";
import { H2AuditQuery } from "./pages/h2-audit-query";
import { ComponentsGallery } from "./pages/components-gallery";

type Tab = "gallery" | "pda-login" | "pc-login" | "audit-query";

const TABS: { value: Tab; label: string }[] = [
  { value: "gallery", label: "组件库" },
  { value: "pda-login", label: "PDA 登录" },
  { value: "pc-login", label: "PC 登录" },
  { value: "audit-query", label: "审计查询" },
];

function App() {
  const initial = (window.location.hash.replace("#", "") as Tab) || "gallery";
  const [tab, setTab] = useState<Tab>(
    TABS.some((t) => t.value === initial) ? initial : "gallery"
  );

  return (
    <div className="min-h-screen bg-muted/40 p-6 font-sans">
      <div className="flex items-center gap-4 mb-6 max-w-[1400px] mx-auto">
        <h1 className="text-xl font-semibold">WMS 原型预览</h1>
        <span className="text-xs text-muted-foreground">ADR-0021 / ADR-0022</span>
        <div className="ml-auto flex gap-2 flex-wrap">
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
      {tab === "pda-login" && (
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
      {tab === "pc-login" && (
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
      {tab === "audit-query" && (
        <div className="flex justify-center">
          <H2AuditQuery />
        </div>
      )}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
