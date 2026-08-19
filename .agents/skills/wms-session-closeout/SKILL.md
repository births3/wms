---
name: wms-session-closeout
description: WMS 会话停止前的收口复盘技能。用户说停止、暂停、收尾、结束前总结、总结前几轮错误、沉淀经验、下次复用、项目停一下，或需要把本轮对话中的错误、漏检、返工、有效做法整理成后续可复用经验时使用；只做总结和规则建议，不直接修复代码或提交。
---

# WMS Session Closeout

在会话停止前，把最近几轮执行里的错误、返工和有效经验整理成下一次可复用的短报告。

## 触发方式

- 手动触发：用户说“停止、暂停、收尾、结束前总结、总结前几轮错误、沉淀经验、下次复用”等。
- 自动触发：由 `scripts/agents/session_closeout_runner.py` 外部 watcher 检查项目空闲时间后，用 `codex exec` 调用本技能。
- 本技能自身不会定时运行；定时能力必须由 `just session-closeout-install-watchdog` 安装 cron watchdog 后提供。

自动触发默认只生成本地报告到 `.codex/session-closeout/reports/`，不修改 tracked 文件、不提交、不推送。常用命令：

```bash
just session-closeout-once --idle-seconds 1800
just session-closeout-once --idle-seconds 1800 --apply
just session-closeout-install-watchdog
just session-closeout-status
```

## 先读

- `AGENTS.md`
- 最近用户明确指出的问题、返工点、验证失败和确认结论
- 本轮触发过的 `.agents/skills/*/SKILL.md`
- 当前 `git status --short`
- 必要时查看相关 diff、测试输出、截图路径或 issue/PR 评论

## 工作流

1. 划定范围：只总结最近一段连续任务，不重写全项目历史；若用户指定模块或 issue，以用户范围为准。
2. 提取错误：记录事实，不写泛泛反省。优先包含：
   - 用户明确指出“不对 / 没检查出来 / 不是我要的 / 偏了”的点。
   - 测试、E2E、self-check、治理脚本之前没覆盖到的点。
   - 子代理、worktree、issue-agent、重启、截图、提交、合并的闭环漏项。
3. 定位断点：每个错误只选一个主因，归类为：
   - 需求理解偏差
   - 输入未进入 prompt
   - 实现只做表面
   - 测试断言层级不够
   - 治理脚本未覆盖
   - 证据未留存或未回写
   - 收口流程缺失
   - 脏区 / worktree / 分支混杂
4. 提炼经验：把经验写成可执行规则，例如“E2E 断言用户级不变量，不断言错误实现细节”。
5. 判断固化落点：
   - 单次提醒：只写在本次收口报告。
   - 可复发执行问题：建议改 `wms-execution-retrospective`、runbook 或相关脚本。
   - 测试覆盖问题：建议改质量矩阵、self-check、Playwright E2E 或 `wms-quality-matrix-governance`。
   - 子代理 / worktree 问题：建议改 `wms-worktree-subagent`。
   - 协作默认行为：建议改 `AGENTS.md`。
6. 输出下一次提示词：给一段短触发语，用户下次可直接复制使用。

## 输出格式

保持短，按以下结构输出：

```markdown
**本轮收口**
- 范围：...
- 当前状态：...

**主要错误**
1. 现象：...
   主因：...
   下次规则：...

**有效经验**
- ...

**建议固化**
- 落点：...
- 验证：...

**下次触发语**
缺口闭环/收尾复盘 <模块或 issue>
```

## 判断标准

- 没有事实证据的，不写成结论。
- 不把多个主因堆在一起；每个错误只写最关键断点。
- 不重复长日志、长 diff、长测试输出，只摘关键证据。
- 不自动提交、推送、清理 worktree 或修改代码。
- 发现需要修规则时，只建议落点；用户要求“落地/修改/修复”后再调用对应技能执行。
