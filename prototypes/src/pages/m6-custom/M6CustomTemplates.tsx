import { Button, Card } from "@wms/ui";
import { Star, Trash2 } from "lucide-react";
import { SAVED_TEMPLATES } from "./m6-custom-data";

/**
 * M6CustomTemplates — 自定义业务报表模板列表
 *
 * 层级：Layer 3 页面级子组件
 * 关联故事：US-M6-003（保存模板 / 权限范围 / 报表订阅）
 * Wave：Wave 0.5（演示原型）→ Wave 5（Metabase 嵌入正式上线）
 *
 * @example
 *   <M6CustomTemplates />
 */
export function M6CustomTemplates() {
  return (
    <div>
      <div className="flex items-center justify-between mb-2">
        <div className="text-sm font-semibold">已保存模板（5）</div>
        <span className="text-xs text-muted-foreground">私有 / 部门共享 / 全局 三档权限</span>
      </div>
      <div className="grid grid-cols-3 gap-2">
        {SAVED_TEMPLATES.map((template) => (
          <Card key={template.id} className="p-2.5 cursor-pointer hover:bg-muted/30">
            <div className="flex items-start justify-between mb-1">
              <div className="flex items-center gap-1.5 flex-1 min-w-0">
                {template.isFavorite && <Star className="size-3.5 text-wms-warning fill-current flex-shrink-0" />}
                <span className="text-xs font-medium truncate">{template.name}</span>
              </div>
              <Button variant="ghost" size="sm" className="size-5 p-0 flex-shrink-0">
                <Trash2 className="size-3 text-muted-foreground" />
              </Button>
            </div>
            <div className="flex items-center gap-2 text-[11px]">
              <span className={`px-1 py-0.5 rounded ${scopeClassName(template.scope)}`}>{template.scope}</span>
              <span className="text-muted-foreground truncate">{template.lastRun}</span>
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
}

function scopeClassName(scope: string) {
  if (scope === "私有") return "bg-muted text-muted-foreground";
  if (scope === "部门") return "bg-primary/10 text-primary";
  return "bg-wms-success/10 text-wms-success";
}
