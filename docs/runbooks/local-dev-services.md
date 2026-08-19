# 本地开发服务常驻运行手册

## 运行边界

本入口只用于开发环境：

- API、H9 Render Worker、H8 ERP Worker 直接在宿主机运行，不构建应用 Docker 镜像。
- PostgreSQL、Redis、MinIO 继续使用 `docker-compose.dev-h2.yml` 提供基础设施。
- 单独运行 `just dev-services` 时 Web Admin 继续使用现有 Vite HMR；运行 `just dev-up` 时由统一入口管理 9002。
- migration 不会因为保存文件自动执行；watcher 发现 migration 变化时只提示运行 `just dev-migrate`。

## 首次准备

```bash
mkdir -p deploy/secrets deploy/env
test -f deploy/env/dev-h2.env || \
  cp deploy/env/dev-h2.env.example deploy/env/dev-h2.env
```

编辑 `deploy/env/dev-h2.env`，至少填写数据库、MinIO、H9 token 和 JWT secret；真实值不提交。
数据库 secret 文件必须与 `WMS_DEV_H2_DB_PASSWORD` 一致：

```bash
set -a
source deploy/env/dev-h2.env
set +a
if test ! -s deploy/secrets/wms_dev_h2_db_password.txt; then
  printf '%s' "$WMS_DEV_H2_DB_PASSWORD" > deploy/secrets/wms_dev_h2_db_password.txt
fi
test "$(cat deploy/secrets/wms_dev_h2_db_password.txt)" = "$WMS_DEV_H2_DB_PASSWORD" || {
  echo "数据库 secret 文件与 dev-h2.env 不一致" >&2
  exit 1
}
```

## 启动

统一启动一个 runtime（推荐）：

```bash
just dev-up runtime=dev-h2
```

`runtime` 会选择 `deploy/env/<runtime>.env`，同一份配置同时用于 API、选择的 Worker 和前端 9002。
不同 runtime 不共用数据库配置，避免前端读到另一套 API / PostgreSQL。

H8 重同步环境有两种用法：

```bash
# 18184 API/Worker 已经在运行，只把前端 9002 接到它
cp deploy/env/h8-resync.env.example deploy/env/h8-resync.env
# 编辑 h8-resync.env，填入本机连接信息
just dev-up runtime=h8-resync services=none

# 由统一入口启动 API 和 H8 Worker（端口必须未被其它进程占用）
just dev-up runtime=h8-resync services=api,h8
```

`services=none` 不会启动后端，只会启动前端并检查配置中的 API 健康状态；适用于外部或既有 runtime。

需要单独启动应用服务时仍可使用：

只启动 API 和渲染服务：

```bash
just dev-services api,render
```

启动 API、渲染服务和 H8 worker：

```bash
just dev-services api,render,h8
```

命令会先启动 PostgreSQL、Redis、MinIO，执行一次 migration，然后在前台运行宿主机服务。源码变化后只重启受影响的服务；Rust 使用增量 `cargo run`，渲染服务使用 `pnpm` + `node`。

直接使用 `just dev-services` 时默认读取 `deploy/env/dev-h2.env`；它不管理前端 9002。

查看不启动服务的命令预览：

```bash
just dev-services-describe api,render,h8
```

H8 还需要在 `dev-h2.env` 中配置 `WMS_H8_WORKER_API_KEY`、`H8_CONNECTOR_ID` 和 `WMS_H8_SECRET_ALIASES`，并准备对应的本地 ERP 连接；未配置时先用 `api,render`。

## 常驻运行

安装用户级 systemd 服务：

```bash
repo_root="$(git rev-parse --show-toplevel)"
mkdir -p "$HOME/.config/systemd/user"
ln -sf "$repo_root/deploy/systemd/wms-dev-services.service" \
  "$HOME/.config/systemd/user/wms-dev-services.service"
systemctl --user daemon-reload
systemctl --user enable --now wms-dev-services.service
```

systemd 默认启动 API 和渲染服务；H8 需要外部 ERP 连接，按需在终端运行 `just dev-services h8`，避免重复启动 API 和渲染服务。

服务文件位于 `deploy/systemd/wms-dev-services.service`，默认假设仓库路径为 `%h/workspace/wms`。查看状态和日志：

```bash
systemctl --user status wms-dev-services.service
journalctl --user -u wms-dev-services.service -f
```

若要求未登录时也自动启动，按机器权限启用 user lingering：

```bash
loginctl enable-linger "$USER"
```

停止并取消开机启动：

```bash
systemctl --user disable --now wms-dev-services.service
```

## 自动行为

- 源码、Cargo 配置、H9 package 或 workspace lock 文件变化：重启对应应用服务。
- `backend/migrations` 变化：只输出迁移提示，不自动改数据库。
- 服务异常退出：等待短暂间隔后自动重启。
- `target`、`node_modules`、`artifacts`、`.git` 等生成目录不会触发重启。
