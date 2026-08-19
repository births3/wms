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

### 灰度发布策略（v3.1，关联软件设计审计 §4.12）

**目的**：版本上线时控制爆炸半径——不是"all 或 nothing"切换，而是按 **租户 / 百分比 / 用户群体** 分阶段放量。

> **ADR 性质声明**：本节为**方向性决策**——确定"做不做、按什么思路做"，**不**承诺实施级细节（具体百分比、阈值、命名、目录路径、审批主体）。所有实施级细节由 W1.D（治理脚本）/ Wave 1 末 retro / Wave 2 迁移任务的工作产出回写至 ADR（v3.2 起）。在那之前，本节的具体数字 / 路径 / 主体是**初稿建议**，不构成"违反 ADR"的依据。读者应理解为"方向已定，细节待校准"。
>
> **数字声明**：本节所有比例 / 阈值（5% / 50% / 基线 × 1.2 / 30% 错误预算 等）均为业界常见默认值（参 Google SRE Workbook ch.16 "Canarying Releases"、AWS Prescriptive Guidance "Canary deployments"），**不是 wms 实测校准值**。Wave 1 末首次正式上线后，必须用真实业务流量数据（医药 PDA 高频但单机用量小、PC 端低频写）校准并以本 ADR v3.2 形式回写；校准前作为初版默认。

#### 灰度三维度

| 维度 | 适用场景 | 实现方式 |
|------|---------|---------|
| **按租户（owner_id）灰度** | SaaS 多货主部署；先放给"白名单货主"验证 | k8s ingress + header `X-Owner-Id` 路由到 canary deployment；或后端读 `WMS_CANARY_OWNERS` 环境变量做特性开关 |
| **按百分比灰度** | 单租户大客户；按 hash(user_id) % 100 比例放量 | k8s service mesh（Istio / Linkerd）流量切分；或 nginx `split_clients` |
| **按用户群体灰度** | 内部员工 → 仓库主管 → 全员 | 读 H1 用户角色 / 标签，命中则路由到 canary |

#### 放量节奏（默认四阶段）

| 阶段 | 流量占比 | 持续时间 | 通过判据 |
|------|---------|---------|---------|
| Stage 0 内部 | 内部测试账号（< 1%）| ≥ 1 天 | 无 P0 缺陷 + SLO 不退化 |
| Stage 1 早期 | 5% | ≥ 2 天 | 错误率 ≤ 基线 × 1.2 + p99 ≤ 基线 × 1.3 |
| Stage 2 半量 | 50% | ≥ 2 天 | 同上 + 业务方主动反馈无阻塞 |
| Stage 3 全量 | 100% | — | 旧版本保留 7 天，可秒级回滚 |

> 紧急 hotfix 可走单阶段全量（运维 + 仓库主管双签）。

#### 自动回滚阈值（Auto Rollback）

放量过程中**任一项触发**即立即回滚到上一稳定版本：

| 指标 | 阈值 | 来源 |
|------|------|------|
| HTTP 5xx 错误率 | > 1% 持续 5 min | Prometheus（参 ADR-0011） |
| API p99 延迟 | > 基线 × 2 持续 5 min | 同上 |
| 监管上报失败率（M-TC）| > 0.1% 持续 1 min | M-TC SLI |
| 审计写入失败率（H2）| > 0% 持续 1 min（任何失败立即回滚）| H2 SLI |
| 错误预算月度消耗 | 单次发布消耗 > 30% | usability-baseline §6.2 |
| 业务方人工告警 | 主管在告警群 ack `STOP` | 人工兜底 |

**回滚操作**：
- k8s：`kubectl rollout undo deployment/wms-api`（< 30 秒）
- docker-compose：`WMS_VERSION=<prev-sha> docker compose up -d`（< 1 分钟）
- 数据库：**不自动回滚 schema**——依赖 §"滚动升级 + DB Migration 兼容窗口"的四步走保证 N-1 应用兼容 N schema

#### Feature Flag（特性开关）

代码内灰度的最后一公里。所有**未确定全量启用**的新功能必须用 Feature Flag 包裹：

```rust
// 后端示例（伪代码）
if feature_flags.is_enabled("m4_outbound_v2_picker", &owner_id, &user_id) {
    new_picker_logic()
} else {
    legacy_picker_logic()
}
```

**Feature Flag 治理**：

- **存储（分波次降级）**：
  - **Wave 1（M1-008 配置中心未就绪）**：用环境变量 `WMS_FEATURE_*` 或 `deploy/feature_flags.toml` 文件做最小存储；运维改 flag = 改文件 + 滚动重启；不支持运行时热更新
  - **Wave 2 起（M1-008 业务配置中心上线后）**：迁移到配置中心同源（ADR-0013 L2 运行时配置）；支持运行时热更新；key 命名 `<module>_<feature>_<phase>`，如 `m4_outbound_v2_picker_stage1`
  - **Wave 1 → Wave 2 迁移路径**（方向性，5 步走；具体脚本路径 / 命名 / 审批主体由 W2 迁移任务实施时确定，回写本 ADR v3.2+）：
    1. 从 `deploy/feature_flags.toml` + 环境变量快照导出当前 flag 全集（含 owner / 创建日期 / 当前状态）
    2. 调用 M1-008 写入 API 批量导入；导入后立即对账（diff 应为空，**对账思路对齐 ADR-0014 §数据校验规则**——仅借鉴"迁移前后对账"原则，不套用 ADR-0014 的 CDC / 双写大型迁移方案）
    3. 应用切换读取源（feature flag SDK 改 backend）；灰度部署验证
    4. 旧 TOML 文件归档（具体归档路径与命名约定 W2 实施时定）；环境变量从 docker-compose / k8s manifest 删除
    5. 迁移脚本（W2 实施时新建，路径与脚本名待定；运行需双人审批，**审批主体（运维 / 仓库主管 / DBA / 业务方）由 W2 实施任务定义并回写**）
