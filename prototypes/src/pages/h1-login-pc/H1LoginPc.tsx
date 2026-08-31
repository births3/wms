import { useState } from "react";
import { Button, Input, Label, Checkbox, Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from "@wms/ui";
import { StatusBadge } from "@wms/ui";
import { Snowflake, Thermometer, ShieldCheck, Activity } from "lucide-react";

/**
 * H1LoginPc — PC 端管理后台登录页
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-H1-001（PC 主用工号 + 密码）
 * Wave：Wave 0.5（P0 必交付）
 * 业务约束：5 次失败显示验证码；多货主切换；JWT 默认 8 小时
 *
 * 视觉方向：精致工业风 + 冷链隐喻
 * - 深色左面板（深蓝近黑）+ 冰晶纹理 SVG 装饰
 * - 右侧纯白表单区，大量留白，精致排版
 * - 温度曲线作为装饰性背景元素
 * - 色彩：深蓝 + 冰蓝 accent + 微量暖色警示
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
      className="w-full max-w-[1280px] min-h-[800px] flex rounded-xl border overflow-hidden font-sans shadow-2xl"
    >
      {/* 左侧深色品牌面板 */}
      <div className="relative w-[480px] shrink-0 overflow-hidden" style={{ background: "hsl(220, 30%, 8%)" }}>
        {/* 冰晶纹理装饰 */}
        <svg className="absolute inset-0 w-full h-full opacity-[0.04]" xmlns="http://www.w3.org/2000/svg">
          <defs>
            <pattern id="ice" x="0" y="0" width="60" height="60" patternUnits="userSpaceOnUse">
              <path d="M30 0v60M0 30h60M30 30l21-21M30 30l-21 21M30 30l21 21M30 30l-21-21" stroke="currentColor" strokeWidth="0.5" fill="none" className="text-sky-300" />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill="url(#ice)" />
        </svg>

        {/* 温度曲线装饰 */}
        <svg className="absolute bottom-0 left-0 w-full h-48 opacity-[0.08]" viewBox="0 0 480 192" preserveAspectRatio="none">
          <path
            d="M0,120 C40,100 80,140 120,110 C160,80 200,130 240,95 C280,60 320,120 360,90 C400,60 440,100 480,80"
            stroke="hsl(189, 94%, 43%)" strokeWidth="2" fill="none"
          />
          <path
            d="M0,150 C40,140 80,160 120,145 C160,130 200,155 240,140 C280,125 320,150 360,135 C400,120 440,145 480,130"
            stroke="hsl(189, 94%, 43%)" strokeWidth="1" fill="none" opacity="0.5"
          />
        </svg>

        {/* 渐变光晕 */}
        <div className="absolute top-1/4 -left-20 size-80 rounded-full opacity-[0.06]" style={{ background: "radial-gradient(circle, hsl(189, 94%, 43%) 0%, transparent 70%)" }} />

        {/* 内容 */}
        <div className="relative z-10 h-full flex flex-col justify-between p-12">
          {/* Logo + 标题 */}
          <div>
            <div className="flex items-center gap-3 mb-12">
              <div className="size-11 rounded-lg flex items-center justify-center border border-sky-400/30" style={{ background: "linear-gradient(135deg, hsl(189, 94%, 43%, 0.2), hsl(217, 84%, 53%, 0.2))" }}>
                <Snowflake className="size-5 text-sky-400" />
              </div>
              <div>
                <p className="text-sm font-semibold text-white/90 tracking-wide">医药冷链</p>
                <p className="text-[11px] text-white/40 tracking-widest uppercase">Cold Chain WMS</p>
              </div>
            </div>

            <h1 className="text-[28px] leading-[1.3] font-bold text-white tracking-tight">
              GSP 合规<br />仓储管理系统
            </h1>
            <p className="mt-3 text-sm text-white/50 leading-relaxed max-w-[280px]">
              三端协同 · 全流程治理 · append-only 审计追踪
            </p>
          </div>

          {/* 特性列表 */}
          <div className="flex flex-col gap-4">
            <FeatureRow icon={<ShieldCheck className="size-4" />} text="多货主隔离 · 3PL / 连锁 / 自营" />
            <FeatureRow icon={<Thermometer className="size-4" />} text="冷链温控 · 2~8°C 全程监测" />
            <FeatureRow icon={<Activity className="size-4" />} text="批次效期 FIFO · 双人验收双签" />
          </div>

          {/* 底部 */}
          <div className="flex items-center gap-2 text-[11px] text-white/30">
            <span>© 2026</span>
            <span className="size-1 rounded-full bg-white/20" />
            <span>v1.0.0</span>
            <span className="size-1 rounded-full bg-white/20" />
            <span>GSP 2024 Compliant</span>
          </div>
        </div>
      </div>

      {/* 右侧表单 */}
      <div className="flex-1 bg-white flex flex-col justify-center px-20 py-16">
        <div className="max-w-[380px]">
          <h2 className="text-[26px] font-bold text-foreground tracking-tight">登录</h2>
          <p className="mt-1.5 text-sm text-muted-foreground">使用工号或手机号登录管理端</p>
          <p className="mt-1 text-[11px] text-muted-foreground/60">演示账号 u001 / u002 · 仅原型环境</p>

          <div className="mt-10 flex flex-col gap-5">
            <div className="flex flex-col gap-2">
              <Label htmlFor="acc" className="text-xs font-medium uppercase tracking-wider text-muted-foreground">工号 / 手机号</Label>
              <Input id="acc" value={account} onChange={(e) => setAccount(e.target.value)} placeholder="请输入工号" className="h-11" />
            </div>

            <div className="flex flex-col gap-2">
              <Label htmlFor="pwd" className="text-xs font-medium uppercase tracking-wider text-muted-foreground">密码</Label>
              <Input id="pwd" type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="请输入密码" className="h-11" />
            </div>

            {withCaptcha && (
              <div className="flex flex-col gap-2">
                <Label htmlFor="cap" className="text-xs font-medium uppercase tracking-wider text-destructive">验证码</Label>
                <div className="flex gap-2">
                  <Input id="cap" placeholder="图形验证码" className="flex-1 h-11 border-destructive" />
                  <div className="w-[100px] h-11 bg-muted rounded-md flex items-center justify-center text-base font-mono tracking-widest cursor-pointer border">
                    4Q3z
                  </div>
                </div>
                <p className="text-xs text-destructive">⚠ 已连续失败 3 次，再失败 2 次将锁定 15 分钟</p>
              </div>
            )}

            <div className="flex flex-col gap-2">
              <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">当前货主</Label>
              <Select value={tenant} onValueChange={setTenant}>
                <SelectTrigger className="h-11">
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
              <Button variant="link" size="sm" className="px-0 text-xs">忘记密码？</Button>
            </div>

            <Button className="w-full h-11 text-sm font-medium mt-2">登录</Button>
          </div>

          {/* 系统状态 */}
          <div className="mt-8 pt-5 border-t flex justify-between items-center">
            <span className="text-[11px] text-muted-foreground/60 uppercase tracking-wider">系统状态</span>
            <span className="flex gap-2">
              <StatusBadge status="completed" size="sm" label="API" />
              <StatusBadge status="completed" size="sm" label="DB" />
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

function FeatureRow({ icon, text }: { icon: React.ReactNode; text: string }) {
  return (
    <div className="flex items-center gap-3">
      <div className="size-8 rounded-md flex items-center justify-center bg-white/[0.04] border border-white/[0.08] text-sky-400/80">
        {icon}
      </div>
      <span className="text-[13px] text-white/70">{text}</span>
    </div>
  );
}
