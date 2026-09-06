import {
  Button,
  Card,
  CardContent,
  DualSignPanel,
  FieldTable,
  OfflineIndicator,
  ScanInput,
  StatusBadge,
  StepFlow,
} from "@wms/ui";
import { AlertTriangle, ArrowLeft, Camera, CheckCircle2, FileSignature, MapPin } from "lucide-react";
import type { MatrixPrototypeSpec } from "./types";
import { isApproval, isOfflineHeavy } from "./prototype-classifiers";
import { buildStoryPrototypeModel } from "./prototype-model";

/**
 * MobilePrototype — PDA/H5 全量矩阵移动端原型
 *
 * 层级：Layer 3 页面级支撑组件
 * 关联故事：docs/prototypes/prototype-matrix-r3.md 中 PDA/H5 story/end
 * Wave：Wave 0.5+ 全量原型补齐
 * 业务约束：PDA 保持 48pt 触控基线；H5 使用相同故事模型但不展示 PDA 离线条
 *
 * @example
 *   <MobilePrototype spec={spec} mode="pda" />
 */
export function MobilePrototype({ spec, mode }: { spec: MatrixPrototypeSpec; mode: "pda" | "h5" }) {
  const model = buildStoryPrototypeModel(spec);
  const isPda = mode === "pda";

  return (
    <div
      data-device={isPda ? "pda" : undefined}
      className={isPda ? "w-full max-w-[480px] min-h-[900px] overflow-hidden rounded-xl border bg-muted/30 shadow-md" : "w-full max-w-[430px] min-h-[820px] overflow-hidden rounded-[28px] border bg-muted/30 shadow-md"}
    >
      {isPda && <OfflineIndicator state={isOfflineHeavy(spec) ? "offline" : "online"} pendingCount={isOfflineHeavy(spec) ? 3 : undefined} />}

      <div className="border-b bg-background px-3 py-3">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="icon" className="size-10">
            <ArrowLeft className="size-5" />
          </Button>
          <div className="min-w-0 flex-1">
            <div className="text-xs text-muted-foreground">{spec.storyId} · {model.moduleName}</div>
            <div className="truncate text-base font-semibold">{spec.title}</div>
          </div>
          <StatusBadge status={isOfflineHeavy(spec) ? "offline_cached" : "in_progress"} size="sm" label={isPda ? "作业中" : "处理中"} />
        </div>
      </div>

      <div className="border-b bg-background px-4 py-3">
        <StepFlow orientation="vertical" steps={model.steps} current={Math.min(2, model.steps.length - 1)} size={isPda ? "default" : "sm"} />
      </div>

      <div className="flex flex-col gap-3 px-4 py-4">
        <ScanInput
          mode={isPda ? "scanner" : "camera"}
          placeholder={model.scanPlaceholder}
          onScan={() => undefined}
          lastScanned={model.lastScanned}
        />

        <Card className="rounded-md">
          <CardContent className="p-3">
            <div className="mb-2 flex items-center justify-between">
              <span className="text-sm font-semibold">当前{model.primaryObject}</span>
              <StatusBadge status="pending" size="sm" label="待确认" />
            </div>
            <FieldTable rows={model.fields} size={isPda ? "default" : "sm"} />
          </CardContent>
        </Card>

        {!isPda && <H5EvidencePanel />}

        {isApproval(spec) && (
          <DualSignPanel
            policy="dual_scan_with_approval"
            first={{ user: "u001 张三", time: "09:14" }}
            second={{ user: "u002 李四", time: "09:18" }}
          />
        )}

        <div className="rounded-md border border-wms-warning/30 bg-wms-warning/10 p-3">
          <div className="flex items-start gap-2">
            <AlertTriangle className="mt-0.5 size-4 shrink-0 text-wms-warning" />
            <div>
              <div className="text-sm font-medium text-wms-warning">{model.exceptions[0] ?? "异常兜底"}</div>
              <div className="mt-1 text-xs text-muted-foreground">
                {model.exceptions.slice(1).join(" / ") || spec.reason}；失败时暂存并进入待处理队列。
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="sticky bottom-0 mt-auto border-t bg-background p-3">
        <div className="grid grid-cols-3 gap-2">
          <Button variant="outline" className="h-12">{model.actions[1] ?? "异常"}</Button>
          <Button variant="outline" className="h-12">{isPda ? "暂存" : "保存"}</Button>
          <Button className="h-12">{model.actions[0] ?? "提交"}</Button>
        </div>
      </div>
    </div>
  );
}

function H5EvidencePanel() {
  const items = [
    { label: "定位", value: "已采集", icon: MapPin },
    { label: "照片", value: "2 张", icon: Camera },
    { label: "签名", value: "待确认", icon: FileSignature },
  ];
  return (
    <Card className="rounded-md">
      <CardContent className="grid grid-cols-3 gap-2 p-3">
        {items.map((item) => {
          const Icon = item.icon;
          return (
            <div key={item.label} className="rounded-md border bg-background p-2 text-center">
              <Icon className="mx-auto size-4 text-primary" />
              <div className="mt-1 text-xs text-muted-foreground">{item.label}</div>
              <div className="mt-1 flex items-center justify-center gap-1 text-xs font-medium">
                <CheckCircle2 className="size-3 text-wms-success" />
                {item.value}
              </div>
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}
