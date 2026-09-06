import { useState } from "react";
import {
  Button,
  Input,
  Label,
  Tabs,
  TabsList,
  TabsTrigger,
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@wms/ui";
import { OfflineIndicator } from "@wms/ui";
import { ScanInput } from "@wms/ui";
import { StatusBadge } from "@wms/ui";
import { Snowflake } from "lucide-react";

type LoginMode = "badge" | "password";

export interface H1LoginPdaProps {
  offlineMode?: boolean;
  errorState?: boolean;
}

/**
 * H1LoginPda — PDA 端登录页
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-H1-001（PDA 主用工牌扫码 + 备用账号密码）
 * Wave：Wave 0.5（P0 必交付）
 * 业务约束：触控基线 ≥ 48pt；离线 token 缓存（H1 §7）；密码 5 次失败锁定
 *
 * 视觉方向：延续 PC 端冷链工业风，适配触控
 * - 深色顶部 header（与 PC 左面板同色系）
 * - 冰晶纹理微装饰
 * - 大触控区域（≥48pt）
 * - 简洁卡片式表单
 *
 * @example
 *   <H1LoginPda />
 *   <H1LoginPda offlineMode errorState />
 */
export function H1LoginPda({ offlineMode, errorState }: H1LoginPdaProps = {}) {
  const [mode, setMode] = useState<LoginMode>("badge");
  const [account, setAccount] = useState("");
  const [password, setPassword] = useState("");
  const [tenant, setTenant] = useState("tianhe");
  const [scanned, setScanned] = useState<string>();

  return (
    <div
      data-device="pda"
      className="w-[480px] min-h-[800px] flex flex-col rounded-xl border overflow-hidden shadow-md font-sans bg-white"
    >
      <OfflineIndicator
        state={offlineMode ? "offline" : "online"}
        pendingCount={offlineMode ? 0 : undefined}
      />

      {/* 深色 Header */}
      <div className="relative overflow-hidden px-6 pt-10 pb-8" style={{ background: "hsl(220, 30%, 8%)" }}>
        {/* 冰晶纹理 */}
        <svg className="absolute inset-0 w-full h-full opacity-[0.03]" xmlns="http://www.w3.org/2000/svg">
          <defs>
            <pattern id="ice-pda" x="0" y="0" width="40" height="40" patternUnits="userSpaceOnUse">
              <path d="M20 0v40M0 20h40M20 20l14-14M20 20l-14 14M20 20l14 14M20 20l-14-14" stroke="currentColor" strokeWidth="0.5" fill="none" className="text-sky-300" />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill="url(#ice-pda)" />
        </svg>

        {/* 光晕 */}
        <div className="absolute -top-10 -right-10 size-40 rounded-full opacity-[0.08]" style={{ background: "radial-gradient(circle, hsl(189, 94%, 43%) 0%, transparent 70%)" }} />

        <div className="relative z-10 flex flex-col items-center text-center">
          <div className="size-14 rounded-xl flex items-center justify-center border border-sky-400/30 mb-4" style={{ background: "linear-gradient(135deg, hsl(189, 94%, 43%, 0.15), hsl(217, 84%, 53%, 0.15))" }}>
            <Snowflake className="size-6 text-sky-400" />
          </div>
          <h1 className="text-xl font-bold text-white tracking-tight">医药冷链 WMS</h1>
          <p className="mt-1 text-xs text-white/40 tracking-widest uppercase">PDA Terminal · v1.0</p>
        </div>
      </div>

      {/* 模式切换 */}
      <div className="px-5 -mt-4 relative z-10">
        <Tabs value={mode} onValueChange={(v) => setMode(v as LoginMode)}>
          <TabsList className="grid grid-cols-2 w-full h-12 shadow-sm">
            <TabsTrigger value="badge" className="text-base">工牌扫码</TabsTrigger>
            <TabsTrigger value="password" className="text-base">账号密码</TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      {/* 表单区 */}
      <div className="flex-1 px-5 pt-6 pb-4 flex flex-col gap-5">
        {mode === "badge" ? (
          <div className="flex flex-col gap-2">
            <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">扫工牌登录</Label>
            <ScanInput
              mode="scanner"
              placeholder="请扫描工牌条码"
              lastScanned={scanned}
              onScan={setScanned}
              error={errorState ? "工牌已停用，请联系仓库主管" : undefined}
            />
            {!errorState && (
              <p className="text-xs text-muted-foreground/70 leading-relaxed mt-1">
                扫码 = 工号 + 设备指纹，无需输入密码。
              </p>
            )}
          </div>
        ) : (
          <>
            <div className="flex flex-col gap-2">
              <Label htmlFor="acc" className="text-xs font-medium uppercase tracking-wider text-muted-foreground">工号 / 手机号</Label>
              <Input id="acc" className="h-13 text-base" value={account} onChange={(e) => setAccount(e.target.value)} placeholder="请输入工号" />
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="pwd" className="text-xs font-medium uppercase tracking-wider text-muted-foreground">密码</Label>
              <Input id="pwd" type="password" className={`h-13 text-base ${errorState ? "border-destructive" : ""}`} value={password} onChange={(e) => setPassword(e.target.value)} placeholder="请输入密码" />
              {errorState && <p className="text-xs text-destructive mt-1">工号或密码错误（剩余 4 次锁定）</p>}
            </div>
          </>
        )}

        <div className="flex flex-col gap-2">
          <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">当前货主</Label>
          <Select value={tenant} onValueChange={setTenant}>
            <SelectTrigger className="h-13 text-base">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="tianhe">天河仓</SelectItem>
              <SelectItem value="baiyun">白云仓</SelectItem>
              <SelectItem value="3pl-a">三方代运营 A</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <Button size="lg" className="w-full h-14 text-base font-medium mt-2">
          {mode === "badge" ? "扫码登录" : "登录"}
        </Button>

        <div className="flex justify-between">
          <Button variant="link" className="px-0 text-xs">忘记密码</Button>
          <Button variant="link" className="px-0 text-xs">切换设备</Button>
        </div>

        {/* 最近登录 */}
        <div className="mt-auto pt-4 border-t">
          <p className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground mb-3">最近登录</p>
          <div className="flex flex-col gap-2">
            <RecentRow user="张三 (u001)" time="今日 09:14" status="completed" />
            <RecentRow user="李四 (u002)" time="昨日 17:30" status="completed" />
            <RecentRow user="王五 (u003)" time="昨日 14:08" status="unqualified" note="密码错误" />
          </div>
        </div>
      </div>

      {/* 底部 */}
      <div className="px-5 py-3 text-center text-[11px] text-muted-foreground/50 border-t">
        © 2026 · GSP 2024 Compliant
      </div>
    </div>
  );
}

function RecentRow({ user, time, status, note }: { user: string; time: string; status: "completed" | "unqualified"; note?: string }) {
  return (
    <div className="flex justify-between items-center px-3 py-2.5 bg-muted/50 rounded-lg">
      <div>
        <p className="text-sm font-medium">{user}</p>
        <p className="text-[11px] text-muted-foreground">{time}{note && ` · ${note}`}</p>
      </div>
      <StatusBadge status={status} size="sm" label={status === "completed" ? "成功" : "失败"} />
    </div>
  );
}
