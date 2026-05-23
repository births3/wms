# ADR-0023 业务报表方案选型（混合方案）

- 状态：Accepted
- 日期：2026-05-23
- 决策者：项目主人
- 相关：US-M6-001 / M6-002 / M6-003 / M6-004 / ADR-0021 高保真原型

## 背景

WMS 报表需求分两类，不能用同一种技术方案：

| 类型 | 业务特征 | 技术要求 |
|---|---|---|
| **A. GSP 法定报表** | 监管硬性规定，模板固定，字段固定，格式规范 | 强类型 / 数据签名 / 留 5 年 / 监管 EDI |
| **B. 业务自定义报表** | 运营自由查询，维度/指标灵活组合，分析探索 | 拖拽配置 / 透视分析 / 多种图表 / 模板共享 |

A 类必须由后端开发预定义实现；B 类做不到的话运营会绕开 WMS 用 Excel，反而失去数据治理。

## 选型对比

### 自研增强 M6Custom

| 维度 | 评估 |
|---|---|
| 开发周期 | 4-6 周（拖拽 + DSL + 图表 + 权限）|
| 灵活度 | 60-70%（pivot 复杂场景做不好）|
| 维护成本 | 高（图表库 / SQL 注入 / 大数据分页）|
| 数据安全 | ✓（所有逻辑在自家代码内）|
| 中文支持 | ✓ |

### Apache Superset

| 维度 | 评估 |
|---|---|
| 部署 | Docker（中等）|
| 灵活度 | 90%+（pivot / 多源 join / SQL Lab）|
| 嵌入难度 | 中（Guest token + iframe）|
| 中文 | 部分（需自己补译）|
| 学习成本 | 高（需要培训运营）|

### Metabase ★ 推荐

| 维度 | 评估 |
|---|---|
| 部署 | Docker（简单，1 行 docker run）|
| 灵活度 | 85%+（拖拽 + Question + Dashboard）|
| 嵌入难度 | 简单（iframe + JWT 签名 URL）|
| 中文 | ✓ 完整 |
| 学习成本 | 低（GUI 拖拽，无需懂 SQL）|
| 协议 | AGPLv3（自托管满足）|
| 行级权限 | ✓ Sandbox 功能 |

### 阿里 QuickBI / 帆软 FineBI

| 维度 | 评估 |
|---|---|
| 部署 | SaaS / 本地化部署 |
| 灵活度 | 95%+ |
| 中文 | ✓✓ |
| 商业许可 | 收费（按用户/CPU）|
| 数据合规 | SaaS 版数据出库（GSP 风险）|

## 决策

**采用 C 混合方案**：

```
┌──────────────────────────────────────────────────────┐
│ 业务报表能力分层                                       │
├──────────────────────────────────────────────────────┤
│ A. GSP 法定报表（M6-001）                              │
│    - WMS 后端实现：5 张固定模板                         │
│    - utoipa 强类型查询 → OpenAPI → 前端类型生成         │
│    - 数据签名 MD5 + 留 5 年 + 监管 EDI                   │
│    - 已实现：m6-purchase / sales / inventory / cold /   │
│      expiry（5 张独立 tab）                             │
│                                                        │
│ B. 业务自定义报表（M6-003）                             │
│    - Wave 5 部署 Metabase（Docker）                     │
│    - WMS PostgreSQL 配置只读账号给 Metabase             │
│    - 行级权限通过 Metabase Sandbox（货主隔离）           │
│    - WMS 嵌入：iframe + JWT 签名（5 分钟过期）          │
│    - 当前阶段（Wave 0.5）：M6Custom 做演示原型           │
│      展示交互模式（拖拽配置、保存模板、嵌入入口）        │
│                                                        │
│ C. 快捷入口                                            │
│    - 每个 DataTable 页加 "另存为自定义报表" 按钮         │
│    - 点击 → 跳 M6Custom（或 Metabase）+ 预填数据源       │
│                                                        │
│ D. 报表订阅（M6-005）                                   │
│    - WMS 自实现（定时 cron + 邮件 + 历史）               │
│    - 因为订阅触发逻辑与权限耦合，不依赖 Metabase         │
└──────────────────────────────────────────────────────┘
```

## 落地阶段

| 阶段 | 任务 | 周期 |
|---|---|---|
| **Wave 0.5（当前）** | 1. M6Custom 做交互演示原型（mock）<br>2. 5 个 DataTable 页加 "另存为报表" 入口（mock）<br>3. M6-005 报表订阅页（mock）<br>4. 本 ADR | 半天 |
| **Wave 1** | OpenAPI 元数据自动暴露（utoipa schema → 字段名/类型/枚举）| 复用 H3 工作 |
| **Wave 1.5** | M6-001 GSP 法定 5 张报表后端实现 + 数据签名 | 2 周 |
| **Wave 2.5** | M6-005 订阅引擎（cron + 邮件 + 历史 + 权限）| 1 周 |
| **Wave 5** | 部署 Metabase + JWT 嵌入 + 权限同步脚本 + 用户培训 | 1.5 周 |

