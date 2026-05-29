// @governance: skip-page-size
// 理由：本页是组件目录的"全展示"页，每加一个 Layer 2 业务复合组件就要新增
// 一个 Showcase（≈ 30-60 行）。当前 16 个组件 ≈ 580 行，未来还会随业务页接入
// 持续扩展。"按字母拆 Part1/Part2"无语义价值；"按层拆"让走查时跨文件来回
// 跳。本页存在的全部目的就是"一站式走查"，500 行门禁的"提取组件改善业务
// 复杂度"动机不适用。
import { useState } from "react";
import { Lock, WifiOff } from "lucide-react";
import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import {
  StatusBadge,
  type StatusKey,
  OfflineIndicator,
  ScanInput,
  FieldTable,
  StepFlow,
  DiffPanel,
  DualSignPanel,
  ApprovalFlow,
  AuditTimeline,
  KanbanBoard,
  PrintPreview,
  RuleEditor,
  TempChart,
  PageHeader,
  DataTable,
  EmptyState,
} from "@wms/ui";

/**
 * ComponentsGallery — 业务复合组件库展示页
 *
 * 层级：Layer 3 页面级
 * 关联故事：无业务关联（组件目录展示，业务方走查工具）
 * Wave：Wave 0.5 起步；持续随业务页接入扩展
 * 业务约束：每个组件至少展示 1 个 variant；含极端状态（error/empty/loading）；
 *           本页须涵盖 packages/business 下全部导出组件，与 component-registry.md 对齐
 *
 * 关于豁免页面行数门禁：见文件顶部注释。
 *
 * @example
 *   <ComponentsGallery />
 */