- **Wave 2 v3.2 实施回写**：
  - 配置中心 Feature Flag API 由 `wms-api` OpenAPI 暴露：
    - `POST /api/v1/config-center/feature-flags/migrate`
    - `POST /api/v1/config-center/feature-flags/import`
    - `GET /api/v1/config-center/feature-flags/export`
    - `GET /api/v1/config-center/feature-flags/reconcile`
    - `POST /api/v1/config-center/feature-flags/source`
    - `POST /api/v1/config-center/feature-flags/archive-file-source`
  - 开发完成静态证据由 `just wave-2-complete-check` 校验；该检查覆盖 M1-008 迁移、批量导入、导出、对账、切换读取源、旧文件归档和 OpenAPI schema。
  - 真实 dev/staging 灰度证据按 `docs/runbooks/wave-2-runtime-evidence.md` 采集，写入 `docs/retros/wave-2-runtime-evidence.json`，并由 `just wave-2-runtime-evidence-validate` 作为预发布 gate 校验。
  - 当前若没有稳定 dev/staging，不得用 localhost / mock / fake / example 证据替代；只能标记为预发布 gate。
- **生命周期**：每个 flag 必须有 **owner + 创建日期 + 计划清理日期**；默认 90 天后必须清理（要么全量启用，要么删功能）
- **审计**：flag 状态变更纳入 H2 审计追踪（actor / before / after / approval_source）
- **治理脚本**（与灰度链路同步落地于 Wave 1，参 ROADMAP W1.D）：`check_feature_flags.py` 校验过期 flag 必须清理

#### 与 ADR / 故事的关联

| 关联点 | 说明 |
|--------|------|
| ADR-0011 可观测性 | 灰度阶段的 SLO 监控、自动回滚阈值数据来源 |
| ADR-0013 配置 secrets | Feature Flag 存于 L2 运行时配置中心 |
| ADR-0015 多端规则 | 前端业务规则灰度同样走 Feature Flag |
| H1-001 多租户 | "按租户灰度"复用 owner_id 隔离能力 |
| H2 审计追踪 | Flag 变更 + 灰度阶段切换写入审计 |
| usability-baseline §6 SLO | 自动回滚的"错误率 / p99"基线来自 SLO 表 |

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
- **灰度阶段 SLO 误判**：基线波动导致自动回滚误触发
- **应对**：基线取近 7 天 p99 移动平均；自动回滚需阈值连续 ≥ 5 分钟（短促波动不触发）
- **Feature Flag 长期残留**：90 天清理纪律若不执行，flag 数会爆炸
- **应对**：Wave 1 与灰度链路同步落地 `check_feature_flags.py`（参 ROADMAP W1.D）；过期未清理 PR 不予合并

---

## 实施约束

1. 所有应用必须健康检查（`/healthz` 存活 + `/readyz` 就绪）
2. 镜像 tag 用 git commit sha + version（不允许 `latest` 进 prod）
3. prod 部署必须双人审批（运维 + 仓库主管或货主）
4. DB migration 4 步走必须遵守（治理脚本 Wave 1 启动后引入）
5. Secrets 走 ADR-0013（k8s Secret / Vault），不入 image
6. 观测三件套（Prometheus + Loki + Grafana）必随主应用部署
7. **Wave 1 末（首次正式上线）必须有可工作的灰度 + 自动回滚链路**；不允许"全量直发"
8. **所有新功能默认包 Feature Flag**；flag 必须登记 owner + 90 天清理期

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
| 2026-05-18 | v3.1 | 增补 §灰度发布策略：三维度（租户/百分比/用户群体）+ 四阶段放量 + 6 项自动回滚阈值 + Feature Flag 治理（关联软件设计审计 §4.12 子项 b 闭环）。所有比例/阈值为业界默认值（Google SRE / AWS），待 Wave 1 末用 wms 真实流量校准 → v3.2 回写 |
| 2026-05-18 | v3.1.1 | Feature Flag 治理修订：存储分波次降级（Wave 1 用环境变量/TOML 文件，Wave 2 起迁配置中心 M1-008）；治理脚本 `check_feature_flags.py` 落地波次从 Wave 2 提前到 Wave 1（与灰度链路同步，参 ROADMAP W1.D）|
| 2026-05-18 | v3.1.2 | 补 Wave 1 → Wave 2 Feature Flag 迁移路径：5 步走 + 迁移脚本 `feature_flags_w1_to_w2.py` + 双人审批（关联 ADR-0014 数据迁移策略；ADR-0013 v1.1 同步加 cross-ref）|
| 2026-05-18 | v3.1.3 | §灰度发布策略顶部加「ADR 性质声明」明示方向性 ADR + 实施级细节由 W1.D / W2 迁移任务回写；修订迁移路径段去掉具体脚本名 / 目录 / 审批主体（B1-B3 修），改"参 ADR-0014"为"对账思路对齐 ADR-0014 §数据校验规则"避免过度引用 |
| 2026-06-03 | v3.2 | Wave 2 W2.G 实施回写：配置中心 Feature Flag API 路径、`just wave-2-complete-check` 静态完成门禁、`docs/runbooks/wave-2-runtime-evidence.md` + `just wave-2-runtime-evidence-validate` 预发布 runtime evidence 门禁；明确无稳定 dev/staging 时不得伪造证据 |
