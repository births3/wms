import { useState } from "react";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { StatusBadge } from "@/components/business";
import { Plus, Copy, Eye, EyeOff, Key } from "lucide-react";

interface ApiKey {
  id: string;
  name: string;
  prefix: string;
  scope: string;
  createdBy: string;
  createdAt: string;
  expiresAt: string;
  lastUsedAt: string;
  status: "active" | "revoked" | "expired";
}

const MOCK_KEYS: ApiKey[] = [
  { id: "k1", name: "ERP 集成主密钥", prefix: "wms_live_a3F2…", scope: "M1.read + M2.write + H8.acl", createdBy: "李四 (u002)", createdAt: "2026-04-12", expiresAt: "2026-10-12", lastUsedAt: "5 分钟前", status: "active" },
  { id: "k2", name: "码上放心上报", prefix: "wms_live_b9X7…", scope: "M-TC.report-only", createdBy: "张三 (u001)", createdAt: "2026-03-01", expiresAt: "2027-03-01", lastUsedAt: "2 小时前", status: "active" },
  { id: "k3", name: "TMS 调度回调", prefix: "wms_test_c1K0…", scope: "M10.callback", createdBy: "王五 (u003)", createdAt: "2026-05-01", expiresAt: "2026-06-01", lastUsedAt: "昨日 14:30", status: "active" },
  { id: "k4", name: "旧版打印服务（已弃用）", prefix: "wms_live_d4M2…", scope: "H9.print-only", createdBy: "赵六 (u004)", createdAt: "2025-09-15", expiresAt: "2026-03-15", lastUsedAt: "30 天前", status: "expired" },
  { id: "k5", name: "测试 - 张三个人", prefix: "wms_test_e7N5…", scope: "M3.read", createdBy: "张三 (u001)", createdAt: "2026-01-10", expiresAt: "—", lastUsedAt: "—", status: "revoked" },
];

const STATUS_MAP = {
  active: { status: "qualified" as const, label: "启用" },
  revoked: { status: "isolated" as const, label: "已吊销" },
  expired: { status: "expired" as const, label: "已过期" },
};

/**
 * H1ApiKey — API Key 生命周期管理
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-H1-006（创建 / 吊销 / 重置 / 审计）
 * Wave：Wave 0.5（P0 必交付）
 * 业务约束：密钥仅在创建时全文返回一次；其后只展示前缀；吊销不可恢复
 *
 * @example
 *   <H1ApiKey />
 *   <H1ApiKey showCreated />
 */
