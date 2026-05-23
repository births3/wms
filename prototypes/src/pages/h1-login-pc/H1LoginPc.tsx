import { useState } from "react";
import { Button, Input, Label, Card, Checkbox, Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from "@wms/ui";
import { StatusBadge } from "@wms/ui";
import { CheckCircle2 } from "lucide-react";

/**
 * H1LoginPc — PC 端管理后台登录页
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-H1-001（PC 主用工号 + 密码）
 * Wave：Wave 0.5（P0 必交付）
 * 业务约束：5 次失败显示验证码；多货主切换；JWT 默认 8 小时
 *
 * @example
 *   <H1LoginPc />
 *   <H1LoginPc withCaptcha />
 */
export function H1LoginPc({ withCaptcha = false }: { withCaptcha?: boolean } = {}) {
  const [account, setAccount] = useState("");
  const [password, setPassword] = useState("");
  const [tenant, setTenant] = useState("tianhe");
  const [remember, setRemember] = useState(true);

  return (
    <div
      data-device="pc"
      className="w-full max-w-[1280px] min-h-[800px] flex items-center justify-center p-6 rounded-xl border overflow-hidden font-sans"
      style={{
        background: "linear-gradient(135deg, hsl(217,84%,53%) 0%, hsl(189,94%,43%) 100%)",
      }}
    >
      <Card className="w-[1000px] min-h-[580px] grid grid-cols-2 overflow-hidden p-0 shadow-2xl">
        {/* 左侧品牌 */}
        <div
          className="text-white p-12 flex flex-col justify-between"
          style={{
            background: "linear-gradient(180deg, hsl(217,84%,53%) 0%, hsl(189,94%,43%) 100%)",
          }}
        >
          <div>
            <div className="w-14 h-14 bg-white/20 rounded-md inline-flex items-center justify-center text-2xl font-bold">
              WMS
            </div>
            <h1 className="mt-6 text-3xl leading-tight font-bold">
              医药冷链 WMS<br />
              <span className="font-normal opacity-90 text-lg">GSP 合规 · 三端协同 · 全流程治理</span>
            </h1>
          </div>
          <ul className="space-y-2 text-sm opacity-95">
            <li className="flex items-center gap-2"><CheckCircle2 className="h-4 w-4" /> 多货主隔离 · 3PL / 连锁 / 自营三态支持</li>
            <li className="flex items-center gap-2"><CheckCircle2 className="h-4 w-4" /> 冷链温控接入 · 自动批次隔离</li>
            <li className="flex items-center gap-2"><CheckCircle2 className="h-4 w-4" /> append-only 审计 · 满足 GSP 法定台账</li>
            <li className="flex items-center gap-2"><CheckCircle2 className="h-4 w-4" /> 批次 / 效期 FIFO · 双人验收双签</li>
          </ul>
          <div className="text-xs opacity-70">© 2026 WMS · v1.0.0 · ADR-0021</div>
        </div>

        {/* 右侧表单 */}
        <div className="px-14 py-16 flex flex-col justify-center">
          <h2 className="text-2xl font-semibold mb-2">登录</h2>
          <p className="text-sm text-muted-foreground mb-1">使用工号 / 手机号登录管理端</p>
          <p className="text-xs text-muted-foreground/70 mb-7">演示账号：u001 / u002 · 仅原型环境可见</p>

          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="acc">工号 / 手机号</Label>
              <Input id="acc" value={account} onChange={(e) => setAccount(e.target.value)} placeholder="请输入工号" />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="pwd">密码</Label>
              <Input id="pwd" type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="请输入密码" />
            </div>

            {withCaptcha && (
              <div className="space-y-1.5">
                <Label htmlFor="cap">验证码</Label>
                <div className="flex gap-2">
                  <Input id="cap" placeholder="请输入图形验证码" className="flex-1 border-destructive" />
                  <div className="w-[100px] h-9 bg-muted rounded-md flex items-center justify-center text-base font-mono tracking-widest cursor-pointer border">
                    4Q3z
                  </div>
                </div>
                <p className="text-xs text-destructive">⚠ 已连续失败 3 次，再失败 2 次将锁定 15 分钟</p>
              </div>
            )}

            <div className="space-y-1.5">
              <Label>当前货主</Label>
              <Select value={tenant} onValueChange={setTenant}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="tianhe">天河仓</SelectItem>
                  <SelectItem value="baiyun">白云仓</SelectItem>
                  <SelectItem value="3pl-a">三方代运营 A</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="flex justify-between items-center">
              <label className="flex items-center gap-2 text-sm cursor-pointer">
                <Checkbox checked={remember} onCheckedChange={(v) => setRemember(v === true)} />
                记住工号
              </label>
              <Button variant="link" size="sm" className="px-0">忘记密码？</Button>
            </div>

            <Button className="w-full">登录</Button>

            <div className="pt-4 border-t flex justify-between items-center text-xs text-muted-foreground">
              <span>系统状态</span>
              <span className="flex gap-2">
                <StatusBadge status="completed" size="sm" label="API 正常" />
                <StatusBadge status="completed" size="sm" label="DB 正常" />
              </span>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}
