# ADR-0013：配置与 secrets 管理

- 状态：Accepted
- 决策日期：2026-05-18
- 决策人：项目主人
- 关联：ADR-0001 / ADR-0010 / ADR-0011 / docs/governance.md §3.7

---

## 背景

软件设计审计 §4 维度 8 识别配置管理缺口：

- M1-008 业务配置中心已有（业务参数）
- governance §3.7 提到 .env 不入库 + gitleaks 扫描
- gsp-field-coding-standards §5 加密分级（none/masked/encrypted）
- **但缺**：
  - 12-Factor App #3：配置存于环境，wms 没明示
  - secrets（DB 密码、JWT 签名密钥、加密密钥）的统一管理
  - 多环境（local/dev/staging/prod）配置覆盖机制
  - 密钥轮换策略
  - 加密密钥的 KMS 集成

不解决会导致 Wave 1 H1 实施时密码硬编码 / 散在各处 / 无轮换。

---

## 候选方案

### 方案 A（推荐）：分层配置 + 外部 secrets 管理

```
启动时合并优先级：
  环境变量 > 部署 secrets（k8s secret / Vault）> 配置中心（M1-008）> 编译期默认
```

- 业务可配置（阈值/开关）→ M1-008 配置中心
- 部署相关（DB URL / 端口 / 副本数）→ 环境变量
- 凭据（DB 密码 / JWT key / 加密密钥）→ 外部 secrets 管理器

### 方案 B：全部用配置中心（M1-008）

**否决**：业务配置和部署/secrets 不该混（多环境时业务方误改 prod DB URL）。

### 方案 C：全部用环境变量

**否决**：业务参数频繁修改不该重启服务。

---

## 决策

**采用方案 A：三层分层配置 + 外部 secrets**。

### 配置三层

| 层 | 内容 | 来源 | 修改频率 |
|---|---|---|---|
| L1 编译期默认 | 业务默认值 / 端口默认 | Rust `Config::default()` | 极低 |
| L2 部署环境 | DB URL / 服务端口 / 副本数 / 监控地址 | 环境变量 / k8s ConfigMap | 部署时 |
| L3 业务运行时 | 业务参数（M1-008 中的 55 项配置）| DB 配置中心 | 运营期 |

### Secrets 管理

| 部署目标 | secrets 存储 | 加密 | 轮换 |
|---|---|---|---|
| local（开发）| `.env` + `.env.example` | 无（开发用）| 无 |
| dev / staging | k8s Secret | 静态加密 | 90 天 |
| prod（推荐）| HashiCorp Vault / AWS Secrets Manager | 动态密钥 + KMS | **强制 90 天** |

### 不入库的硬规则

