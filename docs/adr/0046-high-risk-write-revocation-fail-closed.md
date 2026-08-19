# ADR-0046：高风险写入在撤销存储故障时 fail-closed

- 状态：Accepted
- 日期：2026-08-01
- 决策者：项目主人（AR-01）
- 关联：ADR-0024、ADR-0010、ADR-0011、US-H1-003、US-H1-005

## 背景

ADR-0024 §2.3.1 为保证仓内普通作业可用，在 Redis 撤销存储不可用时跳过
`permissions_changed_at` 和 token blacklist 检查。该策略对读取和普通写入仍然适用，
但角色变更、双人复核、库存质量状态和打印发布等操作不能接受最长一个 access token
TTL 的撤销窗口。AR-01 要求只调整故障语义，不重写 JWT 或引入新的鉴权框架。

## 决策

### 1. 按操作风险分级

请求由共享 AuthContext extractor 按 HTTP 方法和路径分类：

| 风险级别 | 范围 | 撤销存储正常 | 撤销存储命中撤销 | 撤销存储不可用 |
|---|---|---|---|---|
| 读取 | GET、HEAD、OPTIONS | 继续 | 401 `AUTH-004` 或 `AUTH-009` | 继续，记录 P1 告警 |
| 普通写入 | 未列入高风险清单的写请求 | 继续 | 401 `AUTH-004` 或 `AUTH-009` | 继续，记录 P1 告警 |
| 高风险写入 | 下表清单 | 继续 | 401 `AUTH-004` 或 `AUTH-009` | 拒绝，503 `AUTH-010` |

高风险清单保持显式、可审查，并与 `auth.rs::classify_auth_operation` 同步：

- H1 角色、用户、用户角色、API Key、会话撤销和密码变更；
- M-VR 双人策略、M-QL 质量联系单及审批回写；
- 库存盘点提交/审批、ABC 覆盖、库存告警/状态操作、库存调整质检审批/执行；
- M4 波次释放、订单复核/发货/作废及拣货完成等状态转换；
- H9 打印模板/组套发布或停用、分类 PDF 准备、紧急打印和其他以
  `/approve`、`/review`、`/ship`、`/publish`、`/execute`、`/prepare`、
  `/emergency-print` 等受控动作结尾的写请求。

未列入清单的写请求保持普通写入语义，保证普通收货、建单和查询类作业不会因为
Redis 短时故障整体停摆。新增高风险动作必须同时更新清单、ADR、故事和矩阵测试。

### 2. 故障与恢复

- 本决策不引入 PostgreSQL 回查；因此不存在“PostgreSQL 回查失败后放行”的路径。高风险
  写入在撤销存储不可用时直接返回 503，业务事务不会启动。
- Redis 抖动按请求处理：任一权限变更时间或 blacklist 查询失败，高风险写入立即拒绝；
  读取和普通写入按 ADR-0024 的 fail-open 继续，并记录 `alert=P1`。
- Redis 恢复后下一次请求重新执行两项检查，不缓存“故障期间放行”的结果；命中撤销仍
  返回 401。客户端对 503 仅可按幂等键和退避策略重试，禁止盲目重放非幂等写入。
- 告警级别保持 P1；运行手册要求记录故障窗口、受影响的高风险请求和恢复后的验证。

本 ADR 仅局部取代 ADR-0024 §2.3.1 的“所有请求 fail-open”语义；JWT claims、黑名单
存储、错误码体系和其余鉴权边界继续有效。

## 后果

- 正面：撤职、权限变更或 token 吊销期间，高风险写入不会在撤销存储故障时继续改变
  库存、审批或打印状态；普通仓内作业仍保留可用性。
- 负面：Redis 故障期间高风险写入会暂时不可用，调用方需要处理 503；高风险路径清单
  需要随新接口维护。
- 运行成本：沿用现有 Redis、JWT 和日志链路，不增加数据库、缓存或鉴权框架。

## 实施约束

1. `AuthRuntimePolicy::validate_claims_for` 是唯一的风险语义入口；业务 handler 不得
   自行复制 fail-open/fail-closed 分支。
2. 每次改变高风险清单必须补充 `auth_runtime_policy_matrix`，至少覆盖正常、撤销命中、
   存储不可用和恢复四种路径。
3. `AUTH-010` 只表示撤销存储不可用；不要把它转换为权限不足或业务校验错误。

## 验证

```bash
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test auth_runtime_policy_matrix -- --test-threads=1
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test auth_session_postgres -- --test-threads=1
```

## 参考

- [ADR-0024 鉴权模型](0024-auth-model.md)
- [H1 用户故事](../domain/user-stories-h1-auth-tenant.md)
- [撤销存储故障运行手册](../runbooks/auth-revocation-degradation.md)
