import type { Meta, StoryObj } from "@storybook/react";
import { useState } from "react";
import { ScanInput, type ScanMode } from "./ScanInput";

const meta: Meta<typeof ScanInput> = {
  title: "Components/ScanInput",
  component: ScanInput,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "PDA 扫码输入：扫枪 / 摄像头 / 手动三模式切换。扫枪模式靠 keyboard 模拟，必须 autoFocus。覆盖故事 M2-002/003, M4-003, TC-004/006, BA-003。",
      },
    },
  },
};
export default meta;
type Story = StoryObj<typeof ScanInput>;

export const Default: Story = {
  args: {
    mode: "scanner",
    placeholder: "扫码追溯码或商品码",
    onScan: (code) => alert(`扫到：${code}`),
  },
};

export const Interactive: Story = {
  name: "完整交互（模式切换 + 历史反馈）",
  render: () => {
    const [mode, setMode] = useState<ScanMode>("scanner");
    const [last, setLast] = useState<string>();
    return (
      <div style={{ padding: 16, maxWidth: 400 }}>
        <ScanInput
          mode={mode}
          onModeChange={setMode}
          lastScanned={last}
          onScan={(code) => setLast(code)}
          placeholder="扫码或手动输入"
        />
      </div>
    );
  },
};

export const WithError: Story = {
  args: {
    mode: "manual",
    error: "追溯码不在码库中",
    onScan: () => {},
  },
};

export const PdaSize: Story = {
  name: "PDA 端（minTouchTarget=48）",
  args: {
    mode: "scanner",
    minTouchTarget: 48,
    placeholder: "扫码追溯码",
    onScan: () => {},
  },
};
