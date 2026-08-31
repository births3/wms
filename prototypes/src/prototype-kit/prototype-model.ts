/* 治理脚本兼容索引：运行时蓝图与关键字画像来自拆分模块。
export const MODULE_BLUEPRINTS = {
  AL: { name: "告警引擎", primaryObject: "告警定义", columns: ["告警定义", "严重度", "触发源", "SLA", "通知通道", "状态"], rowSample: ["冷链超温 P1 告警", "P1", "温控网关 / IoT", "≤ 5 分钟", "企业微信 + 短信", ""], fields: ["告警编码", "严重度", "触发指标", "响应时限", "通知通道", "升级策略", "静默窗口"], fieldSample: ["AL-2026-0001", "P1", "冷藏库温度 > 8°C", "≤ 5 分钟", "企业微信 + 短信", "升级到质量负责人", "23:00-06:00"], steps: ["选择触发源", "配置阈值", "绑定通道", "发布告警"], actions: ["启用告警", "模拟触发", "暂停通道"], exceptions: ["重复告警合并", "通道发送失败"], stats: ["活跃告警", "P1 未响应", "平均响应", "今日关闭", "静默中"], samples: ["冷链超温 P1", "库存近效期", "接口失败率"], },
  BA: { name: "批号调整", primaryObject: "批号调整单", columns: ["调整单", "原批号", "目标批号", "库存影响", "双签状态", "状态"], rowSample: ["BA-2026-0001", "20260301A", "20260402B", "+128 / -128 盒", "已双签", ""], fields: ["调整原因", "原批号", "目标批号", "货主", "影响数量", "质量审批", "双人签字"], fieldSample: ["效期延长复核", "20260301A", "20260402B", "国药控股北京", "128 盒", "已通过", "u001 / u002"], steps: ["选择库存", "录入目标批号", "质量复核", "双签生效"], actions: ["提交调整", "发起双签", "撤回调整"], exceptions: ["目标批号重复", "库存状态不可调"], stats: ["待复核", "双签中", "今日调整", "隔离批次", "审计覆盖"], samples: ["BA-2026-0001", "20260301A", "20260402B"], },
  DI: { name: "药检单", primaryObject: "药检单", columns: ["药检单", "抽样对象", "检验项目", "检验结论", "放行条件", "状态"], rowSample: ["DI-2026-0001", "葡萄糖注射液 20260301A", "外观 / 含量测定", "合格", "整批放行", ""], fields: ["药检单号", "样品批号", "检验项目", "检验机构", "报告编号", "结论", "放行限制"], fieldSample: ["DI-2026-0001", "20260301A", "外观 / 含量测定", "中检院北京", "RPT-2026-0042", "合格", "无限制"], steps: ["登记样品", "录入检验项", "上传报告", "质量放行"], actions: ["生成药检单", "上传报告", "标记放行"], exceptions: ["报告缺页", "结论不合格"], stats: ["待检", "检验中", "待放行", "不合格", "报告归档"], samples: ["DI-2026-0001", "外观检查", "含量测定"], },
  DOCK: { name: "月台预约", primaryObject: "月台预约", columns: ["预约号", "月台", "承运商", "时间窗", "车辆/司机", "状态"], rowSample: ["DOCK-2026-0001", "月台 D03", "顺丰冷运", "09:00-09:30", "京A-6H58 / 王强", ""], fields: ["预约号", "月台号", "预约时间窗", "承运商", "车牌号", "司机电话", "签到位置"], fieldSample: ["DOCK-2026-0001", "D03", "09:00-09:30", "顺丰冷运", "京A-6H58", "138-0000-1234", "GPS 116.41,39.91"], steps: ["选择时间窗", "锁定月台", "车辆签到", "放行离场"], actions: ["改约", "签到", "释放月台"], exceptions: ["车辆迟到", "月台冲突"], stats: ["今日预约", "待签到", "月台占用", "超时车辆", "准点率"], samples: ["DOCK-2026-0001", "月台 D03", "京A-6H58"], },
  DR: { name: "司机端", primaryObject: "司机任务", columns: ["运输任务", "司机", "车辆", "签收点", "里程/温度", "状态"], rowSample: ["DR-2026-0001", "王强", "沪B-8K21", "同仁堂朝阳门店", "12.8 km / 4.8°C", ""], fields: ["运输任务", "司机姓名", "车牌号", "当前位置", "签收门店", "温度记录", "电子签名"], fieldSample: ["DR-2026-0001", "王强", "沪B-8K21", "北京朝阳路 02 号", "同仁堂朝阳门店", "4.8°C 持续 35 分钟", "已签名"], steps: ["接单", "到仓签到", "门店签收", "回传凭证"], actions: ["上传照片", "采集签名", "回传定位"], exceptions: ["定位漂移", "签收照片缺失"], stats: ["待接单", "在途", "已签收", "异常回传", "定位在线"], samples: ["DR-2026-0001", "司机王强", "沪B-8K21"], },
  H1: { name: "权限与多租户", primaryObject: "身份会话", columns: ["租户/账号", "角色/API Key", "授权范围", "过期时间", "风险", "状态"], rowSample: ["tenant-gsp-a / u001", "role_quality", "仓储 / 质量 / 报表", "2026-12-31 23:59", "低", ""], fields: ["租户", "账号", "角色", "权限域", "Token TTL", "设备指纹", "审计事件"], fieldSample: ["tenant-gsp-a", "u001 张三", "role_quality", "warehouse:* / quality:read", "8 小时", "DEV-X9F2A", "AUD-H1-0001"], steps: ["选择租户", "身份认证", "签发凭证", "进入工作台"], actions: ["强制登出", "轮换密钥", "冻结会话"], exceptions: ["租户不匹配", "Token 过期"], stats: ["在线会话", "高危权限", "过期凭证", "今日登录", "MFA 覆盖"], samples: ["tenant-gsp-a", "u001 张三", "role_quality"], },
  H2: { name: "审计追踪", primaryObject: "审计事件", columns: ["审计事件", "操作人", "资源", "哈希链", "保留策略", "状态"], rowSample: ["AUD-2026-0001", "u001 张三", "inventory.move INV-0001", "sha256:9f2a3c…", "≥5 年保留", ""], fields: ["事件 ID", "操作人", "资源类型", "前后差异", "哈希摘要", "保留年限", "归档分区"], fieldSample: ["AUD-2026-0001", "u001 张三", "inventory.move", "状态: 待处理 → 已完成", "sha256:9f2a3c…", "≥5 年", "分区策略待冻结"], steps: ["接收事件", "追加写入", "链路校验", "归档留存"], actions: ["导出证据", "校验哈希", "启动归档"], exceptions: ["链路断点", "归档失败"], stats: ["今日事件", "待归档", "链路异常", "查询命中", "留存覆盖"], samples: ["AUD-2026-0001", "inventory.move", "sha256:9f2a"], },
  H3: { name: "跨端契约", primaryObject: "API 契约", columns: ["API 路径", "方法", "Schema 版本", "示例", "权限", "状态"], rowSample: ["/api/v1/inbound/asn", "GET", "v2026.05", "已附 sample.json", "Scope: inbound:read", ""], fields: ["接口路径", "HTTP 方法", "Schema 版本", "客户端端", "鉴权 Scope", "示例响应", "发布版本"], fieldSample: ["/api/v1/inbound/asn", "GET", "v2026.05", "PC / PDA / H5", "inbound:read", "已附 sample.json", "v2026.05.01"], steps: ["读取 OpenAPI", "校验 Schema", "生成示例", "发布文档"], actions: ["重新生成", "复制 curl", "下载契约"], exceptions: ["Schema 不兼容", "示例缺失"], stats: ["接口数", "Schema 变更", "示例覆盖", "鉴权覆盖", "发布状态"], samples: ["/api/v1/inbound/asn", "GET", "v2026.05"], },
  H4: { name: "企业微信", primaryObject: "通知模板", columns: ["通知模板", "渠道", "接收角色", "触发事件", "发送窗口", "状态"], rowSample: ["H4-TPL-001", "企业微信", "质量负责人", "冷链超温告警", "07:00-22:00", ""], fields: ["模板编码", "渠道", "接收角色", "变量清单", "触发事件", "发送窗口", "失败重试"], fieldSample: ["H4-TPL-001", "企业微信", "质量负责人", "{batch}/{temp}/{location}", "冷链超温告警", "07:00-22:00", "3 次 / 30 秒"], steps: ["选择模板", "绑定事件", "预览消息", "发送记录"], actions: ["测试发送", "启用模板", "查看回执"], exceptions: ["变量缺失", "企微限流"], stats: ["启用模板", "今日发送", "失败重试", "待确认", "回执率"], samples: ["H4-TPL-001", "企业微信", "质量负责人"], },
  H5: { name: "快递协同", primaryObject: "快递运单", columns: ["快递商/运单", "面单模板", "收件方", "发货波次", "轨迹节点", "状态"], rowSample: ["顺丰 SF-20260524-001", "冷链面单 v3", "同仁堂朝阳门店", "WAVE-051", "已揽收", ""], fields: ["快递商", "运单号", "面单模板", "收件门店", "发货波次", "轨迹节点", "打印机"], fieldSample: ["顺丰冷运", "SF-20260524-001", "冷链面单 v3", "同仁堂朝阳门店", "WAVE-051", "已揽收 09:42", "PRT-DOCK-02"], steps: ["选择快递商", "生成运单", "打印面单", "跟踪轨迹"], actions: ["重新打印", "同步轨迹", "取消运单"], exceptions: ["面单失败", "轨迹延迟"], stats: ["待打印", "已揽收", "轨迹异常", "今日运单", "模板覆盖"], samples: ["SF-20260524-001", "冷链面单", "同仁堂门店"], },
  M1: { name: "基础档案", primaryObject: "主数据档案", columns: ["档案编码", "名称/资质", "货主", "控制属性", "有效期", "状态"], rowSample: ["ITEM-0001", "葡萄糖注射液 / GSP 合格证", "国药控股北京", "冷藏 2~8°C", "2026-12-31", ""], fields: ["档案编码", "档案名称", "货主", "资质证照", "存储条件", "有效期", "启停状态"], fieldSample: ["ITEM-0001", "葡萄糖注射液", "国药控股北京", "GSP 合格证 GSP-2026-A", "冷藏 2~8°C", "2026-12-31", "启用"], steps: ["录入档案", "校验证照", "配置控制属性", "发布生效"], actions: ["新增档案", "停用", "导入校验"], exceptions: ["证照过期", "编码重复"], stats: ["启用档案", "待复核", "证照临期", "导入失败", "字段完整"], samples: ["ITEM-0001", "葡萄糖注射液", "冷藏"], },
  M2: { name: "采购入库", primaryObject: "入库任务", columns: ["ASN/收货任务", "供应商", "商品/批号", "验收结果", "上架库位", "状态"], rowSample: ["ASN-2026-0001", "国药控股北京", "葡萄糖注射液 / 20260301A", "合格", "A01-02-03", ""], fields: ["ASN 单号", "供应商", "商品编码", "生产批号", "效期", "验收结论", "目标库位"], fieldSample: ["ASN-2026-0001", "国药控股北京", "ITEM-0001", "20260301A", "2026-12-31", "合格", "A01-02-03"], steps: ["接收 ASN", "PDA 验收", "双人签字", "上架确认"], actions: ["生成收货任务", "拒收", "打印标签"], exceptions: ["批号不符", "温控缺失"], stats: ["待收货", "验收中", "待上架", "拒收", "准时率"], samples: ["ASN-2026-0001", "国药控股北京", "20260301A"], },
  M3: { name: "库存与质量", primaryObject: "库存对象", columns: ["库存对象", "库位", "批号/效期", "可用数量", "库存状态", "状态"], rowSample: ["INV-0001 葡萄糖注射液", "A01-02-03", "20260301A / 2026-12-31", "128 盒", "合格可用", ""], fields: ["库存 ID", "库位", "批号", "效期", "可用数量", "冻结数量", "质量状态"], fieldSample: ["INV-0001", "A01-02-03", "20260301A", "2026-12-31", "128 盒", "0 盒", "合格"], steps: ["选择库存", "核对批号", "更新状态", "写入台账"], actions: ["冻结库存", "盘点确认", "转移库位"], exceptions: ["账实不符", "效期预警"], stats: ["可用库存", "冻结批次", "近效期", "盘点差异", "状态准确"], samples: ["INV-0001", "A01-02-03", "20260301A"], },
  M4: { name: "销售出库", primaryObject: "出库订单", columns: ["出库订单", "客户/门店", "波次/路径", "拣选进度", "复核/交接", "状态"], rowSample: ["SO-2026-0001", "北京协和药房", "WAVE-051 / 路径 P-A", "32 / 64 行", "复核 u002", ""], fields: ["出库单号", "客户/门店", "波次号", "拣选库位", "复核人", "随货同行单", "交接状态"], fieldSample: ["SO-2026-0001", "北京协和药房", "WAVE-051", "A01-02-03", "u002 李四", "TXS-2026-0001", "已交接"], steps: ["释放波次", "PDA 拣选", "出库复核", "交接发运"], actions: ["释放拣选", "生成同行单", "异常登记"], exceptions: ["拣选短缺", "复核不符"], stats: ["待拣选", "复核中", "待发运", "异常单", "准时出库"], samples: ["SO-2026-0001", "北京协和药房", "WAVE-051"], },
  M5: { name: "冷链集成", primaryObject: "冷链数据", columns: ["冷链设备", "采集点", "温度区间", "货箱/批次", "断点", "状态"], rowSample: ["TEMP-DEV-01", "冷藏库 #1", "2~8°C", "BOX-CC-001 / 20260301A", "0 次", ""], fields: ["设备编号", "采集时间", "温度", "湿度", "货箱编号", "关联批号", "超温时长"], fieldSample: ["TEMP-DEV-01", "2026-05-24 09:42", "4.8°C", "55%", "BOX-CC-001", "20260301A", "0 分钟"], steps: ["接收温度", "识别阈值", "关联业务", "生成告警"], actions: ["查看曲线", "创建质量联系单", "导出记录"], exceptions: ["采集断点", "超温未响应"], stats: ["在线设备", "超温次数", "断点记录", "关联批次", "回传延迟"], samples: ["TEMP-DEV-01", "4.8 C", "BOX-CC-001"], },
  M6: { name: "GSP 报表", primaryObject: "报表/台账", columns: ["报表/台账", "周期", "数据源", "校验项", "导出格式", "状态"], rowSample: ["RPT-GSP-001 库存台账", "2026-05", "WMS / ERP", "数量平衡 / 批号一致", "Excel / PDF", ""], fields: ["报表编码", "统计周期", "数据源", "过滤条件", "校验规则", "导出格式", "订阅人"], fieldSample: ["RPT-GSP-001", "2026-05", "WMS / ERP", "货主=国药控股", "数量平衡 / 批号一致", "Excel / PDF", "u001 张三"], steps: ["选择报表", "汇总数据", "校验口径", "导出归档"], actions: ["生成报表", "订阅", "导出 Excel"], exceptions: ["数据缺口", "口径不一致"], stats: ["报表数", "待生成", "订阅任务", "校验失败", "归档完成"], samples: ["RPT-GSP-001", "2026-05", "库存台账"], },
  M8: { name: "连锁药店", primaryObject: "门店补货", columns: ["门店/补货单", "需求来源", "配送仓", "截止时间", "缺口", "状态"], rowSample: ["STORE-018 / RP-2026-0001", "门店补货", "中央仓 W01", "2026-05-25 18:00", "12 SKU", ""], fields: ["门店编码", "补货单", "需求来源", "配送仓", "缺口数量", "到货截止", "签收方式"], fieldSample: ["STORE-018", "RP-2026-0001", "门店补货", "中央仓 W01", "12 SKU", "2026-05-25 18:00", "门店扫码签收"], steps: ["汇总需求", "生成补货单", "仓库拣配", "门店签收"], actions: ["生成补货", "调整缺口", "确认签收"], exceptions: ["门店拒收", "缺货替代"], stats: ["待补货", "缺口 SKU", "已发运", "签收异常", "满足率"], samples: ["STORE-018", "RP-2026-0001", "中央仓"], },
  M9: { name: "3PL 计费", primaryObject: "计费账单", columns: ["计费合同", "费用项", "客户", "账期", "对账差异", "状态"], rowSample: ["BILL-2026-05", "仓储费", "3PL 客户 A", "2026-05", "-128 元", ""], fields: ["合同编号", "费用项", "客户", "账期", "计费数量", "单价", "对账差异"], fieldSample: ["CON-2026-0001", "仓储费", "3PL 客户 A", "2026-05", "1280 托·天", "5.6 元/托·天", "-128 元"], steps: ["读取作业量", "套用合同", "生成账单", "对账确认"], actions: ["重算费用", "锁定账单", "导出明细"], exceptions: ["合同缺失", "对账差异"], stats: ["待计费", "账单金额", "对账差异", "已锁定", "合同覆盖"], samples: ["BILL-2026-05", "仓储费", "3PL 客户 A"], },
  M10: { name: "运输协同", primaryObject: "在途任务", columns: ["运单/在途任务", "线路", "车辆/容器", "温控状态", "到达预估", "状态"], rowSample: ["TMS-2026-0001", "北京-天津", "京A-6H58 / BOX-CC-001", "5.1°C 正常", "2026-05-24 14:30", ""], fields: ["运单号", "线路", "车牌号", "容器编号", "实时温度", "ETA", "司机回传"], fieldSample: ["TMS-2026-0001", "北京-天津", "京A-6H58", "BOX-CC-001", "5.1°C", "2026-05-24 14:30", "已回传"], steps: ["装车发运", "在途监控", "到店签收", "回传温控"], actions: ["查看地图", "联系司机", "创建超温告警"], exceptions: ["在途超温", "ETA 延误"], stats: ["在途车辆", "超温告警", "已签收", "延误任务", "定位在线"], samples: ["TMS-2026-0001", "北京-天津", "5.1 C"], },
  MPM: { name: "参数对照", primaryObject: "字段映射", columns: ["参数映射", "源字段", "目标字段", "转换规则", "生效范围", "状态"], rowSample: ["MAP-ERP-WMS-001", "ERP.WHSE", "warehouse_id", "前缀去除 / 转大写", "tenant-gsp-a", ""], fields: ["映射编码", "源系统", "源字段", "目标字段", "转换规则", "生效租户", "失败队列"], fieldSample: ["MAP-ERP-WMS-001", "ERP", "WHSE", "warehouse_id", "前缀去除 / 转大写", "tenant-gsp-a", "0 条"], steps: ["选择源字段", "配置映射", "试跑样本", "发布生效"], actions: ["运行试算", "发布映射", "查看失败队列"], exceptions: ["字段缺失", "转换失败"], stats: ["启用映射", "失败队列", "试算通过", "待发布", "覆盖系统"], samples: ["MAP-ERP-WMS-001", "ERP.WHSE", "warehouse_id"], },
  PK: { name: "包装站", primaryObject: "装箱任务", columns: ["装箱任务", "工作站", "箱规/重量", "打印模板", "质控结果", "状态"], rowSample: ["PK-2026-0001", "PACK-ST-02", "L 箱 / 12.4 kg", "PRT-OUT-A", "通过", ""], fields: ["装箱任务", "工作站", "箱号", "重量", "箱规", "打印模板", "复核结果"], fieldSample: ["PK-2026-0001", "PACK-ST-02", "BOX-PK-0001", "12.4 kg", "L (40×30×25 cm)", "PRT-OUT-A", "通过"], steps: ["扫描订单", "装箱称重", "打印标签", "质控交接"], actions: ["称重", "打印箱标", "重开箱"], exceptions: ["重量超差", "打印失败"], stats: ["待装箱", "称重异常", "已打印", "质控拦截", "效率"], samples: ["PK-2026-0001", "PACK-ST-02", "12.4kg"], },
  QL: { name: "质量联系单", primaryObject: "质量联系单", columns: ["联系单", "问题类型", "关联业务", "质量处理人", "审批状态", "状态"], rowSample: ["QL-2026-0001", "冷链超温", "ASN-2026-0001 / 20260301A", "u001 张三", "审批中", ""], fields: ["联系单号", "问题类型", "关联单据", "关联批号", "质量处理人", "审批链", "处理结论"], fieldSample: ["QL-2026-0001", "冷链超温", "ASN-2026-0001", "20260301A", "u001 张三", "u002 → u003", "隔离待复检"], steps: ["登记问题", "质量判定", "业务联动", "关闭归档"], actions: ["发起审批", "隔离库存", "关闭联系单"], exceptions: ["证据缺失", "审批超时"], stats: ["待处理", "审批中", "已隔离", "今日关闭", "超时"], samples: ["QL-2026-0001", "冷链超温", "ASN-2026-0001"], },
  RC: { name: "对账", primaryObject: "对账单", columns: ["对账单", "来源系统", "差异类型", "金额/数量", "处理结论", "状态"], rowSample: ["RC-2026-05", "ERP", "数量差异", "+8 / -8 盒", "调整库存", ""], fields: ["对账单号", "来源系统", "业务期间", "差异类型", "差异数量", "差异金额", "处理结论"], fieldSample: ["RC-2026-05", "ERP", "2026-05", "数量差异", "8 盒", "128 元", "调整库存"], steps: ["拉取来源", "比对差异", "人工确认", "回写结论"], actions: ["重新比对", "标记已处理", "导出差异"], exceptions: ["来源缺失", "金额不平"], stats: ["待对账", "差异项", "已处理", "未平金额", "自动匹配"], samples: ["RC-2026-05", "ERP", "数量差异"], },
  RP: { name: "补货", primaryObject: "补货建议", columns: ["补货建议", "触发原因", "来源库位", "目标库位", "差额", "状态"], rowSample: ["RP-2026-0001", "低于安全库存", "B02-04-01", "A01-02-03", "+128 盒", ""], fields: ["建议单", "触发原因", "SKU", "来源库位", "目标库位", "建议数量", "任务状态"], fieldSample: ["RP-2026-0001", "低于安全库存", "ITEM-0001", "B02-04-01", "A01-02-03", "128 盒", "待释放"], steps: ["计算缺口", "生成建议", "释放任务", "PDA 执行"], actions: ["释放补货", "调整数量", "取消建议"], exceptions: ["来源不足", "目标满位"], stats: ["待释放", "执行中", "缺口 SKU", "取消建议", "满足率"], samples: ["RP-2026-0001", "低于安全库存", "A01-01"], },
  SA: { name: "报损报溢", primaryObject: "库存调整单", columns: ["调整单", "调整类型", "商品/批号", "差异数量", "审批人", "状态"], rowSample: ["SA-2026-0001", "报损", "盐酸吗啡片 / 20260301A", "-8 盒", "u003 王五", ""], fields: ["调整单号", "调整类型", "商品", "批号", "差异数量", "原因", "审批人"], fieldSample: ["SA-2026-0001", "报损", "盐酸吗啡片", "20260301A", "8 盒", "破损不可销", "u003 王五"], steps: ["登记差异", "质量确认", "审批", "更新库存"], actions: ["提交审批", "拍照取证", "冲销调整"], exceptions: ["审批拒绝", "证据不足"], stats: ["待审批", "今日报损", "今日报溢", "隔离库存", "审计覆盖"], samples: ["SA-2026-0001", "报损", "盐酸吗啡片"], },
  ST: { name: "门店端", primaryObject: "门店作业", columns: ["门店订单", "门店", "商品组合", "履约方式", "签收/退货", "状态"], rowSample: ["STORE-ORD-001", "同仁堂朝阳门店", "葡萄糖 + 麻精专柜", "冷链直送", "已签收", ""], fields: ["门店订单", "门店", "收货人", "商品组合", "配送温度", "签收凭证", "退货原因"], fieldSample: ["STORE-ORD-001", "同仁堂朝阳门店", "李丽", "葡萄糖 + 麻精专柜", "2~8°C", "签收照片 + 签名", "无"], steps: ["查看订单", "扫码签收", "拍照确认", "回传结果"], actions: ["确认签收", "申请退货", "上传凭证"], exceptions: ["少货", "温控异常"], stats: ["待签收", "退货申请", "在途订单", "异常凭证", "签收率"], samples: ["STORE-ORD-001", "同仁堂门店", "签收照片"], },
  TC: { name: "追溯码", primaryObject: "追溯码任务", columns: ["追溯码任务", "码段/箱码", "业务单据", "上传批次", "回执", "状态"], rowSample: ["TC-AB12-CD34", "BOX-TC-001", "M4-2026-0001", "UP-2026-0524", "已回执", ""], fields: ["追溯码", "码段", "业务单据", "箱码", "上传批次", "监管回执", "反向追溯"], fieldSample: ["TC-AB12-CD34", "AB12-CD34-EF56", "M4-2026-0001", "BOX-TC-001", "UP-2026-0524", "已回执 200 OK", "已绑定"], steps: ["扫描追溯码", "绑定业务", "上传码段", "处理回执"], actions: ["补扫", "重新上传", "反向追溯"], exceptions: ["码重复", "回执失败"], stats: ["待上传", "上传成功", "回执失败", "重复码", "追溯覆盖"], samples: ["TC-AB12-CD34", "BOX-TC-001", "M4-2026-0001"], },
  TE: { name: "任务引擎", primaryObject: "任务规则", columns: ["任务规则", "工作池", "分配对象", "优先级", "释放条件", "状态"], rowSample: ["TE-RULE-001", "入库工作池", "u001 张三", "P1", "ASN 验收完成", ""], fields: ["任务规则", "工作池", "作业类型", "优先级", "分配策略", "释放条件", "超时策略"], fieldSample: ["TE-RULE-001", "入库工作池", "上架", "P1", "按工号轮询", "ASN 验收完成", "30 分钟超时升级"], steps: ["定义规则", "生成任务", "分配执行", "回收绩效"], actions: ["释放任务", "重新分配", "暂停规则"], exceptions: ["无可用人员", "任务超时"], stats: ["待分配", "执行中", "超时任务", "今日完成", "人效"], samples: ["TE-RULE-001", "入库工作池", "优先级 P1"], },
  VR: { name: "规则引擎", primaryObject: "校验规则", columns: ["校验规则", "规则集", "测试样本", "命中动作", "发布版本", "状态"], rowSample: ["VR-COLD-001", "入库校验", "已通过 12/12", "create_alert", "v2026.05.01", ""], fields: ["规则编码", "规则集", "适用模块", "条件表达式", "命中动作", "沙箱样本", "发布版本"], fieldSample: ["VR-COLD-001", "入库校验", "M2 / M5", "temp > 8°C && hold > 30min", "create_alert", "12 条样本通过", "v2026.05.01"], steps: ["编写规则", "沙箱测试", "审批发布", "命中监控"], actions: ["运行沙箱", "发布版本", "回滚规则"], exceptions: ["规则冲突", "沙箱失败"], stats: ["启用规则", "待发布", "今日命中", "冲突规则", "沙箱通过"], samples: ["VR-COLD-001", "入库校验", "create_alert"], },
} satisfies Record<string, ModuleBlueprint>;
function keywordProfile(spec: MatrixPrototypeSpec): KeywordProfile {
  const text = `${spec.title}${spec.reason}`;
  if (/登录|token 签发/i.test(text)) { return { primaryObject: "登录会话", fields: ["登录端", "工牌码", "MFA 状态", "登录 IP"], fieldSample: ["PC 管理端", "WORK-12345", "已校验", "10.42.18.9"], steps: ["选择三角色入口", "扫码或账号认证", "签发访问 Token", "记录登录审计"], actions: ["切换租户", "重新认证"], exceptions: ["连续失败锁定", "租户角色不匹配"], }; }
  if (/API Key|密钥/i.test(text)) { return { primaryObject: "API Key", fields: ["Key 名称", "Secret 一次性展示", "IP 白名单", "轮换周期"], fieldSample: ["扫码秤接入 Key", "kg_••••sec_5f", "10.42.0.0/16", "90 天"], steps: ["创建 Key", "展示 Secret", "配置 Scope", "到期轮换"], actions: ["立即轮换", "吊销 Key"], exceptions: ["Secret 已隐藏", "Scope 越权"], }; }
  if (/审计|归档|保留/.test(text)) { return { primaryObject: "审计证据", fields: ["归档批次", "保留策略", "哈希链状态", "查询条件"], fieldSample: ["审计分区待冻结", "≥5 年保留", "链路完整", "操作人=u001"], steps: ["筛选事件", "校验链路", "导出证据", "归档封存"], actions: ["校验完整性", "生成证据包"], exceptions: ["归档分区缺失", "哈希校验失败"], }; }
  if (/容器|LPN/i.test(text)) { const isRecovery = /回收|追踪/.test(text); return { primaryObject: isRecovery ? "容器回收任务" : "LPN 容器", columns: ["LPN/容器", "容器类型", "绑定对象", "库位/车辆", "回收状态", "状态"], rowSample: ["LPN-2026-0001", "周转箱 (60L)", "葡萄糖 / 20260301A", "A01-02-03", "在用", ""], fields: ["LPN 号", "容器类型", "绑定商品/批号", "当前库位", "绑定状态", "回收状态", "最后扫码人"], fieldSample: ["LPN-2026-0001", "周转箱 (60L)", "葡萄糖 / 20260301A", "A01-02-03", "已绑定", "在用", "u001 张三"], steps: isRecovery ? ["扫描容器码", "核对门店/车辆", "确认回收状态", "生成回收记录"] : ["扫描 LPN", "绑定商品/批号", "核对库位", "解绑或转移"], actions: isRecovery ? ["确认回收", "登记丢失", "导出追踪"] : ["绑定 LPN", "解绑容器", "打印容器标签"], exceptions: ["LPN 已绑定", "容器丢失", "状态不允许解绑", "重复扫码"], statValues: ["在用容器", "待解绑", "待回收", "异常 LPN", "扫码完成率"], }; }
  if (/客户|门店档案/.test(text)) { return { primaryObject: "客户/门店档案", fields: ["客户编码", "门店编码", "层级关系", "配送地址", "经营资质", "启停状态"], fieldSample: ["CUST-001 同仁堂连锁", "STORE-018 朝阳门店", "总部 → 朝阳大区 → 朝阳门店", "北京朝阳区朝外大街", "GSP 2026-A", "启用"], steps: ["维护客户", "绑定门店层级", "校验资质", "发布档案"], actions: ["新增客户", "导入门店", "停用档案"], exceptions: ["层级冲突", "资质过期"], }; }
  if (/多货主|数据隔离/.test(text)) { return { primaryObject: "货主隔离策略", fields: ["货主", "租户", "数据隔离范围", "默认仓", "权限边界", "跨货主规则"], fieldSample: ["国药控股北京", "tenant-gsp-a", "仓储 / 质量 / 报表", "中央仓 W01", "owner:gsp-a", "禁止跨货主访问"], steps: ["选择货主", "校验租户隔离", "切换数据范围", "写入审计"], actions: ["切换货主", "校验隔离", "导出权限"], exceptions: ["跨货主越权", "租户不匹配"], }; }
  if (/特殊药品|麻精|放射|血液制品/.test(text)) { return { primaryObject: "特殊药品分类", fields: ["特殊药品类别", "麻精标识", "放射药品标识", "血液制品标识", "双人策略", "专用台账"], fieldSample: ["麻醉药品", "是 (麻 1)", "否", "否", "扫码 + 双签", "RPT-MNG-001"], steps: ["维护分类", "绑定管控规则", "启用双人复核", "同步台账"], actions: ["新增分类", "绑定规则", "停用分类"], exceptions: ["分类重复", "缺少双人策略"], }; }
  if (/追踪|跟踪/.test(text)) { return { primaryObject: "追踪时间线", columns: ["追踪对象", "当前位置", "上一节点", "事件时间", "责任人", "状态"], rowSample: ["BOX-TC-001 周转箱", "同仁堂朝阳门店", "顺丰中转站", "2026-05-24 09:42", "u001 张三", ""], fields: ["追踪对象", "事件节点", "当前位置", "上一节点", "责任人", "时间线状态"], fieldSample: ["BOX-TC-001 周转箱", "门店签收", "同仁堂朝阳门店", "顺丰中转站", "u001 张三", "已闭环"], steps: ["选择追踪对象", "加载事件时间线", "定位当前节点", "导出追踪证据"], actions: ["刷新追踪", "导出时间线"], exceptions: ["事件断点", "节点缺失"], statValues: ["追踪对象", "异常节点", "今日事件", "断点记录", "覆盖率"], }; }
  if (/养护/.test(text)) { return { primaryObject: "在库养护任务", fields: ["养护计划", "养护项目", "养护结果", "库位", "批号", "异常结论"], fieldSample: ["MNT-2026-Q2", "外观 / 温度 / 湿度", "通过", "A01-02-03", "20260301A", "无异常"], steps: ["生成养护计划", "PDA 记录养护", "质量复核", "归档养护记录"], actions: ["提交养护", "拍照取证", "生成质量联系单"], exceptions: ["养护逾期", "异常未闭环"], }; }
  if (/ABC/.test(text)) { return { primaryObject: "ABC 分类结果", fields: ["ABC 类别", "分类规则", "周转率", "库存金额", "复核状态"], fieldSample: ["A 类高周转", "近 90 天周转 ≥ 12 次", "14.6 次/年", "128 万元", "已复核"], steps: ["计算分类", "复核规则", "发布结果", "同步策略"], actions: ["重新计算", "调整分类"], exceptions: ["规则缺失", "分类冲突"], }; }
  if (/合并|拆单/.test(text)) { return { primaryObject: "合并/拆单方案", fields: ["合并批次", "拆单规则", "订单组合", "承运限制", "预览结果"], fieldSample: ["WAVE-051", "按温区 + 客户合并", "SO-2026-0001 + SO-2026-0002", "顺丰冷运 ≤ 50 kg", "可执行"], steps: ["选择订单", "预览合并", "执行拆单", "同步发货"], actions: ["合并发货", "拆分订单"], exceptions: ["承运限制冲突", "库存不足"], }; }
  if (/越库|Cross-Docking/i.test(text)) { return { primaryObject: "越库任务", fields: ["越库单", "来源到货", "目标门店", "交叉月台", "转运状态"], fieldSample: ["XD-2026-0001", "ASN-2026-0001", "STORE-018 朝阳门店", "月台 D03", "转运中"], steps: ["识别越库", "扫码交接", "月台转运", "确认出库"], actions: ["释放越库", "扫码交接"], exceptions: ["到货短缺", "目标门店不匹配"], }; }
  if (/退货/.test(text)) { return { primaryObject: "退货处理单", fields: ["退货单号", "退货原因", "原销售单", "批号", "质量结论", "入库去向"], fieldSample: ["RTN-2026-0001", "门店多送 / 临期", "SO-2026-0001", "20260301A", "合格可销", "回库 A01-02-03"], steps: ["登记退货", "PDA 验收", "质量判定", "生成入库/拒收"], actions: ["确认退货", "发起质量处理"], exceptions: ["批号不符", "退货超期"], }; }
  if (/PIX|交易类型|三码/.test(text)) { return { primaryObject: "PIX 交易类型字典", fields: ["交易类型", "PIX 三码", "来源系统", "目标动作", "映射状态"], fieldSample: ["PIX-IN-001 验收入库", "20-30-10", "ERP", "create_inbound", "已发布"], steps: ["维护字典", "绑定 PIX 三码", "试算映射", "发布生效"], actions: ["新增交易类型", "试算映射"], exceptions: ["三码冲突", "交易类型缺失"], }; }
  if (/码库|大中小码/.test(text)) { return { primaryObject: "追溯码码库", fields: ["大码", "中码", "小码", "码段", "层级关系", "占用状态"], fieldSample: ["BOX-AB12", "AB12-CD34", "AB12-CD34-EF56", "EF56-0001 ~ EF56-9999", "1 : 10 : 100", "已占用"], steps: ["导入码段", "建立大中小码层级", "绑定业务单据", "校验码库状态"], actions: ["导入码库", "释放码段"], exceptions: ["码重复", "层级断链"], }; }
  if (/Put-to-Light/i.test(text)) { return { primaryObject: "装箱灯光任务", fields: ["装箱任务", "Put-to-Light 格口", "箱号", "SKU", "应放数量", "实放数量"], fieldSample: ["PK-2026-0001", "格口 #07", "BOX-PK-0001", "ITEM-0001", "12 盒", "12 盒"], steps: ["扫描订单", "点亮格口", "装箱确认", "关闭箱号"], actions: ["点亮格口", "重开箱"], exceptions: ["格口异常", "数量不符"], }; }
  if (/保温箱/.test(text)) { return { primaryObject: "保温箱配置", fields: ["保温箱规格", "温区", "蓄冷介质", "有效时长", "适用线路"], fieldSample: ["L 保温箱 (40 L)", "2~8°C", "蓄冷剂 ×4", "24 小时", "北京-天津"], steps: ["维护箱规", "绑定温区", "校验时长", "发布配置"], actions: ["新增保温箱", "停用配置"], exceptions: ["温区不匹配", "时长不足"], }; }
  if (/打印|面单|随货同行单|标签/.test(text)) { return { primaryObject: "打印任务", fields: ["打印模板", "打印机", "重打原因", "预览页数"], fieldSample: ["PRT-OUT-A", "PRT-DOCK-02", "首次打印", "1 / 1 页"], steps: ["选择模板", "预览版式", "发送打印", "记录回执"], actions: ["重新打印", "切换模板"], exceptions: ["打印机离线", "模板变量缺失"], }; }
  if (/盘点|库存/.test(text)) { return { primaryObject: "库存盘点对象", fields: ["账面数量", "实盘数量", "差异原因", "盘点轮次"], fieldSample: ["128 盒", "120 盒", "破损 -8 盒", "第 1 轮"], steps: ["锁定范围", "扫码盘点", "差异复核", "生成调整"], actions: ["确认差异", "复盘"], exceptions: ["重复扫描", "库存锁定冲突"], }; }
  if (/拣选|复核|出库|交接/.test(text)) { return { primaryObject: "出库作业任务", fields: ["拣选路径", "周转箱", "复核结论", "交接人"], fieldSample: ["路径 P-A", "LPN-2026-0001", "通过", "u002 李四"], steps: ["领取任务", "按路径拣选", "复核批号", "交接发运"], actions: ["登记短拣", "完成复核"], exceptions: ["库位无货", "批号不一致"], }; }
  if (/冷链|温控|温度|超温/.test(text)) { return { primaryObject: "温控记录", fields: ["温度上限", "温度下限", "采集频率", "超温时长"], fieldSample: ["8°C", "2°C", "1 分钟 / 次", "0 分钟"], steps: ["采集温度", "阈值判断", "联动批次", "生成告警"], actions: ["查看曲线", "发起质量处理"], exceptions: ["设备离线", "超温未闭环"], }; }
  if (/规则|策略|配置|映射|参数/.test(text)) { return { primaryObject: "配置规则", fields: ["条件表达式", "命中动作", "生效范围", "灰度状态"], fieldSample: ["temp > 8°C && hold > 30min", "create_alert", "tenant-gsp-a / 全模块", "灰度 30%"], steps: ["编辑规则", "沙箱验证", "审批发布", "监控命中"], actions: ["运行沙箱", "发布规则"], exceptions: ["规则冲突", "灰度失败"], }; }
  if (/签收|司机|门店|H5/.test(text)) { return { primaryObject: "移动签收任务", fields: ["定位", "照片凭证", "电子签名", "签收备注"], fieldSample: ["GPS 116.41,39.91", "2 张照片", "已签", "无异常"], steps: ["打开任务", "扫码核验", "采集凭证", "提交回传"], actions: ["拍照", "签名", "回传"], exceptions: ["定位异常", "凭证缺失"], }; }
  return {};
}
*/

