# 本体与 API（0.4）

## 表结构

`ontology_classes` 注册业务类；`entities` 存储具有稳定 UUID、类、世界维度、评价维度、人生场景、属性、状态、版本和时间戳的实体（JSON 列为 TEXT 并由 CHECK 校验）。`relations` 以 subject/predicate/object 连接实体，具有外键、自关联禁止和唯一约束。类与谓词定义在 `src/ontology.rs` 注册表中；服务启动时由注册表同步 `ontology_classes` 和 `relations_predicate_check`，谓词的 domain/range 签名由 Rust 再次校验。

`vault` 只存储一个版本化认证密文。其内部的 `principle`、`goal`、`reflection` 不注册到业务实体表，泛化接口无法创建、读取或覆盖思维条目。`agent_credentials` 保存外部 Agent 连接（令牌只存 SHA-256）。

## 类

| 类 | 范畴 | 世界 | 默认场景 | 核心属性 |
| --- | --- | --- | --- | --- |
| principle / goal / reflection | 资源（加密） | 思维 | — | 本人文字、价值维度、版本、确认记录 |
| self_profile | 主体 | 自我 | 自我 | 自我本体锚点 |
| person | 主体 | 物理 | 社交 | 姓名、备注、`agent_visible` |
| supplier | 主体 | 物理 | 财务 | 联系方式、税号、备注 |
| identity | 关系 | 自我 + 思维 | 自我 | “我是……”陈述、TST 类别、重要程度 1–5 |
| self_relation | 关系 | 自我 | 自我 | 立场、强度 1–5、关联对象 |
| relationship | 关系 | 自我 | 按类型 | 类型、IOS 亲密度 1–7、RAS 满意度 1–5、联系节奏、最近联系 |
| body_metric / psychological_metric | 观察 | 自我 + 物理 / 思维 | 量表默认 | metric、instrument、value、unit、range、observed_at |
| interaction | 事件 | 自我 | 随关系 | 日期、方式、情绪体验 −2…2、备注 |
| directive | 资源 | 数字 | 全部 | 本人发布的执行要求（核心原则） |
| channel / message / note | 资源 / 事件 / 资源 | 数字 | — / — / 学习 | 会话、消息、知识正文 |
| proposal | 承诺 | 数字 | 随来源 | action、来源与版本、理由、执行方式、引用原则 |
| task | 承诺 | 数字 + 物理 | — | 说明、日期、directive_id、状态 |
| resource | 资源 | 物理 | 生活 | 单位、库位、再订货点、补货量、交货周期、类型（消耗品/资产）、SKU、供应商、单位成本、保修 |
| location | 资源 | 物理 | 生活 | 编码、上级库位 |
| stock_movement | 事件 | 物理 | 随物料 | 类型（入库/出库/盘点调整/期初）、变动、结存、订单行 |
| cash_entry | 事件 | 物理 | 财务 | 整数分、收支方向、日期、分类、币种、采购订单 |
| purchase_order / purchase_order_line | 承诺 | 物理 | 财务 | 状态、总额、已付；订购数量、已收数量、单价 |
| agent_import / agent_run | 事件 | 数字 | — | 导入信封（原样）、路由与执行状态 |
| model_endpoint / route_rule | 资源 | 数字 | — | 本地模型地址与标识；路由优先级、匹配条件、执行方式 |

## 关系（谓词）

