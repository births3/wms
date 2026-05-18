# ADR-0016：部署形态（Docker + docker-compose / Kubernetes 双轨）

- 状态：Accepted
- 决策日期：2026-05-18
- 决策人：项目主人
- 关联：ADR-0001 / ADR-0013 / docs/governance.md §3.7

---

## 背景

软件设计审计 §4 维度 10 识别部署形态完全空白：

- ADR-0001 选了技术栈但**未涉及部署**
- 没有 Dockerfile / docker-compose / k8s manifests
- 没有 CI/CD 流程（GitHub Actions / GitLab CI）
- 多环境（local/dev/staging/prod）配置策略未定

不解决会导致 Wave 1 启动时各开发者本机环境差异 → 部署到客户失败。

---

## 候选方案

### 方案 A（推荐）：Docker + docker-compose（小型）/ Kubernetes（大型）双轨

按客户规模选：
- **小型 3PL**（< 50 用户 / 1 仓）：docker-compose 单机部署
- **中大型连锁 / 多仓 / 多货主**：Kubernetes（k3s 自建 / EKS / 阿里云 ACK）

共用：
- 多阶段 Dockerfile（cargo chef + sccache + 最小 distroless）
- ConfigMap / Secret 走 ADR-0013
- 滚动升级 + DB migration 兼容窗口

### 方案 B：仅 Kubernetes

**否决**：小型 3PL 不需要 k8s 复杂度；运维成本高。

### 方案 C：仅 docker-compose

**否决**：大型客户需要高可用 + 滚动升级 + 多副本，docker-compose 不够。

### 方案 D：传统裸机 + systemd

**否决**：医药仓 IT 运维能力差异大，容器化是标准；裸机部署难以一致。

---

## 决策

**采用方案 A：双轨部署**。

### 部署矩阵

| 客户场景 | 部署形态 | 资源最低要求 |
|---|---|---|
| 小型 3PL（< 50 用户）| docker-compose（单机）| 4C / 16G / 500GB SSD |
| 中型 3PL（50-200 用户 / 多仓）| docker-compose（单机 + 主备 PG）| 8C / 32G / 1TB SSD |
| 大型连锁（> 200 用户 / 全国 / SLO 99.99%）| Kubernetes（k3s 或托管 K8s）| 3 节点 × 8C / 32G / 1TB |

### 容器架构

```
应用容器（必有）：
  - wms-api      （Rust + Axum 后端 API）
  - wms-web      （Vite + React 静态文件，Nginx 反代）
  - wms-pda-bff  （PDA BFF，Wave 4 后启用）

基础设施容器（必有）：
  - postgres     （主库）
  - postgres-replica（dev/staging 可选；prod 必有，AS H10 备份）

可观测容器（推荐，按 ADR-0011）：
  - prometheus
  - loki
  - grafana
  - tempo（trace 后端，可延后）

异步容器（按需，Wave 2+）：
  - redis        （缓存，Wave 1 末引入；H1 token 缓存）
  - kafka 或 nats（事件总线，Wave 2 引入；H2-005 用）

外部协作（不部署，仅对接）：
  - ERP / 码上放心 / 企微 / 快递 API（云上 SaaS）
```

### Dockerfile 模板（多阶段构建）

```dockerfile
# === Stage 1: cargo chef（依赖缓存）===
FROM rust:1.75-bookworm AS chef
WORKDIR /app
RUN cargo install cargo-chef sccache

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ENV RUSTC_WRAPPER=sccache SCCACHE_DIR=/sccache
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/sccache \
    cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN --mount=type=cache,target=/sccache \
    cargo build --release --bin wms-api

# === Stage 2: 运行时（distroless 最小镜像）===
FROM gcr.io/distroless/cc-debian12 AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/wms-api .
COPY shared/openapi/openapi.json ./shared/openapi/
USER nonroot:nonroot
EXPOSE 8080
ENTRYPOINT ["./wms-api"]
```

**关键约束**：
- 镜像 < 100 MB（distroless）
- 不含构建工具（distroless 无 sh / cargo）
- 非 root 用户（USER nonroot:nonroot）
- 健康检查接口（/healthz / /readyz）

### docker-compose 模板（小型 3PL）