import type { FieldRow, Step } from "@wms/ui";
import {
  isApproval,
  isCold,
  isKanban,
  isPrint,
  isRule,
  isScanHeavy,
} from "./prototype-classifiers";
import { MODULE_BLUEPRINTS } from "./prototype-blueprints";
import type { ModuleBlueprint } from "./prototype-blueprints";
import { keywordProfile } from "./prototype-keyword-profile";
import type { MatrixPrototypeSpec } from "./types";

export type LayoutKind = "table" | "kanban" | "print" | "temperature" | "rule";

export interface PrototypeColumn {
  key: string;
  header: string;
  align?: "left" | "center" | "right";
  mono?: boolean;
}

export interface PrototypeRow {
  id: string;
  status: string;
  [key: string]: string;
}

export interface MetricItem {
  label: string;
  value: string;
  hint?: string;
}

export interface StoryPrototypeModel {
  moduleName: string;
  primaryObject: string;
  searchPlaceholder: string;
  scanPlaceholder: string;
  lastScanned: string;
  layoutKind: LayoutKind;
  columns: PrototypeColumn[];
  rows: PrototypeRow[];
  stats: MetricItem[];
  filters: FieldRow[];
  fields: FieldRow[];
  steps: Step[];
  actions: string[];
  exceptions: string[];
  auditEvents: string[];
  before: Record<string, string>;
  after: Record<string, string>;
}

