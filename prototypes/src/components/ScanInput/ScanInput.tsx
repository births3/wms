import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import { colors, fontStack } from "../../tokens";

export type ScanMode = "scanner" | "camera" | "manual";

export interface ScanInputProps {
  /** 当前模式 */
  mode?: ScanMode;
  /** 模式切换回调 */
  onModeChange?: (mode: ScanMode) => void;
  /** 扫码完成回调（按 Enter 或扫枪输入完成） */
  onScan: (code: string) => void;
  /** 占位提示 */
  placeholder?: string;
  /** 校验失败提示（红框） */
  error?: string;
  /** 最近一次扫码值（用于反馈） */
  lastScanned?: string;
  /** 自动 focus（PDA 上必须，扫枪靠 keyboard 模拟） */
  autoFocus?: boolean;
  /** PDA 端使用更大触控目标 */
  minTouchTarget?: number;
  className?: string;
  testId?: string;
}

const MODE_LABEL: Record<ScanMode, string> = {
  scanner: "扫枪",
  camera: "摄像头",
  manual: "手动",
};

export function ScanInput({
  mode = "scanner",
  onModeChange,
  onScan,
  placeholder = "扫码或输入...",
  error,
  lastScanned,
  autoFocus = true,
  minTouchTarget = 48,
  className,
  testId,
}: ScanInputProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [value, setValue] = useState("");
  const [flash, setFlash] = useState(false);

  // PDA 上扫枪靠 keyboard 模拟，必须保持 focus
  useEffect(() => {
    if (autoFocus && mode === "scanner") inputRef.current?.focus();
  }, [autoFocus, mode]);

  // lastScanned 变化时闪烁反馈
  useEffect(() => {
    if (!lastScanned) return;
    setFlash(true);
    const t = setTimeout(() => setFlash(false), 300);
    return () => clearTimeout(t);
  }, [lastScanned]);

  const handleKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && value.trim()) {
      onScan(value.trim());
      setValue("");
    }
  };

  const cycleMode = () => {
    const order: ScanMode[] = ["scanner", "camera", "manual"];
    const next = order[(order.indexOf(mode) + 1) % order.length];
    onModeChange?.(next);
  };

  const containerStyle: CSSProperties = {
    display: "flex",
    alignItems: "stretch",
    border: `2px solid ${error ? colors.danger : flash ? colors.success : colors.neutral[300]}`,
    borderRadius: 8,
    overflow: "hidden",
    background: "#fff",
    transition: "border-color 0.2s",
    fontFamily: fontStack.sans,
  };

  return (
    <div className={className} data-testid={testId} data-mode={mode}>
      <div style={containerStyle}>
        <button
          type="button"
          onClick={cycleMode}
          aria-label={`切换扫码模式，当前 ${MODE_LABEL[mode]}`}
          style={{
            minWidth: minTouchTarget + 8,
            minHeight: minTouchTarget,
            background: colors.neutral[100],
            border: "none",
            borderRight: `1px solid ${colors.neutral[300]}`,
            cursor: "pointer",
            fontSize: 14,
            fontWeight: 500,
            color: colors.neutral[700],
            padding: "0 10px",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            gap: 4,
            boxShadow: "inset 0 -2px 0 rgba(0,0,0,0.06)",
          }}
        >
          <span>{MODE_LABEL[mode]}</span>
          <span style={{ fontSize: 10, opacity: 0.6 }} aria-hidden>▼</span>
        </button>
        <input
          ref={inputRef}
          type="text"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKey}
          placeholder={placeholder}
          readOnly={mode === "camera"}
          style={{
            flex: 1,
            minHeight: minTouchTarget,
            border: "none",
            outline: "none",
            padding: "0 12px",
            fontSize: 18, // PDA ≥ 16pt
          }}
        />
        {mode === "camera" && (
          <button
            type="button"
            aria-label="启动摄像头扫码"
            style={{
              minWidth: minTouchTarget,
              minHeight: minTouchTarget,
              background: colors.primary,
              color: "#fff",
              border: "none",
              cursor: "pointer",
              fontSize: 20,
            }}
          >
            📷
          </button>
        )}
      </div>
      {error && (
        <div role="alert" style={{ color: colors.danger, fontSize: 14, marginTop: 4 }}>
          {error}
        </div>
      )}
      {lastScanned && !error && (
        <div style={{ color: colors.success, fontSize: 14, marginTop: 4 }}>
          ✓ 已识别：{lastScanned}
        </div>
      )}
    </div>
  );
}
