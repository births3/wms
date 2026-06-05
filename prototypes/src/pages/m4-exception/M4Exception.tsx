import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import { ChevronLeft, AlertTriangle, FileX, Package, Shuffle } from "lucide-react";
import {
  StatusBadge,
  OfflineIndicator,
  FieldTable,
  type FieldRow,
} from "@wms/ui";

/**
 * M4Exception — M4-006 异常拣货
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M4-006（PDA 异常处理 / 短拣 / 错位 / 损坏 / 替换批次）
 * Wave：Wave 2.5（M4 出库 PDA 业务流）
 * 业务约束：必须填明确处理选项；替换批次需重新走 FIFO 校验；通知客户/调度
 *
 * @example
 *   <M4Exception />
 */

const FIELDS: FieldRow[] = [
  { label: "SO", value: "SO-2026-0042", autoFilled: true },
  { label: "异常行", value: "第 3 行 · 重组人胰岛素", autoFilled: true },
  { label: "应拣数量", value: "2 瓶（冷藏）", autoFilled: true },
  { label: "实际可拣", value: "0 瓶", required: true, error: "推荐库位无货" },
  { label: "短缺数量", value: "2 瓶", required: true, autoFilled: true },
];

const RESOLUTIONS = [
  {
    code: "REPLACE_BATCH",
    title: "替换批次",
    desc: "用同品 20260315B 批次（库存 5 瓶，效期 2027-03-15）替换",
    icon: Shuffle,
    impact: "保持订单数量；FIFO 偏离 1 个月（合规）",
    selected: true,
    color: "primary",
  },
  {
    code: "PARTIAL_SHIP",
    title: "短拣发货",
    desc: "本行实拣 0 瓶，剩余 10 行正常发货，本行下单后补",
    icon: Package,
    impact: "客户欠收 2 瓶，触发补单流程",
    color: "muted",
  },
  {
    code: "CANCEL_LINE",
    title: "取消本行",
    desc: "整行取消（已拣回退入库），其他行继续",
    icon: FileX,
    impact: "金额减 ¥972；客户需确认",
    color: "muted",
  },
  {
    code: "CANCEL_ORDER",
    title: "取消整单",
    desc: "整张 SO 取消（已拣全部回退入库）",
    icon: FileX,
    impact: "金额减 ¥2076；客户需确认；需主管审批",
    color: "destructive",
  },
];

export function M4Exception() {
  return (
    <div data-device="pda" className="w-[480px] min-h-[900px] flex flex-col rounded-xl border overflow-hidden shadow-md font-sans bg-muted/30">
      <OfflineIndicator state="online" />

      {/* 顶栏 */}
      <div className="bg-background px-3 py-2 border-b flex items-center gap-2">
        <Button variant="ghost" size="icon" className="size-8">
          <ChevronLeft className="size-5" />
        </Button>
        <div className="flex-1">
          <div className="text-xs text-muted-foreground">M4-006 异常处理</div>
          <div className="text-sm font-semibold">SO-2026-0042</div>
        </div>
        <StatusBadge status="unqualified" size="sm" label="待处理" />
      </div>

      {/* 异常说明 */}
      <div className="bg-destructive/10 px-3 py-3 border-b border-destructive/30 flex items-start gap-2">
        <AlertTriangle className="size-4 text-destructive flex-shrink-0 mt-0.5" />
        <div className="text-xs flex-1">
          <div className="font-semibold text-destructive">推荐库位 C-01-01-08 无货</div>
          <div className="text-muted-foreground mt-0.5">
            扫货时发现批号 20260315B 库存为 0（系统记录 2 瓶）·{" "}
            <span className="text-destructive">疑似系统失同步或前序失误</span>
          </div>
        </div>
      </div>

      {/* 字段 */}
      <div className="bg-background px-3 py-3 border-b">
        <FieldTable size="sm" rows={FIELDS} />
      </div>

      {/* 处理选项 */}
      <div className="flex-1 overflow-y-auto bg-muted/30 px-3 py-3">
        <div className="text-sm font-semibold mb-2">处理方式 <span className="text-destructive">*</span></div>
        <div className="flex flex-col gap-2">
          {RESOLUTIONS.map((r) => {
            const Icon = r.icon;
            const isPrimary = r.color === "primary";
            const isDestructive = r.color === "destructive";
            const isSelected = r.selected;
            return (
              <Card
                key={r.code}
                className={`p-3 cursor-pointer ${
                  isSelected
                    ? "border-primary border-2 bg-primary/5"
                    : "hover:bg-background/80"
                }`}
              >
                <div className="flex items-start gap-2">
                  <input
                    type="radio"
                    name="resolution"
                    defaultChecked={isSelected}
                    className={`mt-1 ${isDestructive ? "accent-destructive" : "accent-primary"}`}
                  />
                  <div className="flex-1">
                    <div className="flex items-center gap-1.5 mb-1">
                      <Icon className={`size-4 ${
                        isPrimary ? "text-primary" :
                        isDestructive ? "text-destructive" :
                        "text-muted-foreground"
                      }`} />
                      <span className="text-sm font-medium">{r.title}</span>
                      <span className="font-mono text-[10px] text-muted-foreground">{r.code}</span>
                      {isPrimary && (
                        <span className="text-[10px] px-1.5 py-0.5 bg-primary/10 text-primary rounded font-medium">
                          推荐
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-muted-foreground mb-1.5">{r.desc}</div>
                    <div className={`text-[11px] ${isDestructive ? "text-destructive" : "text-muted-foreground"}`}>
                      <span className="font-medium">影响：</span>{r.impact}
                    </div>
                  </div>
                </div>
              </Card>
            );
          })}
        </div>

        {/* 备注 */}
        <div className="mt-3">
          <div className="text-sm font-semibold mb-2">备注</div>
          <textarea
            placeholder="补充说明……"
            rows={2}
            className="w-full px-3 py-2 text-sm rounded-md border bg-background resize-none"
            defaultValue="经盘点确认 C-01-01-08 实际无货；用 A-02-01-04 同品批替换"
          />
        </div>
      </div>

      {/* 底部 */}
      <div className="bg-background px-3 py-2 border-t flex gap-2">
        <Button variant="outline" className="flex-1 h-10">取消</Button>
        <Button className="flex-[2] h-10">提交并继续拣货</Button>
      </div>
    </div>
  );
}