export { MODULE_BLUEPRINTS } from "./prototype-blueprints";

const STATUS_VALUES = ["待处理", "进行中", "异常", "待复核", "已完成", "已归档"];

export function buildStoryPrototypeModel(spec: MatrixPrototypeSpec): StoryPrototypeModel {
  const blueprint = MODULE_BLUEPRINTS[spec.moduleCode as keyof typeof MODULE_BLUEPRINTS] ?? MODULE_BLUEPRINTS.MPM;
  const keyword = keywordProfile(spec);

  const mergedColumnLabels = [...(keyword.columns ?? []), ...blueprint.columns];
  const columns = buildColumns(mergedColumnLabels);

  // 列头 → 示例值 映射：keyword 优先于 blueprint
  const rowSampleMap = new Map<string, string>();
  zipInto(rowSampleMap, blueprint.columns, blueprint.rowSample);
  if (keyword.columns && keyword.rowSample) {
    zipInto(rowSampleMap, keyword.columns, keyword.rowSample);
  }

  const mergedFieldLabels = dedupe([...(keyword.fields ?? []), ...blueprint.fields]);

  // 字段标签 → 示例值 映射：keyword 优先于 blueprint
  const fieldSampleMap = new Map<string, string>();
  zipInto(fieldSampleMap, blueprint.fields, blueprint.fieldSample);
  if (keyword.fields && keyword.fieldSample) {
    zipInto(fieldSampleMap, keyword.fields, keyword.fieldSample);
  }

  const fields = buildFields(spec, mergedFieldLabels, fieldSampleMap);
  const steps = buildSteps(dedupe([...(keyword.steps ?? []), ...blueprint.steps]));
  const primaryObject = keyword.primaryObject ?? blueprint.primaryObject;
  const layoutKind = chooseLayout(spec);

  return {
    moduleName: blueprint.name,
    primaryObject,
    searchPlaceholder: `${spec.storyId} / ${primaryObject} / ${blueprint.fields.slice(0, 3).join(" / ")}`,
    scanPlaceholder: buildScanPlaceholder(spec, primaryObject),
    lastScanned: `${spec.moduleCode}-${blueprint.samples[0]}`,
    layoutKind,
    columns,
    rows: buildRows(spec, columns, rowSampleMap),
    stats: buildStats(spec, blueprint, keyword.statValues ?? []),
    filters: buildFilters(spec, blueprint),
    fields,
    steps,
    actions: dedupe([...(keyword.actions ?? []), ...blueprint.actions]).slice(0, 5),
    exceptions: dedupe([...(keyword.exceptions ?? []), ...blueprint.exceptions]).slice(0, 4),
    auditEvents: [
      `${primaryObject} 创建`,
      `${blueprint.name}字段核对`,
      isApproval(spec) ? "审批链路推进" : "状态流转",
      "H2 append-only 写入",
    ],
    before: { 状态: "待处理", 处理人: "-", 审计号: "-" },
    after: { 状态: isApproval(spec) ? "待审批" : "进行中", 处理人: "u001", 审计号: `AUD-${spec.storyId}` },
  };
}

