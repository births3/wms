# Wave 6 任务完成情况逐项核对

初版日期：2026-06-04
更新日期：2026-06-09

范围：以 `TODO.md` 当前 Wave 6 勾选项为权威任务清单，并用 ROADMAP、ADR-0035、Wave 1-6 门禁脚本和 evidence validator 交叉核对。Wave 1-5 只核对开发完成门禁；Wave 6 核对预发布 evidence 收口门禁。

## 总结

| 项 | 结论 | 证据 |
|---|---|---|
| Wave 1 开发完成 | 已完成 | `just wave-1-complete-check` 通过；W6.A / W6.B runtime evidence 已补齐并通过 `just wave-1-runtime-evidence-validate` |
| Wave 2 开发完成 | 已完成 | `just wave-2-complete-check` 通过；W6.C runtime evidence 已补齐并通过 `just wave-2-runtime-evidence-validate` |
| Wave 3 开发完成 | 已完成 | `just wave-3-complete-check` 通过；真 PDA / L7 evidence 单独后移 |
| Wave 4 开发完成 | 已完成 | `just wave-4-complete-check` 通过；W4.D 外部 evidence 单独后移 |
| Wave 5 开发完成 | 已完成 | `just wave-5-complete-check` 通过 |
| Wave 6 预发布收口 | 未完成 | `just wave-6-status` 显示 W6.A / W6.B / W6.C 已关闭；剩 W6.D-H 5 个真实 evidence gate 未关闭，另缺 Wave 6 retro |

## 全量 TODO 扫描

脚本扫描 `TODO.md` 全部勾选项后，当前可归纳为：

| 区域 | 完成情况 | 核对结论 |
|---|---:|---|
| 当前 Wave 6 | 12/23 | W6.A / W6.B / W6.C 已补齐；剩 11 个外部依赖 / 真实 evidence / retro 项未完成 |
| 已归档 Wave 5 | 8/8 | `just wave-5-complete-check` 通过 |
| 已归档 Wave 4 | 10/10 | `just wave-4-complete-check` 通过；W4.D 外部 evidence 按 #50 后移 |
| 已归档 Wave 3 | 16/17 | `just wave-3-complete-check` 通过；M9 后续已由 Wave 5.C 关闭，PDA 生产端仍由 Wave 6 承接 |
| 已归档 Wave 0.5 | 32/32 | 原型 / Spike / packages/ui 抽离完成，T1 覆盖 |
| Wave 0.5 退出条件 | 8/8 | Wave 1 准入条件满足 |
| 原型前端检查遗留项 | 1/2 | `prototype-model.ts` 陈旧 TODO 已修正为完成；`m4-manifest` 仍未完成 |

全量 TODO 当前仍未完成的项目只剩两类：

| 类别 | 项目 |
|---|---|
| Wave 6 预发布 evidence / 外部依赖 | 当前 Wave 6 中 11 个未勾选项，见下方逐项表；另有 Wave 3 PDA 生产端后续项由 W6.D 承接 |
| 原型视觉遗留 | `m4-manifest` 随货同行单 PDF 中文竖排仍存在，截图证据见 `governance/visual-baselines/m4-manifest.png` |

## 当前阻塞 / 外部依赖

| TODO 行 | 任务 | 当前状态 | 核对依据 |
|---|---|---|---|
| 15 | 剩余 W6.D-H 仍需真 PDA / 外部系统 / 硬件 / staging 灰度环境 | 未完成 | `just wave-6-status` 显示 W6.A / W6.B / W6.C 已关闭；W6.D-H 仍缺真实 evidence |
| 16 | M-PK 电子秤 / 蓝牙打印机 / 面单打印真实设备未接入 | 未完成 | `just wave-5-hardware-evidence-validate` 缺 `docs/retros/wave-5-hardware-evidence.json` |
| 17 | 外部 TMS dev/staging 接口、回调鉴权、调度结果格式仍需确认 | 未完成 | `just wave-5-tms-evidence-validate` 缺 `docs/retros/wave-5-tms-evidence.json` |
| 18 | “码上放心”账号、正式接口文档、鉴权方式、错误码、频率限制和 dev/staging 回执仍需补齐 | 未完成 | `just wave-4-external-dependencies-validate` 缺 `docs/retros/wave-4-external-dependencies.json` |
| 19 | 首次试运行投产灰度发布环境和回滚链路需按 ADR-0016 准备 | 未完成 | `just wave-6-deploy-evidence-validate` 缺 `docs/retros/wave-6-deploy-evidence.json` |

## 进行中 / 待做

