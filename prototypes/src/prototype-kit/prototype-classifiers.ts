import type { MatrixPrototypeSpec } from "./types";

const textOf = (spec: MatrixPrototypeSpec) => `${spec.title}${spec.reason}${spec.group}`;

export function isCold(spec: MatrixPrototypeSpec) {
  return /冷链|温控|温度|冷藏|保温/.test(textOf(spec)) || spec.moduleCode === "M5";
}

export function isPrint(spec: MatrixPrototypeSpec) {
  return /打印|面单|随货同行单|标签|模板/.test(textOf(spec));
}

export function isRule(spec: MatrixPrototypeSpec) {
  return /规则|策略|配置|矩阵|阈值|映射|参数|告警定义|通道|静默/.test(textOf(spec));
}

export function isApproval(spec: MatrixPrototypeSpec) {
  return /审批|双人|签字|质量联系单|报损|报溢|调整|异常|取消|变更|不合格/.test(textOf(spec));
}

export function isKanban(spec: MatrixPrototypeSpec) {
  return /看板|大屏|占用|跟踪|进度|任务分配|任务释放|任务合并|月台预约/.test(textOf(spec));
}

export function isScanHeavy(spec: MatrixPrototypeSpec) {
  return spec.end === "pda" || /扫码|扫描|拣选|复核|签收|追溯码|装箱|称重|盘点|上架|验收/.test(textOf(spec));
}

export function isOfflineHeavy(spec: MatrixPrototypeSpec) {
  return spec.end === "pda" && /PDA|扫码|执行|作业|盘点|补货|调整|验收|拣选|上架/.test(textOf(spec));
}
