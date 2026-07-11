import type { MatrixPrototypeSpec } from "./types";

/**
 * 关键字派生的局部画像 —— 在某些故事场景下覆盖列/字段及其示例值。
 *
 * 当提供 `columns` 时，必须同时提供等长的 `rowSample`；提供 `fields` 时同理需要 `fieldSample`。
 */
type KeywordProfile = Partial<{
  primaryObject: string;
  columns: string[];
  rowSample: string[];
  fields: string[];
  fieldSample: string[];
  steps: string[];
  actions: string[];
  exceptions: string[];
  statValues: string[];
}>;

export function keywordProfile(spec: MatrixPrototypeSpec): KeywordProfile {
  const text = `${spec.title}${spec.reason}`;
  if (/登录|token 签发/i.test(text)) {
    return {
      primaryObject: "登录会话",
      fields: ["登录端", "工牌码", "MFA 状态", "登录 IP"],
      fieldSample: ["PC 管理端", "WORK-12345", "已校验", "10.42.18.9"],
      steps: ["选择三角色入口", "扫码或账号认证", "签发访问 Token", "记录登录审计"],
      actions: ["切换租户", "重新认证"],
      exceptions: ["连续失败锁定", "租户角色不匹配"],
    };
  }
  if (/API Key|密钥/i.test(text)) {
    return {
      primaryObject: "API Key",
      fields: ["Key 名称", "Secret 一次性展示", "IP 白名单", "轮换周期"],
      fieldSample: ["扫码秤接入 Key", "kg_••••sec_5f", "10.42.0.0/16", "90 天"],
      steps: ["创建 Key", "展示 Secret", "配置 Scope", "到期轮换"],
      actions: ["立即轮换", "吊销 Key"],
      exceptions: ["Secret 已隐藏", "Scope 越权"],
    };
  }
  if (/审计|归档|保留/.test(text)) {
    return {
      primaryObject: "审计证据",
      fields: ["归档批次", "保留策略", "哈希链状态", "查询条件"],
      fieldSample: ["审计分区待冻结", "≥5 年保留", "链路完整", "操作人=u001"],
      steps: ["筛选事件", "校验链路", "导出证据", "归档封存"],
      actions: ["校验完整性", "生成证据包"],
      exceptions: ["归档分区缺失", "哈希校验失败"],
    };
  }
  if (/容器|LPN/i.test(text)) {
    const isRecovery = /回收|追踪/.test(text);
    return {
      primaryObject: isRecovery ? "容器回收任务" : "LPN 容器",
      columns: ["LPN/容器", "容器类型", "绑定对象", "库位/车辆", "回收状态", "状态"],
      rowSample: ["LPN-2026-0001", "周转箱 (60L)", "葡萄糖 / 20260301A", "A01-02-03", "在用", ""],
      fields: ["LPN 号", "容器类型", "绑定商品/批号", "当前库位", "绑定状态", "回收状态", "最后扫码人"],
      fieldSample: ["LPN-2026-0001", "周转箱 (60L)", "葡萄糖 / 20260301A", "A01-02-03", "已绑定", "在用", "u001 张三"],
      steps: isRecovery
        ? ["扫描容器码", "核对门店/车辆", "确认回收状态", "生成回收记录"]
        : ["扫描 LPN", "绑定商品/批号", "核对库位", "解绑或转移"],
      actions: isRecovery ? ["确认回收", "登记丢失", "导出追踪"] : ["绑定 LPN", "解绑容器", "打印容器标签"],
      exceptions: ["LPN 已绑定", "容器丢失", "状态不允许解绑", "重复扫码"],
      statValues: ["在用容器", "待解绑", "待回收", "异常 LPN", "扫码完成率"],
    };
  }
  if (/客户|门店档案/.test(text)) {
    return {
      primaryObject: "客户/门店档案",
      fields: ["客户编码", "门店编码", "层级关系", "配送地址", "经营资质", "启停状态"],
      fieldSample: ["CUST-001 同仁堂连锁", "STORE-018 朝阳门店", "总部 → 朝阳大区 → 朝阳门店", "北京朝阳区朝外大街", "GSP 2026-A", "启用"],
      steps: ["维护客户", "绑定门店层级", "校验资质", "发布档案"],
      actions: ["新增客户", "导入门店", "停用档案"],
      exceptions: ["层级冲突", "资质过期"],
    };
  }
  if (/多货主|数据隔离/.test(text)) {
    return {
      primaryObject: "货主隔离策略",
      fields: ["货主", "租户", "数据隔离范围", "默认仓", "权限边界", "跨货主规则"],
      fieldSample: ["国药控股北京", "tenant-gsp-a", "仓储 / 质量 / 报表", "中央仓 W01", "owner:gsp-a", "禁止跨货主访问"],
      steps: ["选择货主", "校验租户隔离", "切换数据范围", "写入审计"],
      actions: ["切换货主", "校验隔离", "导出权限"],
      exceptions: ["跨货主越权", "租户不匹配"],
    };
  }
  if (/特殊药品|麻精|放射|血液制品/.test(text)) {
    return {
      primaryObject: "特殊药品分类",
      fields: ["特殊药品类别", "麻精标识", "放射药品标识", "血液制品标识", "双人策略", "专用台账"],
      fieldSample: ["麻醉药品", "是 (麻 1)", "否", "否", "扫码 + 双签", "RPT-MNG-001"],
      steps: ["维护分类", "绑定管控规则", "启用双人复核", "同步台账"],
      actions: ["新增分类", "绑定规则", "停用分类"],
      exceptions: ["分类重复", "缺少双人策略"],
    };
  }
  if (/追踪|跟踪/.test(text)) {
    return {
      primaryObject: "追踪时间线",
      columns: ["追踪对象", "当前位置", "上一节点", "事件时间", "责任人", "状态"],
      rowSample: ["BOX-TC-001 周转箱", "同仁堂朝阳门店", "顺丰中转站", "2026-05-24 09:42", "u001 张三", ""],
      fields: ["追踪对象", "事件节点", "当前位置", "上一节点", "责任人", "时间线状态"],
      fieldSample: ["BOX-TC-001 周转箱", "门店签收", "同仁堂朝阳门店", "顺丰中转站", "u001 张三", "已闭环"],
      steps: ["选择追踪对象", "加载事件时间线", "定位当前节点", "导出追踪证据"],
      actions: ["刷新追踪", "导出时间线"],
      exceptions: ["事件断点", "节点缺失"],
      statValues: ["追踪对象", "异常节点", "今日事件", "断点记录", "覆盖率"],
    };
  }
  if (/养护/.test(text)) {
    return {
      primaryObject: "在库养护任务",
      fields: ["养护计划", "养护项目", "养护结果", "库位", "批号", "异常结论"],
      fieldSample: ["MNT-2026-Q2", "外观 / 温度 / 湿度", "通过", "A01-02-03", "20260301A", "无异常"],
      steps: ["生成养护计划", "PDA 记录养护", "质量复核", "归档养护记录"],
      actions: ["提交养护", "拍照取证", "生成质量联系单"],
      exceptions: ["养护逾期", "异常未闭环"],
    };
  }
  if (/ABC/.test(text)) {
    return {
      primaryObject: "ABC 分类结果",
      fields: ["ABC 类别", "分类规则", "周转率", "库存金额", "复核状态"],
      fieldSample: ["A 类高周转", "近 90 天周转 ≥ 12 次", "14.6 次/年", "128 万元", "已复核"],
      steps: ["计算分类", "复核规则", "发布结果", "同步策略"],
      actions: ["重新计算", "调整分类"],
      exceptions: ["规则缺失", "分类冲突"],
    };
  }
  if (/合并|拆单/.test(text)) {
    return {
      primaryObject: "合并/拆单方案",
      fields: ["合并批次", "拆单规则", "订单组合", "承运限制", "预览结果"],
      fieldSample: ["WAVE-051", "按温区 + 客户合并", "SO-2026-0001 + SO-2026-0002", "顺丰冷运 ≤ 50 kg", "可执行"],
      steps: ["选择订单", "预览合并", "执行拆单", "同步发货"],
      actions: ["合并发货", "拆分订单"],
      exceptions: ["承运限制冲突", "库存不足"],
    };
  }
  if (/越库|Cross-Docking/i.test(text)) {
    return {
      primaryObject: "越库任务",
      fields: ["越库单", "来源到货", "目标门店", "交叉月台", "转运状态"],
      fieldSample: ["XD-2026-0001", "ASN-2026-0001", "STORE-018 朝阳门店", "月台 D03", "转运中"],
      steps: ["识别越库", "扫码交接", "月台转运", "确认出库"],
      actions: ["释放越库", "扫码交接"],
      exceptions: ["到货短缺", "目标门店不匹配"],
    };
  }
  if (/退货/.test(text)) {
    return {
      primaryObject: "退货处理单",
      fields: ["退货单号", "退货原因", "原销售单", "批号", "质量结论", "入库去向"],
      fieldSample: ["RTN-2026-0001", "门店多送 / 临期", "SO-2026-0001", "20260301A", "合格可销", "回库 A01-02-03"],
      steps: ["登记退货", "PDA 验收", "质量判定", "生成入库/拒收"],
      actions: ["确认退货", "发起质量处理"],
      exceptions: ["批号不符", "退货超期"],
    };
  }
  if (/PIX|交易类型|三码/.test(text)) {
    return {
      primaryObject: "PIX 交易类型字典",
      fields: ["交易类型", "PIX 三码", "来源系统", "目标动作", "映射状态"],
      fieldSample: ["PIX-IN-001 验收入库", "20-30-10", "ERP", "create_inbound", "已发布"],
      steps: ["维护字典", "绑定 PIX 三码", "试算映射", "发布生效"],
      actions: ["新增交易类型", "试算映射"],
      exceptions: ["三码冲突", "交易类型缺失"],
    };
  }
  if (/码库|大中小码/.test(text)) {
    return {
      primaryObject: "追溯码码库",
      fields: ["大码", "中码", "小码", "码段", "层级关系", "占用状态"],
      fieldSample: ["BOX-AB12", "AB12-CD34", "AB12-CD34-EF56", "EF56-0001 ~ EF56-9999", "1 : 10 : 100", "已占用"],
      steps: ["导入码段", "建立大中小码层级", "绑定业务单据", "校验码库状态"],
      actions: ["导入码库", "释放码段"],
      exceptions: ["码重复", "层级断链"],
    };
  }
  if (/Put-to-Light/i.test(text)) {
    return {
      primaryObject: "装箱灯光任务",
      fields: ["装箱任务", "Put-to-Light 格口", "箱号", "SKU", "应放数量", "实放数量"],
      fieldSample: ["PK-2026-0001", "格口 #07", "BOX-PK-0001", "ITEM-0001", "12 盒", "12 盒"],
      steps: ["扫描订单", "点亮格口", "装箱确认", "关闭箱号"],
      actions: ["点亮格口", "重开箱"],
      exceptions: ["格口异常", "数量不符"],
    };
  }
  if (/保温箱/.test(text)) {
    return {
      primaryObject: "保温箱配置",
      fields: ["保温箱规格", "温区", "蓄冷介质", "有效时长", "适用线路"],
      fieldSample: ["L 保温箱 (40 L)", "2~8°C", "蓄冷剂 ×4", "24 小时", "北京-天津"],
      steps: ["维护箱规", "绑定温区", "校验时长", "发布配置"],
      actions: ["新增保温箱", "停用配置"],
      exceptions: ["温区不匹配", "时长不足"],
    };
  }
  if (/打印|面单|随货同行单|标签/.test(text)) {
    return {
      primaryObject: "打印任务",
      fields: ["打印模板", "打印机", "重打原因", "预览页数"],
      fieldSample: ["PRT-OUT-A", "PRT-DOCK-02", "首次打印", "1 / 1 页"],
      steps: ["选择模板", "预览版式", "发送打印", "记录回执"],
      actions: ["重新打印", "切换模板"],
      exceptions: ["打印机离线", "模板变量缺失"],
    };
  }
  if (/盘点|库存/.test(text)) {
    return {
      primaryObject: "库存盘点对象",
      fields: ["账面数量", "实盘数量", "差异原因", "盘点轮次"],
      fieldSample: ["128 盒", "120 盒", "破损 -8 盒", "第 1 轮"],
      steps: ["锁定范围", "扫码盘点", "差异复核", "生成调整"],
      actions: ["确认差异", "复盘"],
      exceptions: ["重复扫描", "库存锁定冲突"],
    };
  }
  if (/拣选|复核|出库|交接/.test(text)) {
    return {
      primaryObject: "出库作业任务",
      fields: ["拣选路径", "周转箱", "复核结论", "交接人"],
      fieldSample: ["路径 P-A", "LPN-2026-0001", "通过", "u002 李四"],
      steps: ["领取任务", "按路径拣选", "复核批号", "交接发运"],
      actions: ["登记短拣", "完成复核"],
      exceptions: ["库位无货", "批号不一致"],
    };
  }
  if (/冷链|温控|温度|超温/.test(text)) {
    return {
      primaryObject: "温控记录",
      fields: ["温度上限", "温度下限", "采集频率", "超温时长"],
      fieldSample: ["8°C", "2°C", "1 分钟 / 次", "0 分钟"],
      steps: ["采集温度", "阈值判断", "联动批次", "生成告警"],
      actions: ["查看曲线", "发起质量处理"],
      exceptions: ["设备离线", "超温未闭环"],
    };
  }
  if (/规则|策略|配置|映射|参数/.test(text)) {
    return {
      primaryObject: "配置规则",
      fields: ["条件表达式", "命中动作", "生效范围", "灰度状态"],
      fieldSample: ["temp > 8°C && hold > 30min", "create_alert", "tenant-gsp-a / 全模块", "灰度 30%"],
      steps: ["编辑规则", "沙箱验证", "审批发布", "监控命中"],
      actions: ["运行沙箱", "发布规则"],
      exceptions: ["规则冲突", "灰度失败"],
    };
  }
  if (/签收|司机|门店|H5/.test(text)) {
    return {
      primaryObject: "移动签收任务",
      fields: ["定位", "照片凭证", "电子签名", "签收备注"],
      fieldSample: ["GPS 116.41,39.91", "2 张照片", "已签", "无异常"],
      steps: ["打开任务", "扫码核验", "采集凭证", "提交回传"],
      actions: ["拍照", "签名", "回传"],
      exceptions: ["定位异常", "凭证缺失"],
    };
  }
  return {};
}