参见 [governance.md §3.7](../governance.md#37) 的"不得入库的文件 / 内容"6 类清单：

1. 配置类（.env / .env.local / .env.production）
2. 密钥类（.pem / .key / id_rsa*）
3. 凭据类（DB 密码 / API key / OAuth secret / JWT key）
4. 客户/用户数据（生产 dump / PII 样本）
5. 大文件（> 5MB 二进制）
6. IDE 个人配置

**自动检测**：pre-commit `gitleaks` + 治理脚本 `check_secrets.py` + CI `cargo audit` / `pnpm audit`。

### Secrets 命名规范（环境变量）

```
WMS_<MODULE>_<KEY>
```

| 示例 | 含义 |
|---|---|
| `WMS_DB_URL` | 数据库连接串（含密码占位）|
| `WMS_DB_PASSWORD` | 数据库密码（独立，便于轮换）|
| `WMS_JWT_SIGNING_KEY` | JWT 签名密钥 |
| `WMS_JWT_REFRESH_KEY` | Refresh token 密钥 |
| `WMS_ENCRYPTION_MASTER_KEY` | 字段加密主密钥（DEK 派生） |
| `WMS_REGULATORY_API_KEY` | 码上放心 API 密钥 |
| `WMS_WECHAT_CORP_SECRET` | 企业微信 corpsecret |

**禁止**：`SECRET_KEY` / `KEY` / `PASSWORD` 等无前缀的通用名（与系统其他变量冲突 + 命中范围模糊）。

### 加密密钥分级（与 gsp-field-coding-standards §5 联动）

| 分级 | 适用字段 | 密钥层级 |
|---|---|---|
| `none` | 公开数据（商品编码/批号）| 无 |
| `masked` | 显示脱敏（IP/手机号）| 应用层规则，无密钥 |
| `encrypted` | 静态加密（法人身份证/银行账号/患者资料）| 主密钥 → DEK → 字段密钥（3 层）|

**主密钥（Master Key）轮换**：
- 默认 90 天
- 轮换时不重新加密历史数据（用 Key Version 标记）
- 备份/归档需要解密时按 Key Version 取对应密钥

### 多环境配置覆盖

```yaml
# local 开发
env_files:
  - .env.local
config_center: postgres-local

# dev / staging
env_files:
  - k8s ConfigMap: wms-config-dev
secrets:
  - k8s Secret: wms-secrets-dev
config_center: postgres-dev

# prod
env_files:
  - k8s ConfigMap: wms-config-prod
secrets:
  - Vault: wms/prod/*
config_center: postgres-prod
```

### 启动时配置合并算法

```rust
// 伪代码
fn load_config() -> AppConfig {
    let defaults = AppConfig::default();           // L1
    let from_env = AppConfig::from_env();          // L2: 环境变量覆盖
    let from_secrets = SecretsManager::load().await; // L2: secrets 管理器
    let from_db = ConfigCenter::load().await;      // L3: M1-008
    
    defaults
        .merge(from_env)
        .merge(from_secrets)
        .merge_runtime(from_db)  // 业务参数热加载
}
```

**热加载**：M1-008 配置变更通过 H2-005 事件总线推送，应用层订阅后刷新缓存（不重启服务）。

---

## 后果

### 正面

- **12-Factor 合规**：配置外置，环境差异通过 ConfigMap/Secret 表达
- **轮换可执行**：90 天密钥轮换有明确日历 + KMS 集成
- **GSP 合规**：加密分级与字段词典的 encryption 字段对齐
- **审计可追**：所有 secrets 访问写 H2 审计追踪

### 负面

- **运维成本**：Vault / KMS 增加运维复杂度
- **应对**：local/dev 用 k8s Secret 替代 Vault；prod 必上 Vault

### 风险

- **secrets 误入库**：开发者意外提交 `.env`
- **应对**：pre-commit gitleaks（已有）+ check_secrets.py（本 ADR 引入）
- **轮换失败**：旧密钥过期但新密钥未同步
- **应对**：轮换前 7 天双密钥并存（grace period）+ H-AL 告警

---

## 实施约束

1. **代码中禁止硬编码**：任何凭据/密钥必须从环境变量或 secrets 管理器读取
2. **测试用真实凭据禁止**：单元测试用 mock secrets，集成测试用临时账户
3. **日志脱敏**：所有日志不允许出现 secrets（参 ADR-0011 §PII 脱敏）
4. **轮换日历**：每 90 天主密钥轮换，由运维 cron 触发，写 H2 审计
5. **禁止用 .env 在 prod**：prod 必须用 k8s Secret 或 Vault
6. **不允许修改 .gitignore 取消 .env 屏蔽**（治理脚本 check_secrets.py 校验）

---

## 治理脚本

`scripts/governance/check_secrets.py`（T1 级，基于 gitleaks 之外的额外校验）：

- 扫描代码 + 配置文件，识别硬编码的 secret 模式（如 `password = "xxx"` / API key 字符串模式）
- 校验 `.gitignore` 包含必要的 secrets 屏蔽规则（.env / .pem / .key 等）
- 校验环境变量命名规范（WMS_<MODULE>_<KEY>）
- 校验 `.env.example` 占位用法（无真实值）

---

## 参考

- 12-Factor App #3 Config: https://12factor.net/config
- HashiCorp Vault: https://www.vaultproject.io/
- AWS Secrets Manager: https://aws.amazon.com/secrets-manager/
- OWASP Secrets Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html

## 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：三层配置 + 外部 secrets + 命名规范 + 加密密钥分级 + 治理脚本 |
