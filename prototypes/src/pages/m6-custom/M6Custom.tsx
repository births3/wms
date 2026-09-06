import { M6CustomBuilder } from "./M6CustomBuilder";

/**
 * M6Custom — M6-003 业务报表（自建查询 / 行列值三栏）
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M6-003（自定义维度+指标+图表+保存模板+订阅+Metabase 嵌入）
 * Wave：Wave 0.5（演示原型）→ Wave 5（Metabase 嵌入正式上线）
 * 业务约束：仿 Metabase 拖拽体验；不替代 GSP 法定报表（M6-001）；查询写 H2 审计
 *
 * 参考 ADR-0023：混合方案 — 当前 mock；Wave 5 接 Metabase iframe
 *
 * @example
 *   <M6Custom />
 */
export function M6Custom() {
  return <M6CustomBuilder />;
}
