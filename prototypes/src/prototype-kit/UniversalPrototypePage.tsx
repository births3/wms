import type { MatrixPrototypeSpec } from "./types";
import { MobilePrototype } from "./MobilePrototype";
import { WorkspacePrototype } from "./WorkspacePrototype";

/**
 * UniversalPrototypePage — 全量矩阵故事原型路由
 *
 * 层级：Layer 3 页面级支撑组件
 * 关联故事：docs/prototypes/prototype-matrix-r3.md 中所有非豁免 UI 故事
 * Wave：Wave 0.5+ 全量原型补齐
 * 业务约束：每个 story/end 必须经故事专属模型渲染，禁止退回泛化字段模板
 *
 * @example
 *   <UniversalPrototypePage spec={spec} />
 */
export function UniversalPrototypePage({ spec }: { spec: MatrixPrototypeSpec }) {
  if (spec.end === "pda") return <MobilePrototype spec={spec} mode="pda" />;
  if (spec.end === "h5") return <MobilePrototype spec={spec} mode="h5" />;
  if (spec.end === "pad") return <WorkspacePrototype spec={spec} mode="pad" />;
  return <WorkspacePrototype spec={spec} mode="pc" />;
}