| 层 | 谓词 | 签名 | 建立方式 |
| --- | --- | --- | --- |
| 自我关系 | `describes_self` | 指标、自我关系、自我认同 → 自我本体 | 本人 |
| 自我关系 | `relates_to_self` | 自我关系、指标、认同、人际关系 → 自我本体 | 本人 |
| 自我关系 | `measures` | 指标 → 自我本体、资源、任务、人际关系 | 本人 |
| 自我关系 | `supports` | 资源、任务、知识、方向、人际关系、人物 → 自我本体、指标、认同 | 本人 |
| 自我关系 | `influences` | 自我本体、指标、认同、自我关系、人际关系 → 任务、资源、方向 | 本人 |
| 外部关系 | `involves` | 人际关系 → 人物 | 系统 |
| 外部关系 | `occurs_in` | 互动 → 人际关系 | 系统 |
| 资源关系 | `uses` | 任务 → 资源 | 本人，Agent 可建议 |
| 资源关系 | `supplies` | 供应商 → 资源、采购订单 | 本人，Agent 可建议 |
| 资源关系 | `stored_at` | 资源 → 库位 | 本人，Agent 可建议 |
| 资源关系 | `located_in` | 库位 → 库位 | 系统 |
| 资源关系 | `contains` / `orders` | 采购订单 → 订单行 → 资源 | 系统 |
| 资源关系 | `receives` / `records` | 库存流水 → 订单行 / 资源 | 系统 |
| 资源关系 | `pays` | 现金流水 → 采购订单 | 系统 |
| 数字关系 | `implements` | 任务 → 执行方向 | 本人，Agent 可建议 |
| 数字关系 | `references` | 知识、任务、方向、资源、人物、供应商、库位、人际关系、认同、自我关系、自我本体、指标之间 | 本人，Agent 可建议 |
| 数字关系 | `belongs_to` | 消息 → 会话 | 系统 |
| 来源追溯 | `derived_from` | 建议 → 消息、资源、人际关系、导入；任务、资源、知识、人物、供应商、库位 → 建议 | 系统 |
| 来源追溯 | `processes` | 路由运行 → 导入 | 系统 |

这是带类型约束的属性图模型，落地在关系型 PostgreSQL 中，不是完整的 RDF/OWL 推理平台。首版不开放任意自定义类或关系，避免通过扩展绕过安全边界。

## HTTP 约定

服务地址默认 `http://127.0.0.1:4317`。所有修改接口使用 POST + JSON + `X-PRM-Request: 1`，请求体不超过 128 KiB，所有请求结构拒绝未知字段（422）。三类调用方的能力互不相通：

- **本人**：HttpOnly 会话 + 每个请求的 Ed25519 签名（见 SECURITY.md）。
- **本地模型执行器**：`Authorization: Bearer <PRM_AGENT_TOKEN>`，只能访问 `/api/agent/*` 的执行器接口。
- **外部 Agent**：`Authorization: Bearer <本人签发的连接令牌>`，只能访问导入与自身导入状态。

错误码：400 参数错误，401 未登录或令牌无效，403 越权，404 不存在，409 版本或状态冲突，413 请求过大，422 JSON 结构错误，423 思维库锁定，429 验证限速。

### 本人接口

