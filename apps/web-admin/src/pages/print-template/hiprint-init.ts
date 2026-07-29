/**
 * hiprint 全局初始化守卫：hiprint.init 重复执行会重复注册 provider 元素，
 * 设计器与预览弹窗共用此单次初始化入口（模块级标志，行为与首次初始化一致）。
 */
let hiprintInitialized = false;

export function initHiprintOnce(hiprintModule: typeof import("hiprint")) {
  if (hiprintInitialized) return;
  hiprintModule.disAutoConnect();
  hiprintModule.hiprint.init({ providers: [new hiprintModule.defaultElementTypeProvider()] });
  hiprintInitialized = true;
}
