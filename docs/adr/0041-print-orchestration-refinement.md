# ADR-0041：H9 打印编排细化与 ADR-0039 局部取代

- 状态：Accepted
- 决策日期：2026-07-25
- 决策人：项目主人（2026-07-25 授权按 H9 复审建议修复并循环复审）
- 起草人：AI 助手
- 关联：H9 / M1 / M-CG / H6 / H-FILE /
  [ADR-0039 打印组套与 Print Agent](0039-print-suite-and-agent.md) /
  [ADR-0040 Print Agent 机器身份与协议闭环](0040-print-agent-machine-protocol.md)

---

## 背景

ADR-0039 已确认 H9 第二阶段的总体方向，但后续故事展开明确了四个更严格的业务边界：

1. 送货地址隔离还不足以防止跨货主或跨仓错误归集。
2. 发票、药检单已有权威 PDF，不应为了满足“每项模板”而二次渲染。
3. 必需打印项不能由失败策略跳过。
4. 已进入运行态的实例需要一条不会重复物理打印的备用 Agent 合法迁移路径。

ADR 一经 Accepted 不直接改写，因此用本 ADR 记录增量决策并局部取代 ADR-0039 的相应表述。

## 决策

### 1. 取代范围

本 ADR 仅局部取代 ADR-0039 的以下内容：

- §2 的归集硬边界；
- §3～§4 的打印项来源与 PDF 准备方式；
- §5 的必需打印项失败策略；
- §7 的运行中备用 Agent 故障转移。

ADR-0039 的其他业务方向继续有效。机器身份、协议和更新鉴权的取代范围由 ADR-0040 定义。

### 2. 归集硬边界

`owner_id + warehouse_id + delivery_address_id` 是不可配置的硬归集边界，任一值不同都不能
进入同一随货同行单。地址使用 M1 稳定地址主数据 ID，不使用地址文本作键。运营配置只能在
该硬边界内增加受控等值归集维度和调整维度顺序，不能删除或覆盖硬边界。

截单事务同时冻结订单集合、规则版本和硬边界，并通过 M-CG 限定编号主题
`print_document_category:delivery_note` 完成 no-gap 发号。

### 3. 打印分类与文件来源

H9 使用待实现的 M1 系统字典 `print_document_category`，字典项至少包含编码、中文名称和
`source_mode = rendered|external_file`：

- `rendered`：由 Render Worker 根据冻结模板版本和源数据生成 PDF，写入 H-FILE；
- `external_file`：校验并引用已摄取的 H-FILE 权威 PDF，不强制模板，也不二次渲染。

首批分类为随货同行单（`rendered`）、药检单和发票（`external_file`）。这套分类不替代
`print_template_type`；只有 `rendered` 项绑定模板版本。组套实例必须按打印项保存适用的
模板版本或权威文件 ID/版本、来源模式和内容哈希。

### 4. 顺序、失败与安全故障转移

打印项严格串行。失败策略“继续”只适用于非必需项且必须冻结到实例；必需项永远不能跳过，
只能修复并重试、终止失败，或在确认没有在途/结果不明后安全取消组套。

普通改派仍只允许未开始实例。运行中切换备用 Agent 只能通过 H6
`safe_agent_failover: running -> preparing`：

- 至少一项仍为 `pending`；
- 尚无任何已提交尝试，或上一项已确认 `succeeded|skipped`；
- 不存在在途、`result_unknown` 或等待对账尝试；
- 旧 Agent 必须在线，在本地持久化“停止启动新打印项”和最后确认项后完成交接；suspected/
  offline Agent 一律通过 `agent_connection_lost: running -> awaiting_reconciliation` 先进入
  对账，无论断联发生在首项提交前还是两项之间都不能直接故障转移；
- 释放租约的硬守卫始终要求无 `printing`、`result_unknown` 或未决对账；满足硬守卫后，
  冻结租约策略必须允许 `safe_auto`，否则只能由具备专用权限的用户提供原因并完成二次确认；
- 在同一事务中递增 `assignment_epoch`，释放旧 Agent 占用和旧设备租约，领取同站点兼容
  备用 Agent 与带新 `lease_token` 的 active 设备租约并冻结分配，任一步失败整体回滚；
- 备用 Agent 重新校验冻结清单、剩余 PDF、哈希及匹配
  `assignment_epoch + lease_token` 的 active 租约后，才能回到 `running`。

结果不明或等待对账时始终禁止故障转移、自动重打和普通改派。
每次启动打印项和上报结果必须携带当前 `assignment_epoch + lease_token`；旧代次不得推进
正常状态，迟到结果转入对账并告警，不能静默丢弃。

运行中的正常组套可接收授权、带原因且幂等的人工暂停请求；请求期间先完成当前项，只有确认
无在途、`result_unknown` 或待对账后，才通过
`operator_pause_applied: running -> paused` 停在下一项之前。普通暂停不改变 Agent 分配、
优先级或原入队时间；紧急停止结果不明时仍走人工确认，Agent“暂停接单”继续是独立运行状态。

## 后果

- 防止跨货主、跨仓或跨地址误归集。
- 外部权威 PDF 不再被模板模型强行二次加工。
- 必需项与物理打印结果不明时保持 fail-closed。
- 故障转移多一次准备和完整性校验，但获得可验证的租约与状态边界。
- `print_document_category`、M-CG 限定主题和 H6 状态事件仍是待实现能力，不能因 ADR
  Accepted 或故事完整而标记软件完成。

## 实施约束

1. 先冻结 M1 字典、M-CG 内部端口、H6 事件和 H3 OpenAPI，再按 outside-in TDD 实现。
2. L5/L6/L11 至少覆盖三键隔离、并发截单 no-gap、必需项失败、正常暂停/紧急停止分流、
   首项提交前及项目间断联入对账、旧 Agent 交接、租约硬守卫/释放模式、代次 fencing、
   故障转移事务回滚、无 active 租约拒绝运行和结果不明禁止转移。
3. `external_file` 缺失、哈希错误或摄取失败时不得创建可执行打印任务。
4. ADR-0039 保持原文作为历史决策记录；实现与验收以本 ADR 的局部取代规则和当前 H9
   用户故事为准。

## 参考

- [H9 打印组套用户故事](../domain/user-stories-h9-print-orchestration.md)
- [M-CG 编码生成用户故事](../domain/user-stories-mcg-code-generator.md)
- [基础设施技术规格](../infra/technical-specs.md)