| 路径 | 方法 | 用途 |
| --- | --- | --- |
| `/api/auth/status`、`/api/auth/setup`、`/api/auth/login`、`/api/auth/logout` | GET/POST | 账户与会话（setup/login 携带 `public_key`） |
| `/api/vault`、`/api/vault/unlock`、`/api/vault/lock` | GET/POST | 解锁、锁定、读取思维库 |
| `/api/vault/entries`、`/api/vault/review` | POST | 本人创建/编辑、复核思维条目（口令 + `confirmed` + `expected_version`） |
| `/api/vault/publish` | POST | 把条目发布为执行方向；可带 `scenes`（空表示全部场景） |
| `/api/vault/grants`、`/api/vault/grants/revoke`、`/api/shared/vault` | GET/POST | 只读授权与接收者读取 |
| `/api/workspace` | GET | 全部业务实体、关系、最近审计与本体 `schema` |
| `/api/entities` | POST | 创建资源、库位、供应商、采购订单、订单行、任务、知识、会话、人物（可带 `scenes`） |
| `/api/relations` | POST | 按注册表建立本人可建立的关系 |
| `/api/tasks/{id}/status`、`/api/directives/{id}/revoke` | POST | 任务状态、撤回执行方向（`expected_version`） |
| `/api/messages`、`/api/analyze` | POST | 收集消息、运行离线分析 |
| `/api/proposals/{id}/decide` | POST | `{decision, expected_version, title?, directive_id?}`，按建议动作创建任务、实体或关系 |
| `/api/self/catalog` | GET | 量表与指标目录、立场、认同类别、关系类型、互动方式 |
| `/api/self/profile` | GET | 自我本体、指标、认同、自我关系 |
| `/api/self/assessments` | POST | `{instrument, observed_at, scores:[{metric,value}], notes?, agent_visible?, scenes?}`；自定义指标用 `instrument:"custom"` 与 `custom_label/unit/domain` |
| `/api/self/identities` | POST | `{statement, category, centrality, scenes?, agent_visible?}` |
| `/api/self/relations` | POST | `{title, stance, intensity, target_id?, notes?, scenes?, agent_visible?}` |
| `/api/self/visibility` | POST | `{id, agent_visible, expected_version}`，逐条控制本地模型可见性 |
| `/api/self/agent-preview` | GET | 本地模型实际收到的核心原则上下文 |
| `/api/relationships` | POST | `{person_id 或 person_name, type, closeness, satisfaction?, contact_interval_days, notes?, scenes?, agent_visible?}` |
| `/api/relationships/{id}` | POST | 调整亲密度、满意度、节奏、状态、场景（`expected_version`） |
| `/api/relationships/{id}/interactions` | POST | `{date, channel, valence, notes?, expected_version}` |
| `/api/resources/{id}/movement` | POST | `{delta, reason, expected_version, movement_type?}`，类型 receipt/issue/adjustment |
| `/api/cash` | POST | 收支流水，金额 `amount_cents`，可带 `scenes` |
| `/api/purchase-orders/{id}/status` | POST | `{status: ordered/cancelled/closed, expected_version}` |
| `/api/purchase-orders/{id}/receive` | POST | `{lines:[{line_id, quantity}], note?, expected_version}` |
| `/api/purchase-orders/{id}/pay` | POST | `{amount_cents, date, title?, expected_version}`，不超过未付金额 |
| `/api/agents/connections` | GET/POST | 列出 / 签发外部 Agent 连接（令牌只返回一次） |
| `/api/agents/connections/revoke` | POST | `{id}` |
| `/api/agents/models` | POST | `{title, provider: ollama/openai_compatible, base_url（仅回环 http）, model}` |
| `/api/agents/routes` | POST | `{title, priority, agent_id?, scene?, item_kind?, executor: "offline-rules" 或模型 id}` |
| `/api/agents/config/{id}/status` | POST | 启用/停用模型或规则 |
| `/api/demo`、`/api/export` | POST/GET | 业务演示数据；业务导出 `prm-operations-v2`（不含思维库与令牌） |

### 外部 Agent 接口

见 [AGENT-INTEGRATION.md](AGENT-INTEGRATION.md) 与 [agent-openapi.json](agent-openapi.json)：`POST /api/agent/import`、`GET /api/agent/imports`、`GET /api/agent/imports/{id}`。

### 执行器接口

| 路径 | 方法 | 用途 |
| --- | --- | --- |
| `/api/agent/jobs/claim` | POST | `{worker?}`；返回 `{job}` 或 `{job: null}`。job 含导入内容、模型配置、核心原则上下文、输出约定与租约（300 秒，最多 3 次） |
| `/api/agent/jobs/{id}/result` | POST | `{lease, summary?, candidates}`；逐条校验，未通过的候选在 `rejected` 中返回原因 |
| `/api/agent/jobs/{id}/fail` | POST | `{lease, error}`；未满 3 次重新排队 |
| `/api/agent/context` | GET | 业务上下文 + 核心原则上下文 |
| `/api/agent/proposals` | POST | 固定形状的任务建议 `{source_id, source_version, title, rationale}`，同一来源版本只能一条 |

候选结构（`prm-candidates-v1`）：

```json
{
  "ref": "r1",
  "action": "create_task | create_entity | create_relation",
  "kind": "resource | location | supplier | note | person",
  "title": "≤ 120 字",
  "body": "任务说明，≤ 4000 字",
  "properties": {"unit": "台", "location": "会议室", "minimum": 0},
  "scenes": ["work"],
  "values": ["need"],
  "subject": "#p1 或实体 id",
  "predicate": "references | uses | supplies | stored_at | implements",
  "object": "#r1 或实体 id",
  "rationale": "依据、与核心原则的关系、需要本人核实的事项",
  "principles": ["执行方向或自我认同 id"]
}
```

候选不能创建现金、库存、思维条目、自我记录或任何权限；`create_entity` 的属性与本人表单使用同一套校验，并在本人采纳时按当时的数据再校验一次。引用同批候选的关系，需要先采纳被引用的候选。
