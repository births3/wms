# scripts/AGENTS.override.md

`scripts/` 脚本模块规则。

- 治理脚本应提供人类可读输出；可行时同时提供 `--json`。
- T1 脚本必须快速、确定，不依赖网络或外部系统。
- 证据记录器只有在明确记录模式下才可写证据；就绪检查 / 预检模式必须只读。
- 新增检查脚本、验证器、记录器、调度规则时要补测试。
- 脚本有已知误报时，优先让脚本理解约定，不用批量修改文档绕过。
- `governance/gate-rules.toml`、`governance_checks.py`、smoke 测试和文档必须同步。
- 验证用 `just gov-t1`。
