import * as React from "react";
import { useEffect, useRef, useState } from "react";
import { Camera, ChevronDown } from "lucide-react";
import { Button } from "../../ui/button";
import { cn } from "../../lib/utils";

/**
 * ScanInput — PDA 扫码输入框
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M2-002/003（PDA 收货验收）、M4-003（PDA 拣选）、TC-004/006（追溯码）、BA-003（批号调整）
 * Wave：Wave 0.5 起步
 * 业务约束：扫枪模式 autoFocus 不能丢；摄像头模式 input readOnly；触控目标 ≥ 48pt
 *
 * @example
 *   <ScanInput mode="scanner" onScan={(code) => console.log(code)} placeholder="扫码追溯码" />
 */

export type ScanMode = "scanner" | "camera" | "manual";

const MODE_LABEL: Record<ScanMode, string> = {
  scanner: "扫枪",
  camera: "摄像头",
  manual: "手动",
};

export interface ScanInputProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "onScan"> {
  mode?: ScanMode;
  onModeChange?: (mode: ScanMode) => void;
  onScan: (code: string) => void;
  placeholder?: string;
  error?: string;
  /** 最近一次扫码值，触发闪烁反馈 */
  lastScanned?: string;
  autoFocus?: boolean;
}

export const ScanInput = React.forwardRef<HTMLDivElement, ScanInputProps>(
  (
    {
      mode = "scanner",
      onModeChange,
      onScan,
      placeholder = "扫码或输入...",
      error,
      lastScanned,
      autoFocus = true,
      className,
      ...rest
    },
    ref
  ) => {
    const inputRef = useRef<HTMLInputElement>(null);
    const [value, setValue] = useState("");
    const [flash, setFlash] = useState(false);

    useEffect(() => {
      if (autoFocus && mode === "scanner") inputRef.current?.focus();
    }, [autoFocus, mode]);

    useEffect(() => {
      if (!lastScanned) return;
      setFlash(true);
      const t = setTimeout(() => setFlash(false), 300);
      return () => clearTimeout(t);
    }, [lastScanned]);

    const handleKey = (e: React.KeyboardEvent<HTMLInputElement>) => {
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

    const borderClass = error
      ? "border-destructive"
      : flash
      ? "border-wms-success"
      : "border-input";

    return (
      <div ref={ref} data-mode={mode} className={cn("font-sans", className)} {...rest}>
        <div
          className={cn(
            "flex items-stretch h-12 rounded-md border-2 bg-background overflow-hidden transition-colors",
            borderClass
          )}
        >
          {/* mode toggle */}
          <button
            type="button"
            onClick={cycleMode}
            aria-label={`切换扫码模式，当前 ${MODE_LABEL[mode]}`}
            className="min-w-14 px-3 flex items-center justify-center gap-1 bg-muted/80 border-r text-sm font-medium text-foreground/80 cursor-pointer shadow-[inset_0_-2px_0_rgba(0,0,0,0.06)] hover:bg-muted"
          >
            <span>{MODE_LABEL[mode]}</span>
            <ChevronDown aria-hidden className="size-2.5 opacity-60" />
          </button>
          {/* input */}
          <input
            ref={inputRef}
            type="text"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={handleKey}
            placeholder={placeholder}
            readOnly={mode === "camera"}
            className="flex-1 px-3 text-base bg-transparent outline-none border-none placeholder:text-muted-foreground"
          />
          {mode === "camera" && (
            <Button
              type="button"
              variant="default"
              aria-label="启动摄像头扫码"
              className="rounded-none border-l h-auto px-4"
            >
              <Camera aria-hidden className="size-5" />
            </Button>
          )}
        </div>
        {error && (
          <p role="alert" className="text-destructive text-sm mt-1">
            {error}
          </p>
        )}
        {lastScanned && !error && (
          <p className="text-wms-success text-sm mt-1">✓ 已识别：{lastScanned}</p>
        )}
      </div>
    );
  }
);
ScanInput.displayName = "ScanInput";