export function ComponentsGallery() {
  return (
    <div className="max-w-[1280px] mx-auto flex flex-col gap-6">
      <header className="bg-background border rounded-xl p-6">
        <h1 className="text-2xl font-semibold">业务复合组件库</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Layer 2 · 16 个业务复合组件 · ADR-0022 规范 · 走查工具
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
          <div className="flex flex-col gap-2">
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

      <Showcase
        title="⑦ DualSignPanel"
        desc="双人签字 · 三档策略（single / dual_scan / dual_scan_with_approval）"
        stories="覆盖：M2-004 入库双签 / VR-006 策略矩阵 / BA-002 批号调整双签"
      >
        <Subsection title="dual_scan_with_approval（双签 + 主管审批，三段全签）">
          <DualSignPanel
            policy="dual_scan_with_approval"
            first={{ user: "u001 张三", time: "09:14" }}
            second={{ user: "u002 李四", time: "09:16" }}
            approval={{ user: "u005 王主管", time: "09:32", comment: "已审批，可上架" }}
          />
        </Subsection>
        <Subsection title="dual_scan（仅第一人已签，第二人待签）">
          <DualSignPanel
            policy="dual_scan"
            first={{ user: "u001 张三", time: "09:14", comment: "外观完好" }}
          />
        </Subsection>
      </Showcase>

      <Showcase
        title="⑧ ApprovalFlow"
        desc="审批流 · 5 状态（pending / approved / rejected / current / skipped）"
        stories="覆盖：QL-003 质量联系单 / BA-002 批号调整 / DOCK-004 月台预约"
      >
        <ApprovalFlow
          nodes={[
            {
              role: "库管",
              approver: "u001 张三",
              time: "2026-05-23 09:14",
              status: "approved",
              comment: "外观完好，数量一致",
            },
            {
              role: "主管",
              approver: "u005 王主管",
              time: "2026-05-23 09:32",
              status: "current",
            },
            { role: "质量负责人", status: "pending" },
          ]}
        />
      </Showcase>

      <Showcase
        title="⑨ AuditTimeline"
        desc="审计时间线 · append-only · 倒序时间轴 · 可展开详情"
        stories="覆盖：H2-002 审计追踪 / M6-002 月报审计 / M6-004 特殊药品台账"
      >
        <AuditTimeline
          events={[
            {
              id: "e3",
              time: "2026-05-23 09:32:14",
              actor: "u005 王主管",
              action: "审批通过",
              module: "M2",
              resource: "PO-2026-0001",
              status: "completed",
            },
            {
              id: "e2",
              time: "2026-05-23 09:16:08",
              actor: "u002 李四",
              action: "二次签字",
              module: "M2",
              resource: "PO-2026-0001",
              status: "completed",
            },
            {
              id: "e1",
              time: "2026-05-23 09:14:32",
              actor: "u001 张三",
              action: "提交验收",
              module: "M2",
              resource: "PO-2026-0001",
              status: "in_progress",
            },
          ]}
        />
      </Showcase>

      <Showcase
        title="⑩ KanbanBoard"
        desc="多列看板 · 卡片优先级（low/normal/high/urgent）· 实时刷新 ≤ 3s"
        stories="覆盖：M2-008 收货看板 / M4-007 出库看板 / DOCK-006 月台占用"
      >
        <KanbanBoard
          columns={[
            {
              title: "待验收",
              variant: "warning",
              items: [
                {
                  id: "k1",
                  title: "PO-2026-0001",
                  subtitle: "国药控股北京",
                  priority: "high",
                  status: "pending",
                  meta: [
                    { label: "件数", value: "240" },
                    { label: "ETA", value: "10:30" },
                  ],
                },
                {
                  id: "k2",
                  title: "PO-2026-0002",
                  subtitle: "上海医药华东",
                  priority: "normal",
                  meta: [{ label: "件数", value: "180" }],
                },
              ],
            },
            {
              title: "验收中",
              variant: "default",
              items: [
                {
                  id: "k3",
                  title: "PO-2026-0003",
                  subtitle: "九州通医药",
                  priority: "urgent",
                  status: "in_progress",
                  meta: [
                    { label: "进度", value: "5/12" },
                    { label: "操作员", value: "u001" },
                  ],
                },
              ],
            },
            {
              title: "已上架",
              variant: "success",
              items: [
                {
                  id: "k4",
                  title: "PO-2026-0000",
                  subtitle: "甘李药业",
                  status: "completed",
                  meta: [{ label: "件数", value: "120" }],
                },
              ],
            },
          ]}
        />
      </Showcase>

      <Showcase
        title="⑪ PrintPreview"
        desc="打印预览容器 · A4 / 标签 / 面单 三种模板 · 支持缩放 + 翻页"
        stories="覆盖：M2-007 收货单 / M4-005 随货同行单 / H5-003 快递面单"
      >
        <Subsection title="A4 模板（随货同行单缩略）">
          <PrintPreviewDemo />
        </Subsection>
      </Showcase>

      <Showcase
        title="⑫ RuleEditor"
        desc="校验规则配置 · 条件组（AND/OR）+ 动作叠加"
        stories="覆盖：VR-001 校验规则 / M-PM-002 参数映射 / M9-001 计费规则"
      >
        <RuleEditor
          readOnly
          fields={["warehouse_type", "temp_zone", "owner_id", "category"]}
          groups={[
            {
              connector: "AND",
              conditions: [
                { field: "warehouse_type", op: "eq", value: "CR" },
                { field: "temp_zone", op: "in", value: "[2-8°C, -25°C]" },
              ],
            },
          ]}
          actions={[
            {
              type: "charge_unit",
              label: "按托盘日计费",
              params: { unit: "托盘", price: "8.5", currency: "CNY/日" },
            },
            {
              type: "alert",
              label: "超温告警",
              params: { channel: "企微", severity: "P2" },
            },
          ]}
        />
      </Showcase>

      <Showcase
        title="⑬ TempChart"
        desc="温度曲线 · 阈值带 · 超阈高亮"
        stories="覆盖：M5-002 冷链监控 / M6-002d 冷链月报 / M10-002 在途温控"
      >
        <TempChart
          minThreshold={2}
          maxThreshold={8}
          unit="°C"
          height={180}
          points={[
            { t: "00:00", v: 4.2 },
            { t: "02:00", v: 4.5 },
            { t: "04:00", v: 5.0 },
            { t: "06:00", v: 5.2 },
            { t: "08:00", v: 5.6 },
            { t: "10:00", v: 6.1 },
            { t: "12:00", v: 7.4 },
            { t: "14:00", v: 8.7 },
            { t: "16:00", v: 7.2 },
            { t: "18:00", v: 6.0 },
            { t: "20:00", v: 5.4 },
            { t: "22:00", v: 4.8 },
          ]}
        />
      </Showcase>

      <Showcase
        title="⑭ PageHeader"
        desc="管理页通用骨架 · 标题 + 副标题 + 操作 + 面包屑"
        stories="覆盖：M1/M3/M6 全部 PC 列表页"
      >
        <div className="border rounded-md">
          <PageHeader
            title="商品档案"
            subtitle="M1-001 · ADR-0021 · 共 1,243 条记录"
            breadcrumb={
              <span className="text-xs text-muted-foreground">
                M1 基础数据 / 商品档案
              </span>
            }
            actions={
              <div className="flex gap-2">
                <Button variant="outline" size="sm">导出</Button>
                <Button size="sm">+ 新增</Button>
              </div>
            }
          />
        </div>
      </Showcase>

      <Showcase
        title="⑮ DataTable"
        desc="管理页通用表格 · 列定义 + 行选中 + caption + footer + 空态"
        stories="覆盖：M1/M3/M6 全部 PC 列表场景"
      >
        <DataTable<GalleryRow>
          rowKey={(r) => r.code}
          caption="共 4 条 · 按商品编码升序"
          columns={[
            { key: "code", header: "商品编码", mono: true, width: 140 },
            { key: "name", header: "品名" },
            { key: "category", header: "类别", width: 100 },
            {
              key: "status",
              header: "状态",
              width: 110,
              render: (r) => <StatusBadge status={r.status} size="sm" />,
            },
          ]}
          data={GALLERY_ROWS}
        />
      </Showcase>

      <Showcase
        title="⑯ EmptyState"
        desc="空状态 · 图标 + 标题 + 描述 + CTA · 通用兜底"
        stories="覆盖：所有列表 / 看板 / 详情场景（无数据 / 无权限 / 网络错误）"
      >
        <div className="grid grid-cols-3 gap-3">
          <div className="border rounded-md">
            <EmptyState
              title="暂无数据"
              description="尝试调整筛选条件或新增第一条记录"
            />
          </div>
          <div className="border rounded-md">
            <EmptyState
              icon={<Lock className="size-10" aria-hidden />}
              title="无访问权限"
              description="该模块需 GSP 报表查阅权限，请联系管理员"
            />
          </div>
          <div className="border rounded-md">
            <EmptyState
              icon={<WifiOff className="size-10" aria-hidden />}
              title="网络异常"
              description="无法连接到服务器，请检查后重试"
              action={<Button size="sm">重试</Button>}
            />
          </div>
        </div>
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
    <Card className="p-6 flex flex-col gap-4">
      <div className="flex items-baseline gap-3 flex-wrap">
        <h2 className="text-lg font-semibold">{title}</h2>
        <span className="text-sm text-muted-foreground">{desc}</span>
        <span className="text-xs text-muted-foreground/80 ml-auto">{stories}</span>
      </div>
      <div className="flex flex-col gap-3">{children}</div>
    </Card>
  );
}

