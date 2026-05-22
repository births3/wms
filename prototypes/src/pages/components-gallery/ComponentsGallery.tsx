import { useState } from "react";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
  StatusBadge,
  type StatusKey,
  OfflineIndicator,
  ScanInput,
  FieldTable,
  StepFlow,
  DiffPanel,
} from "@/components/business";

/**
 * ComponentsGallery — 业务复合组件库展示页
 *
 * 层级：Layer 3 页面级
 * 关联故事：无业务关联（组件目录展示，业务方走查工具）
 * Wave：Wave 0.5 起步
 * 业务约束：每个组件至少展示 3 个 variant；含极端状态（error/empty/loading）
 *
 * @example
 *   <ComponentsGallery />
 */
export function ComponentsGallery() {
  return (
    <div className="max-w-[1280px] mx-auto space-y-6">
      <header className="bg-background border rounded-xl p-6">
        <h1 className="text-2xl font-semibold">业务复合组件库</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Layer 2 · 6 个组件 · ADR-0022 规范 · Wave 0.5 已开发
        </p>
        <p className="text-xs text-muted-foreground mt-2">
          走查工具 — 业务方 review 每个组件的视觉表达；新增组件须在
          <code className="mx-1 px-1 py-0.5 bg-muted rounded">docs/prototypes/component-registry.md</code>
          注册
        </p>
      </header>

      <Showcase
        title="① StatusBadge"
        desc="9 状态映射 + 三档尺寸 · 跨端"
        stories="覆盖：全部状态机故事"
      >
        <Subsection title="9 状态全展示">
          <div className="flex flex-wrap gap-2">
            {(
              [
                "qualified",
                "unqualified",
                "pending",
                "isolated",
                "expired",
                "near_expiry",
                "in_progress",
                "completed",
                "offline_cached",
              ] as StatusKey[]
            ).map((s) => (
              <StatusBadge key={s} status={s} />
            ))}
          </div>
        </Subsection>
        <Subsection title="三档尺寸（PDA 推荐 lg）">
          <div className="flex items-center gap-3">
            <StatusBadge status="qualified" size="sm" />
            <StatusBadge status="qualified" size="default" />
            <StatusBadge status="qualified" size="lg" />
            <StatusBadge status="unqualified" label="外观破损" />
          </div>
        </Subsection>
      </Showcase>

      <Showcase
        title="② OfflineIndicator"
        desc="PDA 顶部 banner · 自动隐藏 online + 100% 完成态"
        stories="覆盖：全部 PDA 故事"
      >
        <Subsection title="离线模式（含待同步计数）">
          <div className="space-y-2">
            <OfflineIndicator state="online" pendingCount={3} />
            <OfflineIndicator state="offline" pendingCount={12} />
            <OfflineIndicator state="syncing" pendingCount={12} syncProgress={45} />
          </div>
        </Subsection>
      </Showcase>

      <Showcase
        title="③ ScanInput"
        desc="扫枪/摄像头/手动 三模式 · Enter 提交 + 闪烁反馈"
        stories="覆盖：M2-002/003, M4-003, TC-004/006, BA-003"
      >
        <ScanInputDemo />
      </Showcase>

      <Showcase
        title="④ FieldTable"
        desc="字段核对表 · autoFilled 蓝边高亮 · required 红星"
        stories="覆盖：M2-003 验收 / M4-004 复核 / M3-005 盘点"
      >
        <Subsection title="M2-003 PDA 验收（lg）">
          <FieldTable
            size="lg"
            rows={[
              { label: "ASN 号", value: "PO-2026-0001" },
              { label: "品名", value: "葡萄糖注射液", autoFilled: true },
              { label: "批号", value: "20260301A", autoFilled: true, required: true },
              { label: "实际到货数量", value: "240 瓶", required: true },
              {
                label: "标签",
                value: <StatusBadge status="unqualified" size="sm" />,
                error: "标签污损，需拍照取证",
              },
            ]}
          />
        </Subsection>
      </Showcase>

      <Showcase
        title="⑤ StepFlow"
        desc="多步骤指示器 · 横向/纵向 · 4 状态（pending/current/completed/error）"
        stories="覆盖：M2-003 验收 / M2-004 双人签字 / M4-003 拣选"
      >
        <Subsection title="横向（双人签字 dual_scan_with_approval）">
          <StepFlow
            current={2}
            steps={[
              { label: "第一人签字", description: "张三 ✓" },
              { label: "第二人签字", description: "李四 ✓" },
              { label: "主管审批", description: "企微推送中" },
              { label: "上架中" },
            ]}
          />
        </Subsection>
        <Subsection title="纵向（M2-003 验收，含失败步）">
          <div className="max-w-[360px]">
            <StepFlow
              orientation="vertical"
              size="lg"
              current={4}
              errorSteps={[3]}
              steps={[
                { label: "扫描追溯码" },
                { label: "核对品名规格" },
                { label: "录入数量" },
                { label: "外观检查", description: "标签污损" },
                { label: "判定质量状态" },
                { label: "提交验收结论" },
              ]}
            />
          </div>
        </Subsection>
      </Showcase>

      <Showcase
        title="⑥ DiffPanel"
        desc="旧值-新值对比 · 变化字段加粗高亮 · side-by-side / stacked"
        stories="覆盖：H2-002 审计 / M-BA-001 批号调整 / M-VR-003 校验异常"
      >
        <Subsection title="side-by-side 布局（默认）">
          <DiffPanel
            before={{ 状态: "验收中", 验收员: "—", 数量: "未确认" }}
            after={{ 状态: "已验收", 验收员: "u001", 数量: "240 瓶" }}
          />
        </Subsection>
        <Subsection title="stacked 布局（紧凑空间）">
          <DiffPanel
            layout="stacked"
            before={{ 批号: "20260301A", 数量: "240" }}
            after={{ 批号: "20260301B", 数量: "240" }}
          />
        </Subsection>
        <Subsection title="无变更">
          <DiffPanel />
        </Subsection>
      </Showcase>
    </div>
  );
}

function Showcase({
  title,
  desc,
  stories,
  children,
}: {
  title: string;
  desc: string;
  stories: string;
  children: React.ReactNode;
}) {
  return (
    <Card className="p-6 space-y-4">
      <div className="flex items-baseline gap-3 flex-wrap">
        <h2 className="text-lg font-semibold">{title}</h2>
        <span className="text-sm text-muted-foreground">{desc}</span>
        <span className="text-xs text-muted-foreground/80 ml-auto">{stories}</span>
      </div>
      <div className="space-y-3">{children}</div>
    </Card>
  );
}

function Subsection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="space-y-2">
      <p className="text-xs font-medium text-muted-foreground">{title}</p>
      {children}
    </div>
  );
}

function ScanInputDemo() {
  const [last, setLast] = useState<string>();
  const [mode, setMode] = useState<"scanner" | "camera" | "manual">("scanner");
  return (
    <div className="space-y-3">
      <Subsection title="完整交互（点'扫枪'按钮切换模式）">
        <div className="max-w-md">
          <ScanInput
            mode={mode}
            onModeChange={setMode}
            lastScanned={last}
            onScan={(c) => setLast(c)}
            placeholder="扫码追溯码或商品码"
          />
        </div>
      </Subsection>
      <Subsection title="错误态">
        <div className="max-w-md">
          <ScanInput mode="manual" error="追溯码不在码库中" onScan={() => {}} />
        </div>
      </Subsection>
    </div>
  );
}