```yaml
# deploy/docker-compose.yml（最小可运行示例）
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_DB: wms
      POSTGRES_USER: wms
      POSTGRES_PASSWORD_FILE: /run/secrets/db_password
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./initdb:/docker-entrypoint-initdb.d:ro
    secrets:
      - db_password
    healthcheck:
      test: ["CMD", "pg_isready", "-U", "wms"]

  wms-api:
    image: wms/api:${WMS_VERSION:-latest}
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      WMS_DB_URL: postgres://wms@postgres/wms
      WMS_DB_PASSWORD_FILE: /run/secrets/db_password
      WMS_LOG_LEVEL: info
    secrets:
      - db_password
      - wms_jwt_signing_key
    ports:
      - "8080:8080"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/healthz"]

  wms-web:
    image: wms/web:${WMS_VERSION:-latest}
    depends_on:
      - wms-api
    ports:
      - "80:80"

  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus_data:/prometheus

  loki:
    image: grafana/loki:latest
    volumes:
      - loki_data:/loki

  grafana:
    image: grafana/grafana:latest
    depends_on:
      - prometheus
      - loki
    ports:
      - "3000:3000"
    volumes:
      - grafana_data:/var/lib/grafana

volumes:
  postgres_data:
  prometheus_data:
  loki_data:
  grafana_data:

secrets:
  db_password:
    file: ./secrets/db_password.txt   # .gitignore 已屏蔽
  wms_jwt_signing_key:
    file: ./secrets/jwt_signing_key.txt
```

### Kubernetes 模板（大型）

`deploy/k8s/`（Wave 4-5 启动时按需补全）：

- `namespace.yaml`
- `configmap.yaml`（部署配置 / Wave 与 ADR-0013 L2）
- `secret.yaml`（环境 secrets，正式 prod 用 Vault Provider）
- `deployment-wms-api.yaml`（rolling update strategy）
- `service-wms-api.yaml`
- `ingress.yaml`
- `hpa-wms-api.yaml`（按 CPU / 自定义 metric 弹缩）
- `pdb-wms-api.yaml`（PodDisruptionBudget）
- `prometheus-stack.yaml`（kube-prometheus-stack chart）

### CI/CD 流程

| 阶段 | 工具 | 触发 |
|---|---|---|
| **PR 检查** | GitHub Actions / GitLab CI | PR 创建 |
| **Build + Test** | cargo + pnpm | 自动 |
| **T1 quick-check** | governance 脚本 | PR 必过 |
| **T2 full check** | governance 脚本 | merge 前 |
| **镜像构建** | docker buildx + push | merge 到 main |
| **dev 自动部署** | ArgoCD / FluxCD | 镜像就绪 |
| **staging 半自动** | 主管手动审批 | 镜像就绪 + 测试通过 |
| **prod 部署** | 主管手动审批 + 双人确认 | staging 验证 + 业务方签字 |

### 滚动升级 + DB Migration 兼容窗口

**约束**：DB schema 变更必须支持 N-1 与 N 版本应用同时运行：

| 变更类型 | 兼容方式 |
|---|---|
| 加字段 | 先加（可空），下次升级再设非空 / 加约束 |
| 删字段 | 先停止使用（代码不用），下次升级再删 |
| 改字段类型 | 加新字段 + 数据迁移 + 切换 + 下次升级删旧字段（4 步走）|
| 重命名 | 视为删 + 加（不直接 rename）|

**Migration 工具**：sqlx migrate，迁移文件入库版本控制。

---

## 后果

### 正面

- **多场景适配**：小型 3PL 一键 docker-compose up；大型连锁有 k8s 高可用
- **环境一致性**：local/dev/staging/prod 共用 image，仅 ConfigMap/Secret 不同
- **滚动升级**：零停机部署
- **可回滚**：`docker-compose pull && up` 或 `kubectl rollout undo`

### 负面

- **维护两套部署**：docker-compose + k8s manifests 两个目录
- **应对**：core image 共用；compose 仅 dev/小客户；k8s 大型客户

### 风险

- **Migration 兼容窗口违反**：开发者直接删字段 → N-1 应用炸库
- **应对**：governance 脚本 + PR review 强制四步走
- **distroless 调试难**：无 sh 无法 exec 进容器
- **应对**：dev 用 debian-slim 镜像；prod 才 distroless

---

## 实施约束

1. 所有应用必须健康检查（`/healthz` 存活 + `/readyz` 就绪）
2. 镜像 tag 用 git commit sha + version（不允许 `latest` 进 prod）
3. prod 部署必须双人审批（运维 + 仓库主管或货主）
4. DB migration 4 步走必须遵守（治理脚本 Wave 1 启动后引入）
5. Secrets 走 ADR-0013（k8s Secret / Vault），不入 image
6. 观测三件套（Prometheus + Loki + Grafana）必随主应用部署

---

## 参考

- distroless: https://github.com/GoogleContainerTools/distroless
- cargo-chef: https://github.com/LukeMathWalker/cargo-chef
- k3s: https://k3s.io/
- ArgoCD: https://argo-cd.readthedocs.io/

## 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：双轨部署 + Dockerfile 模板 + docker-compose 示例 + k8s 清单 + CI/CD + Migration 4 步走 |
