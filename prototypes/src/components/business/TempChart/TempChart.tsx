import * as React from "react";
import { cn } from "@/lib/utils";
import { StatusBadge } from "../StatusBadge";

/**
 * TempChart — 温度曲线（SVG 渲染，无 chart 库依赖）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：US-M5-002（温控接入展示）/ US-M5-003（温度超标）/ US-M10-002（在途温控）/ US-ST-006（门店冷链）
 * Wave：Wave 4（M5/M10 冷链业务）
 * 业务约束：超阈值区域必须红色着色；阈值线虚线；最低/最高 3 档色阶
 *
 * @example
 *   <TempChart points={[{t:"08:00",v:5.2},...]} minThreshold={2} maxThreshold={8} />
 */

export interface TempPoint {
  t: string;
  v: number;
}

export interface TempChartProps extends React.HTMLAttributes<HTMLDivElement> {
  points: TempPoint[];
  minThreshold: number;
  maxThreshold: number;
  /** 单位 */
  unit?: string;
  /** 视图高度 */
  height?: number;
}

export const TempChart = React.forwardRef<HTMLDivElement, TempChartProps>(
  ({ points, minThreshold, maxThreshold, unit = "℃", height = 240, className, ...rest }, ref) => {
    if (points.length === 0) {
      return (
        <div ref={ref} className={cn("text-sm text-muted-foreground p-8 text-center", className)} {...rest}>
          无温度数据
        </div>
      );
    }

    const W = 800;
    const H = height;
    const PAD_L = 40;
    const PAD_R = 16;
    const PAD_T = 16;
    const PAD_B = 28;

    const values = points.map((p) => p.v);
    const dataMin = Math.min(...values, minThreshold);
    const dataMax = Math.max(...values, maxThreshold);
    const range = dataMax - dataMin || 1;
    const yPad = range * 0.15;
    const yMin = dataMin - yPad;
    const yMax = dataMax + yPad;

    const xScale = (i: number) => PAD_L + (i / Math.max(1, points.length - 1)) * (W - PAD_L - PAD_R);
    const yScale = (v: number) => PAD_T + ((yMax - v) / (yMax - yMin)) * (H - PAD_T - PAD_B);

    const path = points.map((p, i) => `${i === 0 ? "M" : "L"} ${xScale(i).toFixed(1)} ${yScale(p.v).toFixed(1)}`).join(" ");
    const areaPath = `${path} L ${xScale(points.length - 1).toFixed(1)} ${yScale(yMin).toFixed(1)} L ${xScale(0).toFixed(1)} ${yScale(yMin).toFixed(1)} Z`;

    const breaches = points.filter((p) => p.v < minThreshold || p.v > maxThreshold);
    const status = breaches.length === 0 ? "qualified" : "unqualified";

    // X 轴抽样标签（最多 6 个）
    const labelStep = Math.max(1, Math.floor(points.length / 6));
    const labels = points.filter((_, i) => i % labelStep === 0);

    return (
      <div ref={ref} className={cn("font-sans", className)} {...rest}>
        <div className="flex items-center justify-between mb-2 text-sm">
          <div className="flex items-center gap-3">
            <span className="font-medium">温度曲线</span>
            <span className="text-xs text-muted-foreground">
              范围 {minThreshold}~{maxThreshold}{unit} · 共 {points.length} 个采样点
            </span>
          </div>
          <StatusBadge
            status={status}
            size="sm"
            label={breaches.length === 0 ? "全程合格" : `${breaches.length} 次超阈值`}
          />
        </div>
        <svg viewBox={`0 0 ${W} ${H}`} className="w-full h-auto bg-background border rounded-md">
          {/* 阈值带（合格区域） */}
          <rect
            x={PAD_L}
            y={yScale(maxThreshold)}
            width={W - PAD_L - PAD_R}
            height={yScale(minThreshold) - yScale(maxThreshold)}
            fill="hsl(var(--wms-success))"
            fillOpacity={0.06}
          />
          {/* 阈值线 */}
          <line x1={PAD_L} x2={W - PAD_R} y1={yScale(maxThreshold)} y2={yScale(maxThreshold)} stroke="hsl(var(--wms-warning))" strokeDasharray="4,3" />
          <line x1={PAD_L} x2={W - PAD_R} y1={yScale(minThreshold)} y2={yScale(minThreshold)} stroke="hsl(var(--wms-warning))" strokeDasharray="4,3" />
          {/* 阈值文字 */}
          <text x={PAD_L - 6} y={yScale(maxThreshold) + 3} fontSize={10} textAnchor="end" fill="hsl(var(--muted-foreground))">{maxThreshold}{unit}</text>
          <text x={PAD_L - 6} y={yScale(minThreshold) + 3} fontSize={10} textAnchor="end" fill="hsl(var(--muted-foreground))">{minThreshold}{unit}</text>
          {/* 曲线下方填充 */}
          <path d={areaPath} fill="hsl(var(--primary))" fillOpacity={0.08} />
          {/* 曲线 */}
          <path d={path} fill="none" stroke="hsl(var(--primary))" strokeWidth={1.5} />
          {/* 超标点 */}
          {points.map((p, i) => {
            const breach = p.v < minThreshold || p.v > maxThreshold;
            if (!breach) return null;
            return <circle key={i} cx={xScale(i)} cy={yScale(p.v)} r={3} fill="hsl(var(--destructive))" />;
          })}
          {/* X 轴标签 */}
          {labels.map((p, i) => {
            const idx = i * labelStep;
            return (
              <text key={i} x={xScale(idx)} y={H - 8} fontSize={10} textAnchor="middle" fill="hsl(var(--muted-foreground))">
                {p.t}
              </text>
            );
          })}
        </svg>
      </div>
    );
  }
);
TempChart.displayName = "TempChart";
