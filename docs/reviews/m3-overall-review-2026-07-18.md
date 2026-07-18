# M3 库存模块诚实复审（2026-07-18）

> 对照 `docs/domain/user-stories-m3-*.md` 与实现；矩阵当前 9 条 US-M3 均为 verified，但软件路径仍有缺口。  
> 不伪造 PDA / 硬件 / 真企业微信 / 真 ERP 回执（S4）。

## 总览

| 故事 | 矩阵 | 现实摘要 |
|------|------|----------|
| US-M3-001 查询 | verified | 多维筛选、货主隔离、导出较实 |
| US-M3-002 批号效期 | verified | 过期隔离/召回双人实；近效期天数常硬编码 180；H4 推送薄 |
| US-M3-003 状态 | verified | 状态机/审计/ERP outbox 实；PC 审批源自由文本 |
| US-M3-004 养护 | verified | **异常结论曾被硬拒绝**，无 M-QL；计划生成过薄；PC 无提交记录 |
| US-M3-005 盘点 | verified | 后端 create/submit/approve 实；**PC 缺 submit/approve**；**分配不读盘点锁** |
| US-M3-006 移库 | verified | 同事务移库+温区/容量校验；无 M-TC 扫码、容积回写弱 |
| US-M3-009 预警 | verified | 近效期生成有；**handle 无审计/幂等**；类型不全 |
| US-M3-010 ABC | verified | recompute/override 有；**无审计/幂等**；阈值固定 |
| US-M3-011 库位历史 | verified | API+风险+PC 列表较实 |

## P0（本轮优先修）

1. 养护异常可写记录 + 建质量联系单/通知 + 批次隔离（`approval_source=养护异常`）
2. 盘点 PC：明细实盘提交 + 差异审批
3. 盘点中批次禁止新出库分配
4. 预警处理 / ABC 重算与覆盖写审计（写路径最小闭环）

## P1（后续）

- 养护计划按重点/一般/入库 7 天规则生成；完成率与逾期预警  
- 近效期读货主 `expiry_warning_days`  
- 盘点差异阈值与盲盘隐藏 `book_qty`  
- 移库 trace_codes / used_volume 回写  
- 状态审批源字典化  

## 已较扎实

- 库存查询 owner 隔离与组合筛选  
- 过期隔离幂等、召回双人审批  
- 盘点后端原子调账与幂等  
- 移库同事务加减与隔离/召回拦截  
- 库位历史风险与页面跳转  

## 本轮修复跟踪

| 项 | 状态 |
|----|------|
| P0-1 养护异常 | **已修**：允许 abnormal+exception_type；隔离 + status_change(approval_source=养护异常) + H4 通知；类型 `maintenance_abnormal` 存在时建 M-QL |
| P0-2 盘点 PC | **已修**：明细弹窗提交实盘 + 审批差异；self-check 覆盖 submit/approve API 与文案 |
| P0-3 盘点锁分配 | **已修**：`allocate_inventory_for_outbound` 排除 `inventory_counts.status=in_progress` 批次 |
| P0-4 预警/ABC 审计 | **已修**：handle_inventory_alert / recompute_abc / override_abc 写 audit_event |

### 验证

- `cargo test -p wms-domain maintenance --lib`
- `cargo test -p wms-api --test m3_maintenance_postgres`
- `cargo test -p wms-api --test m3_ops_closeout_postgres`
- `node apps/web-admin/self-checks/m3-ops-pages-self-check.mjs`

### 本轮 P1 修复（续）

| 项 | 状态 |
|----|------|
| 近效期天数 | **已修**：alerts/job/handler 走 `resolve_expiry_warning_days`（字典缺失回退 180） |
| 养护计划 | **已修**：冷链/特药约 30 天、一般约 90 天 + 近效期窗口；PC 可提交养护结果 |
| 盘点盲盘 | **已修**：in_progress/pending 未交实盘前回显 book_qty=0 |
| 盘点差异阈值 | **已修**：\|差异\|/账面>10% 需审批源「盘点-高级」；PC 自动选择 |

### 仍属 P1 余量 / S4

移库 M-TC 扫码、容积回写、真 PDA/企微/ERP、养护完成率统计与逾期预警看板。