function chooseLayout(spec: MatrixPrototypeSpec): LayoutKind {
  if (isPrint(spec)) return "print";
  if (isCold(spec)) return "temperature";
  if (isRule(spec)) return "rule";
  if (isKanban(spec)) return "kanban";
  return "table";
}

function buildColumns(labels: string[]): PrototypeColumn[] {
  const unique = dedupe(labels).slice(0, 6);
  return unique.map((label, index) => ({
    key: index === unique.length - 1 ? "status" : `c${index}`,
    header: label,
    mono: /单|号|码|ID|Key|API|路径|哈希/.test(label),
    align: /数量|金额|差额|温度|SLA|进度/.test(label) ? "right" : "left",
  }));
}

/**
 * buildRows — 按"列头 → 示例值"映射逐列填充，列头/单元格值同源
 *
 * 不再使用关键字猜测：每个列头在 blueprint.rowSample（或 keyword.rowSample）中
 * 都有显式对应值。状态列由 STATUS_VALUES 接管。行偏移仅用于：
 *   1. 状态列在 STATUS_VALUES 中轮转
 *   2. ID 风格列（形如 PREFIX-NNNN）尾号按行递增，便于呈现多行差异
 *   3. 形如 20260301A 的批号在固定小池中轮转
 */
function buildRows(
  spec: MatrixPrototypeSpec,
  columns: PrototypeColumn[],
  rowSampleMap: Map<string, string>,
): PrototypeRow[] {
  return Array.from({ length: 6 }, (_, idx) => {
    const row: PrototypeRow = {
      id: `${spec.slug}-${idx + 1}`,
      status: STATUS_VALUES[idx % STATUS_VALUES.length],
    };
    columns.forEach((col) => {
      if (col.key === "status") return;
      const base = rowSampleMap.get(col.header) ?? "";
      row[col.key] = varyRowValue(base, col.header, idx);
    });
    return row;
  });
}

