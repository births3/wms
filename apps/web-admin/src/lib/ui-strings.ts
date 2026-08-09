/**
 * 共享中文文案收敛模块：四维扫描 deferred 清单中跨文件语义一致的高/中/低频字面量。
 * 值逐字沿用各页面历史实现，替换后不改变渲染结果。
 * 按用途分组：列头/字段标签、按钮/操作、状态值、loading、筛选、错误兜底、枚举值映射。
 */

// ---------- 列头 / 字段标签 ----------

/** 审计列对：创建时间（46 文件最高频列头）。 */
export const COLUMN_CREATED_AT = "创建时间";

/** 审计列对：更新时间。 */
export const COLUMN_UPDATED_AT = "更新时间";

/** 通用状态列标题/字段标签。 */
export const COLUMN_STATUS = "状态";

/** 版本列/字段标签。 */
export const COLUMN_VERSION = "版本";

/** 商品编码列/字段标签（ERP 商品主数据核心标识）。 */
export const COLUMN_PRODUCT_CODE = "商品编码";

/** 批号列/字段标签。 */
export const COLUMN_BATCH_NO = "批号";

/** 仓库列/字段标签。 */
export const COLUMN_WAREHOUSE = "仓库";

/** 仓库 ID 字段标签（与 COLUMN_WAREHOUSE 是不同字面量）。 */
export const FIELD_WAREHOUSE_ID = "仓库 ID";

/** 单据类型列/字段标签（入库/出库单据均使用）。 */
export const COLUMN_DOCUMENT_TYPE = "单据类型";

/** 货主（owner）字段标签（ERP 身份隔离概念）。 */
export const COLUMN_OWNER = "货主";

/** 质量状态字段标签。 */
export const COLUMN_QUALITY_STATUS = "质量状态";

/** 事件类型列/字段标签。 */
export const COLUMN_EVENT_TYPE = "事件类型";

/** 车牌号字段标签（TMS/码头/入库）。 */
export const FIELD_PLATE_NO = "车牌号";

/** 规则名称列/字段标签（规则类页面通用）。 */
export const COLUMN_RULE_NAME = "规则名称";

/** 有效期字段标签（批次/证照/规则）。 */
export const FIELD_VALIDITY = "有效期";

/** 温区列/字段标签（温度分区概念跨库存/码头/出库/主数据）。 */
export const COLUMN_TEMP_ZONE = "温区";

/** 作用域（scope）字段标签。 */
export const FIELD_SCOPE = "作用域";

/** 关键字搜索框标签/占位（keyword 搜索）。 */
export const FIELD_KEYWORD = "关键字";

/** 统一社会信用代码（证照编号）字段标签，集中在 master-data feature。 */
export const FIELD_UNIFIED_SOCIAL_CREDIT_CODE = "统一社会信用代码";

// ---------- 按钮 / 操作 ----------

/** 新增/创建动作按钮 label（列表 createAction 等）。 */
export const BUTTON_ADD = "新增";

/** 列表页刷新按钮/操作标签。 */
export const BUTTON_REFRESH = "刷新";

/** 保存按钮（不含 "保存中..."）。 */
export const BUTTON_SAVE = "保存";

// ---------- 状态值 ----------

/** enabled 开关二态文案之一。 */
export const STATUS_ENABLED = "启用";

/** enabled 开关二态文案之一（与 STATUS_ENABLED 配对）。 */
export const STATUS_DISABLED = "停用";

/** 停用状态值（与开关文案 "停用" 是不同字面量）。 */
export const STATUS_DEACTIVATED = "已停用";

/** 取消状态值。 */
export const STATUS_CANCELLED = "已取消";

/** 草稿状态值。 */
export const STATUS_DRAFT = "草稿";

/** 发布状态值（打印域）。 */
export const STATUS_PUBLISHED = "已发布";

/** 完成状态值。 */
export const STATUS_COMPLETED = "已完成";

/** 待处理状态值。 */
export const STATUS_PENDING = "待处理";

/** 待录入状态占位值（详情行占位/出库列表）。 */
export const STATUS_PENDING_INPUT = "待录入";

// ---------- loading 文案 ----------

/** 表单保存按钮 loading 提示。 */
export const LOADING_SAVING = "保存中...";

/** 表单提交按钮 loading 提示（与 LOADING_SAVING 是不同字面量）。 */
export const LOADING_SUBMITTING = "提交中...";

/** 处理中 loading 文案。 */
export const LOADING_PROCESSING = "处理中...";

// ---------- 筛选 ----------

/** 筛选器全量选项（下拉/筛选 all 选项）。 */
export const FILTER_ALL = "全部";

// ---------- 错误兜底 ----------

/** API 鉴权失败兜底文案（errorText(err, fallback) 兜底）。 */
export const ERROR_AUTH_API_CHECK = "请检查鉴权和 API 服务";

/** 入库单未找到错误消息。 */
export const ERROR_INBOUND_ORDER_NOT_FOUND = "未找到对应入库单，请刷新列表后重试";

// ---------- 枚举值映射 ----------

/** 温区枚举值：常温（与 TEMP_COLD 同族）。 */
export const TEMP_AMBIENT = "常温";

/** 温区枚举值：冷藏（与 TEMP_AMBIENT 同族）。 */
export const TEMP_COLD = "冷藏";

/** 出库单据类型值：采购退货出库。 */
export const DOC_TYPE_PURCHASE_RETURN = "采购退货出库";
