import { useState } from "react";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { StatusBadge } from "@/components/business";
import { AlertCircle, Clock, Smartphone, Monitor, ScanLine } from "lucide-react";

interface Session {
  id: string;
  device: "pc" | "pda" | "h5";
  client: string;
  ip: string;
  loginAt: string;
  expiresAt: string;
  current?: boolean;
}

const MOCK_SESSIONS: Session[] = [
  { id: "s001", device: "pc", client: "Chrome 124 / macOS", ip: "10.2.0.18", loginAt: "今日 09:14", expiresAt: "今日 17:14", current: true },
  { id: "s002", device: "pda", client: "PDA-A1B2C3 / RN", ip: "10.2.10.42", loginAt: "今日 08:30", expiresAt: "今日 16:30" },
  { id: "s003", device: "pc", client: "Safari 17 / iPad", ip: "10.2.0.55", loginAt: "昨日 14:08", expiresAt: "昨日 22:08" },
  { id: "s004", device: "h5", client: "微信内置浏览器", ip: "183.21.5.88", loginAt: "前日 09:20", expiresAt: "前日 17:20" },
];

const DEVICE_META = {
  pc: { icon: Monitor, label: "PC" },
  pda: { icon: ScanLine, label: "PDA" },
  h5: { icon: Smartphone, label: "H5" },
};

/**
 * H1TokenLogout — Token 失效与登出
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-H1-005（token 过期 / 主动登出 / 历史会话清理）
 * Wave：Wave 0.5（P0 必交付）
 * 业务约束：JWT 默认 8 小时；Refresh Token 7 天；登出 = 加入服务端黑名单
 *
 * @example
 *   <H1TokenLogout />
 *   <H1TokenLogout showExpireDialog />
 */
export function H1TokenLogout({ showExpireDialog = false }: { showExpireDialog?: boolean } = {}) {
  const [sessions, setSessions] = useState(MOCK_SESSIONS);

  const handleRevoke = (id: string) => {
    setSessions(sessions.filter((s) => s.id !== id));
  };

  return (
    <div className="w-[1280px] min-h-[800px] bg-muted/40 border rounded-xl p-6 font-sans">
      <header className="mb-6">
        <h1 className="text-xl font-semibold">登录会话与登出</h1>
        <p className="text-sm text-muted-foreground mt-1">H1-005 / Token 黑名单 + 多设备会话管理</p>
      </header>

      {/* Token 过期弹窗（条件渲染） */}
      {showExpireDialog && (
        <div className="fixed inset-0 bg-foreground/30 flex items-center justify-center z-50">
          <Card className="w-[420px] p-6 space-y-4 shadow-xl">
            <div className="flex items-center gap-3">
              <AlertCircle className="size-6 text-destructive" />
              <h3 className="text-lg font-semibold">登录已过期</h3>
            </div>
            <p className="text-sm text-muted-foreground">
              你的 token 已于 <span className="font-mono">2026-05-22 17:14</span> 过期。
              <br />
              为保障操作安全，请重新登录。已暂存 3 条未提交操作（重登后自动恢复）。
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" size="sm">取消（保留页面 5 分钟）</Button>
              <Button size="sm">重新登录</Button>
            </div>
          </Card>
        </div>
      )}

      {/* 当前会话信息 */}
      <Card className="p-6 mb-4">
        <h2 className="text-base font-semibold mb-4">当前会话</h2>
        <div className="grid grid-cols-2 gap-x-12 gap-y-3 text-sm">
          <Row label="用户" v="张三 (u001)" />
          <Row label="登录方式" v="工号 + 密码" />
          <Row label="货主" v="天河仓 (3 个可切换)" />
          <Row label="登录时间" v="2026-05-22 09:14:23" />
          <Row label="IP" v="10.2.0.18" mono />
          <Row label="设备指纹" v="DEV-X9F2A-mac" mono />
          <Row label="Token 过期" v="2026-05-22 17:14（剩 7h 56m）" />
          <Row label="Refresh Token" v="2026-05-29 09:14（剩 6 天 23h）" />
        </div>
        <div className="mt-6 pt-4 border-t flex justify-between items-center">
          <p className="text-xs text-muted-foreground">登出 = token 加入服务端黑名单（GSP 审计：actor + 时间 + IP）</p>
          <Button variant="destructive" size="sm">登出</Button>
        </div>
      </Card>

      {/* 历史会话列表 */}
      <Card className="p-6">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-base font-semibold">活跃会话（{sessions.length}）</h2>
          <Button variant="outline" size="sm">登出全部其他会话</Button>
        </div>
        <table className="w-full text-sm">
          <thead>
            <tr className="text-xs text-muted-foreground text-left bg-muted/40 border-b">
              <th className="px-4 py-2 font-medium">设备</th>
              <th className="px-4 py-2 font-medium">客户端</th>
              <th className="px-4 py-2 font-medium">IP</th>
              <th className="px-4 py-2 font-medium">登录时间</th>
              <th className="px-4 py-2 font-medium">过期时间</th>
              <th className="px-4 py-2 font-medium">操作</th>
            </tr>
          </thead>
          <tbody>
            {sessions.map((s) => {
              const D = DEVICE_META[s.device];
              return (
                <tr key={s.id} className="border-b last:border-b-0 hover:bg-accent/40">
                  <td className="px-4 py-3">
                    <span className="inline-flex items-center gap-1.5">
                      <D.icon className="size-3.5 text-muted-foreground" />
                      {D.label}
                    </span>
                  </td>
                  <td className="px-4 py-3">{s.client}</td>
                  <td className="px-4 py-3 font-mono text-xs text-muted-foreground">{s.ip}</td>
                  <td className="px-4 py-3 inline-flex items-center gap-1.5">
                    <Clock className="size-3 text-muted-foreground" />
                    {s.loginAt}
                  </td>
                  <td className="px-4 py-3 text-muted-foreground">{s.expiresAt}</td>
                  <td className="px-4 py-3">
                    {s.current ? (
                      <StatusBadge status="in_progress" size="sm" label="当前会话" />
                    ) : (
                      <Button variant="outline" size="sm" onClick={() => handleRevoke(s.id)}>
                        强制登出
                      </Button>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </Card>
    </div>
  );
}

function Row({ label, v, mono }: { label: string; v: string; mono?: boolean }) {
  return (
    <>
      <div className="flex">
        <span className="text-muted-foreground w-24 shrink-0">{label}</span>
        <span className={mono ? "font-mono text-xs" : ""}>{v}</span>
      </div>
    </>
  );
}