function buildStats(spec: MatrixPrototypeSpec, blueprint: ModuleBlueprint, extraValues: string[]): MetricItem[] {
  const labels = dedupe([...extraValues, ...blueprint.stats]).slice(0, 5);
  return labels.map((label, idx) => ({
    label,
    value: idx === 4 ? "100%" : String([18, 7, 2, 126][idx] ?? 42),
    hint: idx === 0 ? spec.storyId : `${blueprint.name} · H2 已记录`,
  }));
}

function buildFilters(spec: MatrixPrototypeSpec, blueprint: ModuleBlueprint): FieldRow[] {
  return [
    { label: "状态", value: isApproval(spec) ? "待审批 / 进行中" : "待处理 / 进行中" },
    { label: "业务域", value: blueprint.name },
    { label: "时间范围", value: "最近 7 天" },
  ];
}

/**
 * buildFields — 按"字段标签 → 示例值"映射填充，字段标签/字段值同源
 *
 * 第一行字段使用 storyId 作为编码占位（保留原有约定），其余字段从映射取值。
 */
function buildFields(
  spec: MatrixPrototypeSpec,
  labels: string[],
  fieldSampleMap: Map<string, string>,
): FieldRow[] {
  return labels.slice(0, 7).map((label, idx) => ({
    label,
    value: idx === 0 ? spec.storyId : fieldSampleMap.get(label) ?? "",
    required: idx < 4,
    autoFilled: idx === 0 || (isScanHeavy(spec) && idx < 3),
  }));
}

