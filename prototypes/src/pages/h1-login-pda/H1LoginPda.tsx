import { useState } from "react";
import {
  Button,
  Input,
  Label,
  Card,
  Tabs,
  TabsList,
  TabsTrigger,
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui";
import { OfflineIndicator } from "@/components/business";
import { ScanInput } from "@/components/business";
import { StatusBadge } from "@/components/business";

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
      className="w-[480px] min-h-[800px] bg-muted/40 flex flex-col rounded-xl border overflow-hidden shadow-md font-sans"
    >
      <OfflineIndicator
        state={offlineMode ? "offline" : "online"}
        pendingCount={offlineMode ? 0 : undefined}
      />

      {/* Header */}
      <div className="px-6 pt-8 pb-4 text-center">
        <div className="w-[72px] h-[72px] bg-primary rounded-xl text-primary-foreground text-3xl font-bold inline-flex items-center justify-center mb-3">
          WMS
        </div>
        <h1 className="text-[22px] font-semibold">医药冷链 WMS</h1>
        <p className="text-sm text-muted-foreground">PDA 端 · v1.0</p>
      </div>

      {/* Mode tabs */}
      <div className="px-4 pb-4">
        <Tabs value={mode} onValueChange={(v) => setMode(v as LoginMode)}>
          <TabsList className="grid grid-cols-2 w-full h-12">
            <TabsTrigger value="badge" className="text-base">工牌扫码</TabsTrigger>
            <TabsTrigger value="password" className="text-base">账号密码</TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      {/* Form card */}
      <Card className="mx-4 p-5 space-y-4">
        {mode === "badge" ? (
          <div className="space-y-1.5">
            <Label>扫工牌登录</Label>
            <ScanInput
              mode="scanner"
              placeholder="请扫描工牌条码"
              lastScanned={scanned}
              onScan={setScanned}
              error={errorState ? "工牌已停用，请联系仓库主管" : undefined}
            />
            <p className="text-xs text-muted-foreground leading-relaxed">
              扫码 = 工号 + 设备指纹，无需输入密码。<br />
              工牌丢失？请切换"账号密码"模式。
            </p>
          </div>
        ) : (
          <>
            <div className="space-y-1.5">
              <Label htmlFor="acc">工号 / 手机号</Label>
              <Input id="acc" className="h-12 text-base" value={account} onChange={(e) => setAccount(e.target.value)} placeholder="请输入工号" />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="pwd">密码</Label>
              <Input id="pwd" type="password" className={`h-12 text-base ${errorState ? "border-destructive" : ""}`} value={password} onChange={(e) => setPassword(e.target.value)} placeholder="请输入密码" />
              {errorState && <p className="text-xs text-destructive">工号或密码错误（剩余 4 次锁定）</p>}
            </div>
          </>
        )}

        <div className="space-y-1.5">
          <Label>当前货主</Label>
          <Select value={tenant} onValueChange={setTenant}>
            <SelectTrigger className="h-12 text-base">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="tianhe">天河仓</SelectItem>
              <SelectItem value="baiyun">白云仓</SelectItem>
              <SelectItem value="3pl-a">三方代运营 A</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <Button size="lg" className="w-full h-14 text-base">
          {mode === "badge" ? "扫码登录" : "登录"}
        </Button>

        <div className="flex justify-between">
          <Button variant="link" className="px-0">忘记密码</Button>
          <Button variant="link" className="px-0">切换设备</Button>
        </div>
      </Card>

      {/* Recent logins */}
      <Card className="m-4 p-5">
        <p className="text-sm font-medium mb-3">最近登录</p>
        <div className="space-y-2">
          <RecentLoginRow user="张三 (u001)" time="今日 09:14" status="completed" />
          <RecentLoginRow user="李四 (u002)" time="昨日 17:30" status="completed" />
          <RecentLoginRow user="王五 (u003)" time="昨日 14:08" status="unqualified" note="密码错误" />
        </div>
      </Card>

      <div className="flex-1" />
      <p className="px-4 py-3 text-center text-xs text-muted-foreground/80">© 2026 医药冷链 WMS · GSP 合规</p>
    </div>
  );
}

function RecentLoginRow({
  user,
  time,
  status,
  note,
}: {
  user: string;
  time: string;
  status: "completed" | "unqualified";
  note?: string;
}) {
  return (
    <div className="flex justify-between items-center px-3 py-2 bg-muted rounded-md">
      <div>
        <p className="text-[15px] font-medium">{user}</p>
        <p className="text-xs text-muted-foreground">
          {time}
          {note && ` · ${note}`}
        </p>
      </div>
      <StatusBadge status={status} size="sm" label={status === "completed" ? "成功" : "失败"} />
    </div>
  );
}
