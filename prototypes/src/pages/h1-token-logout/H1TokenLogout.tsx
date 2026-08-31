import { useState } from "react";
import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@wms/ui";
import {
  StatusBadge,
  PageHeader,
  DataTable,
  type DataTableColumn,
} from "@wms/ui";
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
 * H1TokenLogout — 登录会话与登出
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-H1-005
 * Wave：Wave 0.5（P0）
 * 业务约束：JWT 默认 8h；Refresh Token 7 天；登出 = 加入服务端黑名单
 *
 * @example
 *   <H1TokenLogout showExpireDialog />
 */
export function H1TokenLogout({ showExpireDialog = false }: { showExpireDialog?: boolean } = {}) {
  const [sessions, setSessions] = useState(MOCK_SESSIONS);
  const [open, setOpen] = useState(showExpireDialog);

  const columns: DataTableColumn<Session>[] = [
    {
      key: "device",
      header: "设备",
      render: (s) => {
        const D = DEVICE_META[s.device];
        return <span className="inline-flex items-center gap-1.5"><D.icon className="size-3.5 text-muted-foreground" />{D.label}</span>;
      },
    },
    { key: "client", header: "客户端" },
    { key: "ip", header: "IP", mono: true },
    {
      key: "loginAt",
      header: "登录时间",
      render: (s) => <span className="inline-flex items-center gap-1.5"><Clock className="size-3 text-muted-foreground" />{s.loginAt}</span>,
    },
    { key: "expiresAt", header: "过期时间", render: (s) => <span className="text-muted-foreground">{s.expiresAt}</span> },
    {
      key: "action",
      header: "操作",
      render: (s) =>
        s.current ? (
          <StatusBadge status="in_progress" size="sm" label="当前会话" />
        ) : (
          <Button variant="outline" size="sm" onClick={(e) => { e.stopPropagation(); setSessions(sessions.filter((x) => x.id !== s.id)); }}>
            强制登出
          </Button>
        ),
    },
  ];

  return (
    <div className="w-full max-w-[1280px] min-h-[800px] bg-muted/40 border rounded-xl p-6 font-sans">
      <PageHeader title="登录会话与登出" subtitle="H1-005 / Token 黑名单 + 多设备会话管理" />

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <AlertCircle className="size-5 text-destructive" />
              登录已过期
            </DialogTitle>
            <DialogDescription>
              你的 token 已于 <span className="font-mono">2026-05-22 17:14</span> 过期。<br />
              为保障操作安全，请重新登录。已暂存 3 条未提交操作（重登后自动恢复）。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" size="sm" onClick={() => setOpen(false)}>取消（保留 5 分钟）</Button>
            <Button size="sm">重新登录</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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
          <p className="text-xs text-muted-foreground">登出 = token 加入服务端黑名单（GSP 审计）</p>
          <Button variant="destructive" size="sm">登出</Button>
        </div>
      </Card>

      <Card className="p-6">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-base font-semibold">活跃会话（{sessions.length}）</h2>
          <Button variant="outline" size="sm">登出全部其他会话</Button>
        </div>
        <DataTable columns={columns} data={sessions} rowKey={(s) => s.id} />
      </Card>
    </div>
  );
}

function Row({ label, v, mono }: { label: string; v: string; mono?: boolean }) {
  return (
    <div className="flex">
      <span className="text-muted-foreground w-24 shrink-0">{label}</span>
      <span className={mono ? "font-mono text-xs" : ""}>{v}</span>
    </div>
  );
}