export function H1ApiKey({ showCreated = false }: { showCreated?: boolean } = {}) {
  const [revealCreated, setRevealCreated] = useState(false);

  return (
    <div className="w-[1280px] min-h-[800px] bg-muted/40 border rounded-xl p-6 font-sans">
      <header className="flex justify-between items-center mb-6">
        <div>
          <h1 className="text-xl font-semibold">API Key 管理</h1>
          <p className="text-sm text-muted-foreground mt-1">H1-006 / 外部系统对接密钥</p>
        </div>
        <Button>
          <Plus className="size-4" />
          创建 API Key
        </Button>
      </header>

      {/* 创建成功的 banner（仅一次性展示） */}
      {showCreated && (
        <Card className="p-4 mb-4 border-wms-warning bg-orange-50/50">
          <div className="flex items-start gap-3">
            <Key className="size-5 text-wms-warning shrink-0 mt-0.5" />
            <div className="flex-1">
              <p className="font-medium text-sm">新 API Key 创建成功 — 请立即复制保存</p>
              <p className="text-xs text-muted-foreground mt-1">
                密钥仅在此刻全文展示一次，关闭后无法找回；如丢失请吊销后重建。
              </p>
              <div className="mt-3 flex items-center gap-2">
                <code className="flex-1 px-3 py-2 bg-background rounded font-mono text-xs break-all">
                  {revealCreated
                    ? "wms_live_a3F2k9X7m1Z5p8Q4r6T2v0W8y3Z7"
                    : "wms_live_a3F2…••••••••••••••••"}
                </code>
                <Button variant="outline" size="sm" onClick={() => setRevealCreated(!revealCreated)}>
                  {revealCreated ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                </Button>
                <Button variant="outline" size="sm">
                  <Copy className="size-4" />
                </Button>
              </div>
            </div>
          </div>
        </Card>
      )}

      {/* 创建表单 */}
      <Card className="p-6 mb-4">
        <h2 className="text-sm font-medium mb-3">创建新密钥</h2>
        <div className="grid grid-cols-3 gap-4 items-end">
          <div className="space-y-1">
            <Label className="text-xs">名称</Label>
            <Input placeholder="如：ERP 集成主密钥" />
          </div>
          <div className="space-y-1">
            <Label className="text-xs">权限范围</Label>
            <Select defaultValue="custom">
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="readonly">只读（read.*）</SelectItem>
                <SelectItem value="erp">ERP 集成（M1+M2+H8）</SelectItem>
                <SelectItem value="trace">码上放心（M-TC.report-only）</SelectItem>
                <SelectItem value="custom">自定义...</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-1">
            <Label className="text-xs">有效期</Label>
            <Select defaultValue="180">
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="30">30 天</SelectItem>
                <SelectItem value="90">90 天</SelectItem>
                <SelectItem value="180">180 天（推荐）</SelectItem>
                <SelectItem value="365">1 年</SelectItem>
                <SelectItem value="never">永不（不推荐）</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      </Card>

      {/* 列表 */}
      <Card className="p-0 overflow-hidden">
        <div className="px-6 py-3 text-xs text-muted-foreground bg-muted/40 border-b">
          共 {MOCK_KEYS.length} 个密钥 · 启用 {MOCK_KEYS.filter((k) => k.status === "active").length} · 已过期 {MOCK_KEYS.filter((k) => k.status === "expired").length} · 已吊销 {MOCK_KEYS.filter((k) => k.status === "revoked").length}
        </div>
        <table className="w-full text-sm">
          <thead>
            <tr className="text-xs text-muted-foreground text-left bg-muted/40 border-b">
              <th className="px-4 py-2 font-medium">名称</th>
              <th className="px-4 py-2 font-medium">前缀</th>
              <th className="px-4 py-2 font-medium">权限范围</th>
              <th className="px-4 py-2 font-medium">创建</th>
              <th className="px-4 py-2 font-medium">最近使用</th>
              <th className="px-4 py-2 font-medium">过期</th>
              <th className="px-4 py-2 font-medium">状态</th>
              <th className="px-4 py-2 font-medium">操作</th>
            </tr>
          </thead>
          <tbody>
            {MOCK_KEYS.map((k) => {
              const meta = STATUS_MAP[k.status];
              return (
                <tr key={k.id} className="border-b last:border-b-0 hover:bg-accent/40">
                  <td className="px-4 py-3 font-medium">{k.name}</td>
                  <td className="px-4 py-3 font-mono text-xs text-muted-foreground">{k.prefix}</td>
                  <td className="px-4 py-3 text-xs text-muted-foreground">{k.scope}</td>
                  <td className="px-4 py-3 text-xs">
                    <div>{k.createdBy}</div>
                    <div className="text-muted-foreground">{k.createdAt}</div>
                  </td>
                  <td className="px-4 py-3 text-xs text-muted-foreground">{k.lastUsedAt}</td>
                  <td className="px-4 py-3 text-xs text-muted-foreground">{k.expiresAt}</td>
                  <td className="px-4 py-3">
                    <StatusBadge status={meta.status} size="sm" label={meta.label} />
                  </td>
                  <td className="px-4 py-3">
                    {k.status === "active" && (
                      <Button variant="outline" size="sm">吊销</Button>
                    )}
                    {k.status === "expired" && (
                      <Button variant="outline" size="sm">续期</Button>
                    )}
                    {k.status === "revoked" && (
                      <span className="text-xs text-muted-foreground">—</span>
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
