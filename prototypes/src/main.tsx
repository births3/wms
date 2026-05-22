import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import "./globals.css";
import { Button } from "./components/ui";
import { H1LoginPda } from "./pages/h1-login-pda";
import { H1LoginPc } from "./pages/h1-login-pc";
import { H2AuditQuery } from "./pages/h2-audit-query";

type Tab = "pda-login" | "pc-login" | "audit-query";

function App() {
  const initial = (window.location.hash.replace("#", "") as Tab) || "pc-login";
  const [tab, setTab] = useState<Tab>(
    ["pda-login", "pc-login", "audit-query"].includes(initial) ? initial : "pc-login"
  );

  return (
    <div className="min-h-screen bg-muted/40 p-6 font-sans">
      <div className="flex items-center gap-4 mb-6 max-w-[1400px] mx-auto">
        <h1 className="text-xl font-semibold">WMS P0 原型 · shadcn/ui</h1>
        <span className="text-xs text-muted-foreground">ADR-0021 Layer 1 = shadcn 验证</span>
        <div className="ml-auto flex gap-2">
          <Button variant={tab === "pda-login" ? "default" : "outline"} size="sm" onClick={() => { setTab("pda-login"); window.location.hash = "pda-login"; }}>
            PDA 登录
          </Button>
          <Button variant={tab === "pc-login" ? "default" : "outline"} size="sm" onClick={() => { setTab("pc-login"); window.location.hash = "pc-login"; }}>
            PC 登录
          </Button>
          <Button variant={tab === "audit-query" ? "default" : "outline"} size="sm" onClick={() => { setTab("audit-query"); window.location.hash = "audit-query"; }}>
            审计查询
          </Button>
        </div>
      </div>

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