function Subsection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-2">
      <p className="text-xs font-medium text-muted-foreground">{title}</p>
      {children}
    </div>
  );
}

function ScanInputDemo() {
  const [last, setLast] = useState<string>();
  const [mode, setMode] = useState<"scanner" | "camera" | "manual">("scanner");
  return (
    <div className="flex flex-col gap-3">
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

function PrintPreviewDemo() {
  const [zoom, setZoom] = useState(0.6);
  return (
    <PrintPreview
      template="a4"
      pageCount={1}
      currentPage={1}
      zoom={zoom}
      onZoomChange={setZoom}
    >
      <div className="font-sans text-[10px] leading-snug">
        <h3 className="text-base font-semibold text-center mb-1">随货同行单</h3>
        <p className="text-center text-muted-foreground mb-3 text-[9px]">
          SO-2026-0042 · 国药控股北京 → 北京同仁堂
        </p>
        <table className="w-full border-collapse">
          <thead>
            <tr className="border-b border-foreground/30">
              <th className="text-left p-1 font-medium">商品编码</th>
              <th className="text-left p-1 font-medium">品名</th>
              <th className="text-left p-1 font-medium">批号</th>
              <th className="text-right p-1 font-medium">数量</th>
            </tr>
          </thead>
          <tbody>
            <tr className="border-b border-foreground/10">
              <td className="p-1">P-001234</td>
              <td className="p-1">葡萄糖注射液</td>
              <td className="p-1">20260301A</td>
              <td className="p-1 text-right">120 瓶</td>
            </tr>
            <tr className="border-b border-foreground/10">
              <td className="p-1">P-001235</td>
              <td className="p-1">氯化钠注射液</td>
              <td className="p-1">20260315B</td>
              <td className="p-1 text-right">80 瓶</td>
            </tr>
          </tbody>
        </table>
        <div className="mt-4 grid grid-cols-2 gap-2 text-[9px]">
          <div>发货人签字：________</div>
          <div>收货人签字：________</div>
        </div>
        <div className="text-center mt-6 text-[8px] text-muted-foreground">
          GSP-BJ-2026-0001 · SimSun · 210x297mm
        </div>
      </div>
    </PrintPreview>
  );
}

type GalleryRow = {
  code: string;
  name: string;
  category: string;
  status: StatusKey;
};

const GALLERY_ROWS: GalleryRow[] = [
  { code: "P-001234", name: "葡萄糖注射液", category: "普药", status: "qualified" },
  { code: "P-001235", name: "氯化钠注射液", category: "普药", status: "qualified" },
  { code: "P-002001", name: "盐酸吗啡注射液", category: "麻精", status: "isolated" },
  { code: "P-003045", name: "辉瑞牌阿托伐他汀", category: "近效期", status: "near_expiry" },
];
