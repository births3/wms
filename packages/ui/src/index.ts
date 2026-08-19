/**
 * @wms/ui — barrel export
 *
 * 消费方典型用法：
 *   import { Button, StatusBadge, cn } from "@wms/ui";
 *   import "@wms/ui/styles/globals.css";  // 仅在应用入口 import 一次
 *
 * 子路径导入（细粒度）：
 *   import { Button } from "@wms/ui/ui";
 *   import { StatusBadge } from "@wms/ui/business";
 *   import { cn } from "@wms/ui/lib/utils";
 */

// Layer 1 primitives (shadcn/ui)
export * from "./ui";

// Layer 2 业务复合组件（含 16 个：StatusBadge / ScanInput / DualSignPanel ...）
export * from "./business";

// 工具函数
export { cn } from "./lib/utils";
export {
  LOCATION_BATCH_MAX_COUNT,
  buildLocationBatchPreview,
  locationBatchRangeCsv,
  parseLocationBatchCsv,
  toLocationBatchGeneratePayload,
  validateLocationBatchRange,
} from "./lib/location-batch";
export type {
  LocationBatchEncoding,
  LocationBatchPreview,
  LocationBatchPreviewGroup,
  LocationBatchRange,
} from "./lib/location-batch";
export {
  formatDateTime,
  formatZhDate,
} from "./lib/datetime";
