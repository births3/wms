import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import { Input } from "@wms/ui";
import { StatusBadge, PageHeader } from "@wms/ui";
import { ChevronRight, FileJson, Search, Lock, Globe } from "lucide-react";

interface ApiEndpoint {
  method: "GET" | "POST" | "PUT" | "DELETE";
  path: string;
  summary: string;
  module: string;
  auth: "jwt" | "apikey" | "public";
  deprecated?: boolean;
}

const MOCK_ENDPOINTS: Record<string, ApiEndpoint[]> = {
  "M2 入库": [
    { method: "POST", path: "/api/v1/inbound/asn", summary: "创建 ASN（采购入库通知）", module: "M2", auth: "jwt" },
    { method: "GET", path: "/api/v1/inbound/asn/{id}", summary: "查询 ASN 详情", module: "M2", auth: "jwt" },
    { method: "POST", path: "/api/v1/inbound/asn/{id}/receive", summary: "收货签收（PDA/PC Web）", module: "M2", auth: "jwt" },
    { method: "POST", path: "/api/v1/inbound/asn/{id}/verify", summary: "验收提交（PDA/PC Web）", module: "M2", auth: "jwt" },
    { method: "POST", path: "/api/v1/inbound/asn/{id}/dual-sign", summary: "双人复核签字", module: "M2", auth: "jwt" },
  ],
  "M3 库存": [
    { method: "GET", path: "/api/v1/inventory", summary: "实时库存查询", module: "M3", auth: "jwt" },
    { method: "POST", path: "/api/v1/inventory/move", summary: "库内移库", module: "M3", auth: "jwt" },
    { method: "POST", path: "/api/v1/inventory/stocktake", summary: "盘点单创建", module: "M3", auth: "jwt" },
    { method: "PUT", path: "/api/v1/inventory/status", summary: "库存状态变更（隔离/解隔离）", module: "M3", auth: "jwt" },
  ],
  "H2 审计": [
    { method: "GET", path: "/api/v1/audit/events", summary: "审计追踪查询（多维度）", module: "H2", auth: "jwt" },
    { method: "GET", path: "/api/v1/audit/events/{id}", summary: "审计事件详情（含 diff）", module: "H2", auth: "jwt" },
  ],
  "M-TC 追溯码（部分公开）": [
    { method: "GET", path: "/api/v1/trace/events", summary: "追溯码事件查询（供 ERP 上报使用）", module: "M-TC", auth: "apikey" },
    { method: "POST", path: "/api/v1/trace/callback", summary: "回调 webhook（已迁 v2）", module: "M-TC", auth: "apikey", deprecated: true },
    { method: "GET", path: "/health/trace", summary: "追溯码服务健康检查", module: "M-TC", auth: "public" },
  ],
};

const METHOD_COLOR: Record<ApiEndpoint["method"], string> = {
  GET: "bg-wms-success/15 text-wms-success border-wms-success/30",
  POST: "bg-primary/15 text-primary border-primary/30",
  PUT: "bg-wms-warning/15 text-wms-warning border-wms-warning/30",
  DELETE: "bg-destructive/15 text-destructive border-destructive/30",
};

const AUTH_META = {
  jwt: { icon: Lock, label: "JWT", color: "text-primary" },
  apikey: { icon: Lock, label: "API Key", color: "text-wms-warning" },
  public: { icon: Globe, label: "公开", color: "text-muted-foreground" },
};

/**
 * H3SwaggerUi — API 文档可访问性页面（Swagger UI 定制）
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-H3-004（OpenAPI 文档展示 + 鉴权类型 + 弃用标记）
 * Wave：Wave 0.5（P0 必交付）
 * 业务约束：弃用接口必须显式标记；公开接口与鉴权接口区分
 *
 * @example
 *   <H3SwaggerUi />
 */
export function H3SwaggerUi() {
  const total = Object.values(MOCK_ENDPOINTS).flat().length;
  const deprecated = Object.values(MOCK_ENDPOINTS).flat().filter((e) => e.deprecated).length;

  return (
    <div className="w-full max-w-[1280px] min-h-[800px] bg-muted/40 border rounded-xl p-6 font-sans">
      <PageHeader
        title="WMS API 文档"
        subtitle={`H3-004 · OpenAPI 3.1 · v1.0.0 · 共 ${total} 个端点 · ${deprecated} 个已弃用`}
        actions={
          <>
            <Button variant="outline" size="sm"><FileJson data-icon="inline-start" />下载 openapi.json</Button>
            <Button variant="outline" size="sm">Postman 导出</Button>
            <Button size="sm">尝试调用</Button>
          </>
        }
      />

      <Card className="p-4 mb-4">
        <div className="flex gap-3 items-center">
          <div className="relative flex-1">
            <Search className="size-4 absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" />
            <Input className="pl-9" placeholder="搜索路径 / 方法 / 模块（如 inbound、M2、POST）..." />
          </div>
          <Button variant="outline" size="sm">服务器：dev (https://api-dev.wms.local)</Button>
        </div>
      </Card>

      <div className="grid grid-cols-[280px_1fr] gap-4">
        {/* 左侧分组导航 */}
        <Card className="p-0 overflow-hidden self-start">
          <div className="px-4 py-2 text-xs text-muted-foreground bg-muted/40 border-b">分组</div>
          <ul className="py-1">
            {Object.entries(MOCK_ENDPOINTS).map(([group, items]) => (
              <li
                key={group}
                className="flex items-center justify-between px-4 py-2 text-sm hover:bg-accent/40 cursor-pointer border-l-2 border-transparent hover:border-primary"
              >
                <span>{group}</span>
                <span className="text-xs text-muted-foreground">{items.length}</span>
              </li>
            ))}
            <li className="flex items-center justify-between px-4 py-2 text-sm bg-primary/10 cursor-pointer border-l-2 border-primary text-primary font-medium">
              <span>所有端点</span>
              <ChevronRight className="size-3.5" />
            </li>
          </ul>
        </Card>

        {/* 右侧端点列表 */}
        <div className="flex flex-col gap-4">
          {Object.entries(MOCK_ENDPOINTS).map(([group, items]) => (
            <Card key={group} className="p-0 overflow-hidden">
              <div className="px-4 py-3 bg-muted/40 border-b flex items-center justify-between">
                <h3 className="text-sm font-semibold">{group}</h3>
                <span className="text-xs text-muted-foreground">{items.length} 个端点</span>
              </div>
              <ul className="divide-y">
                {items.map((e, i) => {
                  const A = AUTH_META[e.auth];
                  return (
                    <li key={i} className={`flex items-center gap-3 px-4 py-2.5 hover:bg-accent/40 cursor-pointer ${e.deprecated ? "opacity-60" : ""}`}>
                      <span className={`px-2 py-0.5 rounded text-[11px] font-mono font-semibold border ${METHOD_COLOR[e.method]}`}>
                        {e.method}
                      </span>
                      <code className={`flex-1 font-mono text-xs ${e.deprecated ? "line-through" : ""}`}>{e.path}</code>
                      <span className="text-sm text-foreground/80 flex-1">{e.summary}</span>
                      <span className={`inline-flex items-center gap-1 text-xs ${A.color}`}>
                        <A.icon className="size-3" />
                        {A.label}
                      </span>
                      {e.deprecated && <StatusBadge status="expired" size="sm" label="弃用" />}
                    </li>
                  );
                })}
              </ul>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
