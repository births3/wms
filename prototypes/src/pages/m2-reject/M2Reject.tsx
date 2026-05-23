import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import { ChevronLeft, Camera, FileX } from "lucide-react";
import {
  StatusBadge,
  OfflineIndicator,
  DiffPanel,
  FieldTable,
  type FieldRow,
} from "@/components/business";

/**
 * M2Reject — M2-006 PDA 拒收
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M2-006（PDA 拒收 / 必须填原因+拍照 / 触发供应商质量评分）
 * Wave：Wave 1.5（M2 PDA 业务流）
 * 业务约束：拒收原因必须从字典选；至少 2 张照片；自动通知采购员 + 供应商
 *
 * @example
 *   <M2Reject />
 */

const REJECT_REASONS = [
  { code: "PKG_DAMAGE", label: "外包装破损", selected: true },
  { code: "LABEL_DAMAGE", label: "标签破损/不清" },
  { code: "QTY_SHORTAGE", label: "数量短缺" },
  { code: "EXPIRY_NEAR", label: "效期不足" },
  { code: "TEMP_EXCURSION", label: "运输温度超标" },
  { code: "QUAL_DOC_MISSING", label: "质量证明文件缺失" },
  { code: "OTHER", label: "其他" },
];

const FIELDS: FieldRow[] = [
  { label: "ASN", value: "PO-2026-0001", autoFilled: true },
  { label: "供应商", value: "国药控股北京", autoFilled: true },
  { label: "品名", value: "葡萄糖注射液 500ml", autoFilled: true },
  { label: "到货数量", value: "240 瓶", autoFilled: true },
  { label: "拒收数量", value: "240 瓶", required: true },
];

export function M2Reject() {
  const [photos] = useState<number>(2);

  return (
    <div data-device="pda" className="w-[480px] min-h-[900px] flex flex-col rounded-xl border overflow-hidden shadow-md font-sans bg-muted/30">
      <OfflineIndicator state="online" />

      {/* 顶栏 */}
      <div className="bg-background px-3 py-2 border-b flex items-center gap-2">
        <Button variant="ghost" size="icon" className="h-8 w-8">
          <ChevronLeft className="h-5 w-5" />
        </Button>
        <div className="flex-1">
          <div className="text-xs text-muted-foreground">M2-006 拒收登记</div>
          <div className="text-sm font-semibold">PO-2026-0001</div>
        </div>
        <StatusBadge status="unqualified" size="sm" label="拟拒收" />
      </div>

      {/* 字段 */}
      <div className="bg-background px-3 py-3 border-b">
        <FieldTable size="sm" rows={FIELDS} />
      </div>

      {/* 拒收原因 */}
      <div className="bg-background px-3 py-3 border-b">
        <div className="text-sm font-semibold mb-2">拒收原因 <span className="text-destructive">*</span></div>
        <div className="space-y-1.5">
          {REJECT_REASONS.map((r) => (
            <label
              key={r.code}
              className={`flex items-center gap-2 px-3 py-2 rounded-md border text-sm cursor-pointer ${
                r.selected ? "border-destructive bg-destructive/5" : "border-input"
              }`}
            >
              <input
                type="radio"
                name="reject-reason"
                defaultChecked={r.selected}
                className="accent-destructive"
              />
              <span className={r.selected ? "font-medium" : ""}>{r.label}</span>
              <span className="ml-auto text-[11px] text-muted-foreground font-mono">{r.code}</span>
            </label>
          ))}
        </div>
      </div>

      {/* 备注 */}
      <div className="bg-background px-3 py-3 border-b">
        <div className="text-sm font-semibold mb-2">备注说明</div>
        <textarea
          placeholder="详细描述异常情况……"
          rows={3}
          className="w-full px-3 py-2 text-sm rounded-md border bg-background resize-none"
          defaultValue="外箱潮湿变形，2 箱底部破损，疑似运输过程压损"
        />
      </div>

      {/* 拍照取证 */}
      <div className="bg-background px-3 py-3 border-b">
        <div className="text-sm font-semibold mb-2">现场照片 <span className="text-destructive">*</span> <span className="text-xs text-muted-foreground font-normal">（至少 2 张）</span></div>
        <div className="grid grid-cols-3 gap-2">
          <Card className="aspect-square flex items-center justify-center bg-muted">
            <span className="text-xs text-muted-foreground">📷 1</span>
          </Card>
          <Card className="aspect-square flex items-center justify-center bg-muted">
            <span className="text-xs text-muted-foreground">📷 2</span>
          </Card>
          <Card className="aspect-square flex items-center justify-center border-dashed cursor-pointer">
            <Camera className="h-6 w-6 text-muted-foreground" />
          </Card>
        </div>
        <div className="text-xs text-muted-foreground mt-2">已拍 {photos}/2 张 ✓</div>
      </div>

      {/* 影响预览 */}
      <div className="flex-1 bg-muted/30 px-3 py-3">
        <div className="text-sm font-semibold mb-2">提交后影响</div>
        <DiffPanel
          layout="stacked"
          before={{
            "ASN 状态": "待验收",
            "可用库存": "0",
            "供应商评分": "92 分",
          }}
          after={{
            "ASN 状态": "已拒收",
            "可用库存": "0（不变）",
            "供应商评分": "82 分（-10）",
          }}
        />
        <div className="mt-3 flex items-start gap-2 p-3 bg-destructive/10 rounded-md border border-destructive/30">
          <FileX className="h-4 w-4 text-destructive flex-shrink-0 mt-0.5" />
          <div className="text-xs">
            <div className="font-medium text-destructive">提交后将自动</div>
            <div className="text-muted-foreground mt-1">
              · 通知采购员 + 供应商（企微）<br />
              · 触发供应商质量评分扣分<br />
              · 创建 H2 审计事件（不可撤销）· GSP §73 拒收档案
            </div>
          </div>
        </div>
      </div>

      {/* 底部 */}
      <div className="bg-background px-3 py-2 border-t flex gap-2">
        <Button variant="outline" className="flex-1 h-10">取消</Button>
        <Button variant="destructive" className="flex-[2] h-10">提交拒收</Button>
      </div>
    </div>
  );
}
