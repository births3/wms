# backend/AGENTS.override.md

`backend/` 后端模块规则。

- 技术栈：Rust + Axum + SQLx + PostgreSQL。
- 遵守 [docs/layered-design.md](../docs/layered-design.md)：`bin/runtime -> handler -> service -> domain/repository`。
- `domain` 代码不得依赖 Axum、SQLx、Redis、环境变量或直接读取系统时间。
- `handler` 只负责提取请求 / 上下文、调用 `service` / `repository facade`、转换错误；业务规则放到 `service` / `domain`。
- SQL 放在 `repository` 或聚焦的持久化辅助模块中。
- 后端改动必须先查现有 domain 实体/值对象、service 命令、repository trait、错误类型、审计 helper、OpenAPI DTO 和测试夹具；能复用就复用。
- 没有现成能力时，新增为标准可复用单元：业务不变量放 `domain`，用例编排放 `service`，持久化放 `repository`，HTTP 转换放 `handler`，跨用例测试数据放既有 fixture/helper。
- 新增后端 helper 或 trait 需说明复用缺口、放置理由和后续复用点；禁止为单个接口复制相似 SQL、状态流转或审计写入逻辑。
- 货主隔离必须使用 `ctx.owner_id` 或显式 owner scope 查询。
- 审计表只追加：只能 INSERT，禁止 UPDATE / DELETE。
- 禁止 `unwrap`；使用类型化错误。测试里的 `expect` 必须有有用信息。
- 非平凡逻辑需要新增或更新最小相关 Rust 测试。
- 迁移文件名必须符合 `check_file_naming.py` 接受的时间戳格式。
- 验证用 `just gov-t1`；后端行为变化时补跑相关 `cargo test`。
