# 用户故事：M1 设备中台（AGV / PTL / DWS / RFID）

> 模块：M1 device platform
> 依赖：M1 基础档案（库位/货架字段）、H1 权限、H2 审计/事件总线、H4 告警、M-CG 编号
> 关联 ADR：0048（Phase 3）、0038（v1 前直接改基线）
> 规格事实源：`docs/domain/device-platform-phase3-spec.md`（Frozen）

---

## US-M1-010：设备中台（AGV / PTL / DWS / RFID）

**作为** 仓库主管 / 设备管理员
**我要** 统一登记 AGV / PTL / DWS / RFID 设备，下发指令并通过事件回执闭环账务联动
**以便** 设备作业（货到人拍灯、称重复核、RFID 复核）自动确认落账，异常任务可人工介入

### 验收标准

1. **设备生命周期**：注册设备档案（仓库/编码/类型/厂商/协议/IP 端口），重复编码 409；启停开关停用后不再下发新指令；心跳上报置在线，超 90 秒无心跳置离线并触发 H4 告警。
2. **库位绑定**：库位-设备点位绑定（角色 `ptl_light` / `rfid_antenna` 与设备类型匹配）；同一库位同一角色仅一条生效绑定（409）；软解绑置 `valid_to` 保留历史链；离线/停用设备禁止绑定。
3. **指令-事件闭环**：业务事务内生成 `wcs_tasks`（M-CG 编号 `WCST-`、幂等键唯一）；派发 → 回执 → 校验 → 账务确认同事务落账 → `succeeded`；终态重复回执幂等忽略。
4. **PTL 拍灯**：亮灯互斥（同一 PTL 未终态任务 409）；拍灯即确认按拍灯量落账；数量差异未超阈值（±20% 或 |Δ|≤10）落账并告警，超阈值阻断转人工。
5. **DWS / RFID**：`dws_result` 校验 pass 且重量 ±20% 内落账；`rfid_batch` EPC 集合覆盖目标集合落账；校验失败任务回 `failed` 不落账。
6. **AGV 货到人**：`pod_move` 一托一搬（同货架活跃任务 409）；executing 置格口 `agv_unreachable_at` 不可达标记，终态清除；不可达期间格口账务动作 422 阻断；搬运全程不产生库存账变。
7. **超时重试与人工介入**：超时扫描置 `timeout`，退避重试（1/5/15 分钟，max_retries=3）耗尽 `failed` + H4 告警；管理端可重发/作废（未落账）/跳过确认。
8. **孤儿事件与一致性**：无任务 `ptl_press` 30 秒窗口内认领，超窗 H4 `device_event_orphan`；AGV 不可达标记与活跃任务不一致 → H4 `agv_marker_inconsistent`。
9. **模拟器先行**：设备端以可编程模拟网关替代（测试内联模拟器 + 开发 Mock）；以受 `m1.device.manage`、`Idempotency-Key` 和审计保护的 `dispatch` / `receipt` API 驱动状态闭环；真机协议驱动不在本切片范围。
10. **审计与幂等**：`iot_event_logs` 纯审计追加流只 INSERT；写路径统一 `Idempotency-Key` 幂等。

### 验收对照（GWT 1-22，规格 §11）

| GWT | 验收 | 测试落点 |
|---|---|---|
| 1-3 | 设备注册/重复/非法类型 | `m1_device_lifecycle_postgres.rs` |
| 4-6 | 绑定冲突/角色不匹配/离线禁绑 | 同上 |
| 7-8 | 心跳上线/超时离线告警 | 同上 |
| 9-10 | 指令生成幂等/亮灯互斥 | `m1_wcs_task_engine_postgres.rs` |
| 11 | 回执链路与终态幂等 | 同上 |
| 12-14 | PTL 拍灯落账/差异/超阈值 | `m1_ptl_agv_postgres.rs` |
| 15 | 孤儿事件窗口与告警 | `m1_wcs_task_engine_postgres.rs` |
| 16-18 | AGV 不可达/一托一搬/不落账 | `m1_ptl_agv_postgres.rs` |
| 19-20 | DWS/RFID 校验 | `m1_wcs_task_engine_postgres.rs` |
| 21 | 重试耗尽与人工介入 | 同上 |
| 22 | 标记一致性告警 | `m1_ptl_agv_postgres.rs` |

---

## 跨故事约束（适用于设备中台）

1. **只 INSERT 审计流**：`iot_event_logs` 为纯审计追加流，禁止 UPDATE/DELETE（对齐项目审计原则），保留期 ≥ 5 年。
2. **账务同事务**：事件校验通过后，任务状态推进与业务落账必须同事务；落账失败任务回 `failed` 且业务账不回。
3. **终态幂等**：`succeeded` / `failed` 为终态，重复回执/事件幂等忽略（事件仍记录）。
4. **不做真机协议**：模拟器/回放先行，厂商协议驱动（HTTP/TCP/Modbus/MQTT/RCS）留真机切片；`sorter_divert` 派发与 `stacker` 设备仅登记类型不实现。
5. **不改既有模块语义**：补货/波次/6 维/容器质量锁语义不回改；`pod_move` 不落库存账；格口不可达仅作作业可用性隔离，账面在手量不变。
6. **系统级扫描跨货主**：超时扫描与标记一致性扫描为系统级作业（查询内联 service，不落仓储层 owner 门禁），仅写事件不改业务账。
7. **多货主隔离**：`wcs_tasks` 按 `owner_id` 隔离；`iot_devices` 为仓库级共享资产不按货主隔离；设备事件按 `warehouse_id` 归属。
8. **编号与幂等**：`wcs_tasks.task_no` 走 M-CG `wcs_task` 规则；指令生成与事件处理按幂等键防重发重复执行。