function buildSteps(labels: string[]): Step[] {
  return labels.slice(0, 5).map((label, idx) => ({
    label,
    description: idx === 0 ? "入口校验" : idx === labels.length - 1 ? "H2 审计" : "业务规则",
  }));
}

function buildScanPlaceholder(spec: MatrixPrototypeSpec, primaryObject: string) {
  if (spec.end === "h5") return `扫码 / 拍照 / 输入${primaryObject}`;
  if (isScanHeavy(spec)) return `扫描${primaryObject} / 批号 / 追溯码`;
  return `输入${primaryObject}编号或关键字`;
}

function dedupe(items: string[]) {
  return Array.from(new Set(items.filter(Boolean)));
}

/** 把 labels[i] → samples[i] 写入 map（已存在的键会被覆盖，便于 keyword 优先） */
function zipInto(map: Map<string, string>, labels: string[], samples: string[]) {
  const len = Math.min(labels.length, samples.length);
  for (let i = 0; i < len; i++) {
    const label = labels[i];
    const sample = samples[i];
    if (!label) continue;
    if (sample === undefined || sample === "") continue;
    map.set(label, sample);
  }
}

const BATCH_POOL = ["20260301A", "20260402B", "20260503C", "20260604D", "20260705E", "20260806F"];

/**
 * 按行偏移生成轻量行变化：
 *   - ID 列（形如 `PREFIX-0001`）：尾号 4 位按行索引递增
 *   - 批号列（形如 `20260301A`，独立出现）：在 BATCH_POOL 中轮转
 *   - 其他列：保持示例值原样（同列各行视觉一致即可）
 */
function varyRowValue(value: string, label: string, idx: number): string {
  if (!value) return "";
  // ID 风格：PREFIX-NNNN（尾部 4 位数字）
  const idMatch = value.match(/^(.*-)(\d{4})(?!.*\d)$/);
  if (idMatch && /单|号|码|编码|ID|Key/.test(label)) {
    return `${idMatch[1]}${String(idx + 1).padStart(4, "0")}`;
  }
  // 纯批号：20YYMMDDX
  if (/^20\d{6}[A-Z]$/.test(value)) {
    return BATCH_POOL[idx % BATCH_POOL.length];
  }
  return value;
}
