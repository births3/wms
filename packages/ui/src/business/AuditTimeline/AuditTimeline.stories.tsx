import type { Meta, StoryObj } from "@storybook/react";
import { AuditTimeline } from "./AuditTimeline";

const meta: Meta<typeof AuditTimeline> = {
  title: "Components/AuditTimeline",
  component: AuditTimeline,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "审计时间线 · append-only · 倒序时间轴 · 可展开详情。覆盖 H2-002 审计追踪 / M6-002 月报审计 / M6-004 特殊药品台账。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof AuditTimeline>;

export const M2InboundChain: Story = {
  name: "M2 入库审计链（提交 → 双签 → 主管审批）",
  args: {
    events: [
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
    ],
  },
};

export const Expanded: Story = {
  name: "展开详情（点击 e2 查看 diff）",
  args: {
    expandedId: "e2",
    events: [
      {
        id: "e2",
        time: "2026-05-23 09:16:08",
        actor: "u002 李四",
        action: "二次签字",
        module: "M2",
        resource: "PO-2026-0001",
        status: "completed",
        detail: (
          <div className="text-xs">
            <div className="text-muted-foreground mb-1">变化字段</div>
            <div>
              <code>second_sign_user</code>: <span className="text-muted-foreground">null</span> →{" "}
              <code>u002</code>
            </div>
            <div>
              <code>second_sign_time</code>: <span className="text-muted-foreground">null</span> →{" "}
              <code>2026-05-23T09:16:08+08:00</code>
            </div>
          </div>
        ),
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
    ],
  },
};

export const Empty: Story = {
  name: "空时间线（无事件）",
  args: { events: [] },
};
