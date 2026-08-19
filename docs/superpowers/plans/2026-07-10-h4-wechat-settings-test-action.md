# H4 参数测试入口调整实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 H4 参数测试动作移动到新增/修改参数弹窗，并确保测试当前表单对应的已保存参数。

**Architecture:** 复用现有 `SettingsDialog`、保存 mutation 和测试 mutation。弹窗测试动作按“保存当前表单 -> 调用现有测试接口”串行执行，列表页不再保留独立测试动作和确认弹窗。

**Tech Stack:** React、TypeScript、TanStack Query、Playwright、现有 `@wms/ui`。

## Global Constraints

- 不新增后端 API 或依赖。
- 测试只校验已保存参数的完整性和启用状态，不声明外部网络连通。
- 保存或测试期间禁止重复提交。

---

### Task 1: 移动 H4 参数测试动作

**Files:**
- Modify: `apps/web-admin/self-checks/h4-wechat-notify-slice-self-check.mjs`
- Modify: `prototypes/e2e/web-admin-h4-dev.spec.ts`
- Modify: `apps/web-admin/src/pages/wechat-notify/H4WechatNotifyDialogs.tsx`
- Modify: `apps/web-admin/src/pages/wechat-notify/H4WechatNotifyPage.tsx`

**Interfaces:**
- Consumes: `useUpsertH4WechatSettingsMutation()`、`useTestH4WechatSettingsMutation()`。
- Produces: `SettingsDialog` 新增 `testing: boolean` 和 `onTest: () => void` 属性。

- [ ] **Step 1: 写失败的自检和 E2E**

自检要求 `SettingsDialog` 接收 `onTest={testSettings}`，并禁止页面出现 `key: "test-settings"` 和 `SettingsTestDialog`。E2E 从“修改”弹窗点击“测试”，断言保存请求完成后才出现测试请求。

- [ ] **Step 2: 验证测试按预期失败**

Run: `pnpm --dir apps/web-admin run self-check`

Expected: FAIL，指出参数测试仍位于列表工具栏或弹窗缺少测试动作。

- [ ] **Step 3: 实现最小改动**

删除 `SettingsTestDialog` 和列表工具栏测试动作；在 `SettingsDialog` 页脚加入：

```tsx
<Button type="button" variant="outline" disabled={saving || testing} onClick={onTest}>测试</Button>
<Button type="button" disabled={saving || testing} onClick={onSave}>保存</Button>
```

页面的 `testSettings()` 先调用现有保存 mutation 保存 `settingsForm`，再调用测试 mutation；成功或失败后关闭参数弹窗并显示现有通知。

- [ ] **Step 4: 验证实现**

Run: `pnpm --dir apps/web-admin run self-check`

Expected: PASS。

Run: `pnpm --dir prototypes exec playwright test --config=playwright-web-admin-h4-dev-config.ts`

Expected: 1 passed。

Run: `pnpm --dir apps/web-admin run build`

Expected: exit 0。

Run: `just gov-t1`

Expected: 52/52 ok。

- [ ] **Step 5: 重启和提交**

```bash
just dev-web-restart
git add -- apps/web-admin/self-checks/h4-wechat-notify-slice-self-check.mjs prototypes/e2e/web-admin-h4-dev.spec.ts apps/web-admin/src/pages/wechat-notify/H4WechatNotifyDialogs.tsx apps/web-admin/src/pages/wechat-notify/H4WechatNotifyPage.tsx docs/superpowers/plans/2026-07-10-h4-wechat-settings-test-action.md mkdocs.yml
git commit -m "修复(企微)：移动参数测试入口"
```