## 关键约束

1. **数据安全（高优）**
   - Metabase 配置 PostgreSQL 只读账号
   - JWT 签名包含 `user_id` + `owner_ids`（货主隔离）
   - Metabase Sandbox 按 `owner_id` 行级过滤
   - 所有查询写 H2 审计

2. **权限映射**
   - WMS 用户 → Metabase 用户（同步脚本，每日）
   - WMS 角色 → Metabase Group
   - 货主 → Metabase Group + Sandbox

3. **不允许（红线）**
   - ❌ Metabase 直连业务库（应只读副本或专用数据库）
   - ❌ 让运营在 Metabase 里写 SQL（关闭 SQL 编辑器）
   - ❌ Metabase 公网暴露（必须经 WMS 反向代理 + JWT 鉴权）
   - ❌ 报表查询触发原始 SQL（必须经 WMS 后端代理 + 字段过滤）

4. **不替代**
   - GSP 法定报表必须 WMS 后端实现，不能用 Metabase（Metabase 不能保证字段、签名、保留期合规）

## 技术选型详细

### Metabase 嵌入流程

```ts
// WMS 前端：用户点击"业务报表 → 在 Metabase 中打开"
async function openInMetabase(dashboardId: number) {
  // 1. WMS 后端签发 JWT
  const { url } = await api.post('/m6/metabase/embed', {
    dashboardId,
    params: { owner_id: currentUser.ownerIds }
  });
  // 2. 在新窗口打开（或 iframe 嵌入）
  window.open(url, '_blank');
}
```

```rust
// WMS 后端：JWT 签发
fn sign_metabase_url(dashboard_id: u32, params: HashMap<&str, Value>) -> String {
    let claims = json!({
        "resource": { "dashboard": dashboard_id },
        "params": params,
        "exp": now() + 5 * 60,  // 5 分钟过期
    });
    let token = encode(&claims, METABASE_SECRET);
    format!("{}/embed/dashboard/{}#bordered=false", METABASE_URL, token)
}
```

### 字段元数据自动暴露

利用 H3（utoipa）：
```rust
#[utoipa::schema]
struct PurchaseAsn {
    /// 采购单号
    asn: String,
    /// 供应商
    #[schema(example = "国药控股北京")]
    supplier: String,
    /// 入库件数
    qty: i32,
    /// 入库金额（元）
    amount: f64,
}
```

→ OpenAPI schema → M6Custom 字段抽屉自动生成（Wave 1 接入）

## 替代方案分析（已否决）

- **纯自研**：6 周做不出 Metabase 30% 的能力，维护负担
- **纯 Metabase**：GSP 法定报表的字段固定 + 数据签名做不到
- **SaaS BI**：数据出库违反 GSP 第 14 条
- **Tableau / PowerBI**：商业、贵、Tableau 中文弱

## 影响范围

- 新增页面：m6-custom（升级）+ m6-subscriptions
- 业务页改动：5 个 DataTable 页加快捷入口（H2-002 / M1-001 / M1-002 / M3-001 / M4-005）
- 后端工作：Wave 1.5 起增加 GSP 报表 5 个 API + Wave 5 起 Metabase 部署
- 文档：本 ADR + 更新 docs/architecture-dependencies.md（M6 章节）

## 后果

### 正面

- 业务用户获得真正的"前台自建报表"能力（拖拽 + 透视 + 多图表 + 订阅）
- 开发不需要为每个新报表写代码（运营自助）
- GSP 法定与业务自定义清晰分离，合规边界明确
- Metabase 嵌入降低 4-6 周自研成本到 1-2 周接入
- ADR-0023 把"什么用法定 / 什么用 Metabase"写死，避免 Wave 5 来回讨论

### 负面 / 风险

- 新增运维负担（Metabase 实例 + JWT 服务）
- Metabase 学习成本（虽然简单但运营仍需 1 天培训）
- 行级权限映射依赖同步脚本（每日 cron），同步延迟 ≤ 24h
- AGPLv3 协议（Metabase）不能闭源 fork（不影响自托管使用）
- 本 ADR 不替代 GSP 法定报表的合规性 — 法定报表必须 WMS 后端实现

### 中性

- 引入新的部署组件（Metabase Docker），增加 1 个监控目标
- 数据源接入需要数据库账号管理（Metabase 只读）
- 当前 Wave 0.5 的 M6Custom / M6Subscriptions 是演示原型，
  Wave 5 时按 ADR 重新对接 Metabase

## 演进

- v1（2026-05-23）：初版 — 混合方案确定
- 预留：未来可换 Superset（同样 iframe 嵌入），架构无锁定