| TODO 行 | 任务 | 当前状态 | 核对依据 |
|---|---|---|---|
| 23 | W6 scope | 已完成 | ADR-0035、ROADMAP、TODO、architecture-dependencies 已登记；`just wave-6-status` 中 `W6-startup` 通过 |
| 24 | W6 status / complete check | 已完成 | `just wave-6-status` 与 `just wave-6-complete-check` 已存在并可运行 |
| 25 | W6 closeout runbook | 已完成 | `docs/runbooks/wave-6-closeout.md` 存在；`W6-tooling` 通过 |
| 26 | W6.A Wave 1 H2 runtime evidence | 已完成 | `docs/retros/wave-1-h2-runtime-evidence.json` 存在；`just wave-1-runtime-evidence-validate` 通过 |
| 27 | W6.B Wave 1 W1.D 自动回滚 evidence | 已完成 | `docs/retros/wave-1-runtime-evidence.json` 存在；`just wave-1-runtime-evidence-validate` 通过 |
| 28 | W6.C tooling | 已完成 | `record_wave2_runtime_evidence.py` 与 `just wave-2-runtime-evidence-record` 已登记；`W6-tooling` 通过 |
| 29 | W6.C Wave 2 配置中心 Feature Flag evidence | 已完成 | `docs/retros/wave-2-runtime-evidence.json` 存在；`just wave-2-runtime-evidence-validate` 通过 |
| 30 | W6.D tooling | 已完成 | Wave 3 PDA evidence record / validate 脚本与 just 入口已登记；`W6-tooling` 通过 |
| 31 | W6.D Wave 3 真 PDA + L7 evidence | 未完成 | `just wave-3-pda-runtime-evidence-validate` 缺 `docs/retros/wave-3-pda-runtime-evidence.json` |
| 32 | W6.E Wave 4 M-TC “码上放心” external evidence | 未完成 | `just wave-4-external-dependencies-validate` 缺 `docs/retros/wave-4-external-dependencies.json` |
| 33 | W6.F tooling | 已完成 | Wave 5 hardware evidence runbook、record / validate 脚本与 just 入口已登记；`W6-tooling` 通过 |
| 34 | W6.F Wave 5 M-PK hardware evidence | 未完成 | `just wave-5-hardware-evidence-validate` 缺 `docs/retros/wave-5-hardware-evidence.json` |
| 35 | W6.G tooling | 已完成 | Wave 5 TMS evidence runbook、record / validate 脚本与 just 入口已登记；`W6-tooling` 通过 |
| 36 | W6.G Wave 5 M10 TMS+ evidence | 未完成 | `just wave-5-tms-evidence-validate` 缺 `docs/retros/wave-5-tms-evidence.json` |
| 37 | W6.H tooling | 已完成 | Wave 6 deploy evidence runbook、record / validate 脚本与 just 入口已登记；`W6-tooling` 通过 |
| 38 | W6.H 首次试运行灰度发布 evidence | 未完成 | `just wave-6-deploy-evidence-validate` 缺 `docs/retros/wave-6-deploy-evidence.json` |
| 39 | W6 retro | 未完成 | `just wave-6-complete-check` 缺 `docs/retros/wave-6-retro.md` |

## 缺口清单

Wave 6 当前缺 5 个 evidence gate，对应 5 个真实 evidence JSON 文件：

| Gate | 缺失文件 |
|---|---|
| W6.D | `docs/retros/wave-3-pda-runtime-evidence.json` |
| W6.E | `docs/retros/wave-4-external-dependencies.json` |
| W6.F | `docs/retros/wave-5-hardware-evidence.json` |
| W6.G | `docs/retros/wave-5-tms-evidence.json` |
| W6.H | `docs/retros/wave-6-deploy-evidence.json` |

已关闭的 evidence gate：

| Gate | Evidence 文件 | 验证 |
|---|---|---|
| W6.A | `docs/retros/wave-1-h2-runtime-evidence.json` | `just wave-1-runtime-evidence-validate` 通过 |
| W6.B | `docs/retros/wave-1-runtime-evidence.json` | `just wave-1-runtime-evidence-validate` 通过 |
| W6.C | `docs/retros/wave-2-runtime-evidence.json` | `just wave-2-runtime-evidence-validate` 通过 |

此外，Wave 6 最终完成还缺 `docs/retros/wave-6-retro.md`。该 retro 只能在上述剩余 evidence 全部通过后编写。

## 本次发现并修正的不一致

`just wave-4-complete-check` 原先仍要求 `TODO.md` 是“当前 Wave：Wave 4”，导致 Wave 4 已归档到当前 Wave 6 后复跑失败。已修正为接受“当前 Wave：Wave 4”或“已归档：Wave 4”，并补测试覆盖归档状态。

`TODO.md` 中 `prototype-model.ts` 原型 mock 语义错位项仍未勾选，但当前代码已通过 `rowSample` / `fieldSample` 建立列/字段与示例值逐项对应，并且 `just gov-t1` 中 `check_prototype_fidelity.py` 通过。已把该项改为完成，并保留核对说明。

`TODO.md` 中 Wave 3 的 M9 自动计费后续项仍未勾选，但 Wave 5.C 已落地自动计费、计费明细和月结账单，并且 `just wave-5-complete-check` 通过。已把该项改为完成，并指向 Wave 5.C 证据。

2026-06-09 更新：W6.A / W6.B / W6.C 的真实 evidence 已补齐并通过 validator，`TODO.md`、`ROADMAP.md` 与本审计报告已同步为剩余 W6.D-H + Wave 6 retro。

验证：

| 命令 | 结果 |
|---|---|
| `just wave-4-complete-check` | 通过 |
| `python3 -m pytest scripts/governance/tests/test_core_logic.py -q` | 141 passed |
| `just gov-t1` | 30/30 ok |
| `just task-check` | 6/6 ok |
| `git diff --check` | 通过 |
