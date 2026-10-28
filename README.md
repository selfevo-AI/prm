# PRM · 个人资源与关系管理

基于 `Three_Worlds_Economic_Model_AI_and_Embodied_Intelligence_EN.docx` 开发的本机应用。Rust 后端 + PostgreSQL + React 中文界面，默认离线运行。

PRM 是一个基于本体论框架的个人信息、Resource（资源）与 Relation（关系）的管家与 Agent。R 同时表示 Resource 与 Relation：自我本体既包含身体与心理这些可调用的资源，也包含本人与外界、与自我的关系。所有实体按**世界**（自我、思维、数字、物理）、**评价维度**（需要、欲望、存在）和**人生场景**（自我、情感、生活、工作、健康、学习、财务、社交）三组相互独立的维度组织，上位范畴采用 REA（资源—事件—主体）本体并扩展承诺、关系与观察。

当前是 0.4 版本：

- 自我本体加入主流心理学量表目录与“我是……”自我认同、自我关系立场。
- 关系网络管理与他人的关系，以及互动流水。
- 数字界提供面向主流 AI Agent 的导入接口（HTTP / OpenAPI、MCP）。导入经路由交给离线规则或本地模型，本地模型以本人发布的核心原则和可见的自我本体为依据处理。结果统一进入决策中心，由本人审核后纳入本体。
- 物理界按 PRP（个人资源计划，Personal Resource Planning）设计：物料、库位、供应商、采购订单，以及收货与付款流程。

## 启动

在 PowerShell 中运行：

```powershell
cd D:\prm
.\scripts\Start-PRM.ps1 -Build
```

默认地址为 `http://127.0.0.1:4317`，端口占用时自动使用下一个端口，脚本会打印实际地址和进程 PID。首次打开由用户本人设置账户口令，思维库初始为空。不要把真实口令提交到代码、聊天记录或环境变量中。

需要本地模型时加 `-ModelWorker`：脚本会为本次运行生成执行器令牌，并同时启动 `scripts/prm-model-worker.mjs`（见 [本地模型与路由](docs/LOCAL-AI.md)）。

当前机器已安装 Rust GNU 工具链，项目内 `.tools/w64devkit` 提供 C 编译器。本机 `.tools/pgsql` 提供 PostgreSQL 17.9；启动脚本会启动绑定 127.0.0.1:55432 的数据库，创建非超级用户 `prm`。随机数据库凭证保存在只允许当前用户访问的 `.tools/postgresql.json`，不应提交该文件。已有数据库可直接设置 `PRM_DATABASE_URL`，启动脚本将使用它。数据库连接只接受显式环回地址。

其他机器需安装 PostgreSQL 17+（或将官方二进制解压到 `.tools/pgsql`），创建非超级用户应用角色及其专用数据库，再设置 `PRM_DATABASE_URL=postgresql://<user>:<password>@127.0.0.1:<port>/<database>`。建议显式设置 `PRM_DATABASE_SCHEMA=prm_main` 便于迁移和恢复。另需 Node.js 22+、Rust stable，以及对应平台的 C 编译工具。Windows 推荐 Rust MSVC + Visual Studio C++ Build Tools；也支持 GNU + MinGW。`.cargo/config.toml` 为 GNU 使用 Rust 自带运行库，不影响 MSVC/Linux/macOS。

## 已实现

| 模块 | 可操作流程 |
| --- | --- |
| 工作台 | 任务、建议、净现金流、资源与联系提醒；人生场景概览，点选后全局按场景筛选 |
| 自我本体 | 生理指标（睡眠、心血管、体成分、活动、体感）；心理量表目录（RSES、SCCS、CSES、GSES、SCS-SF、BPNSFS、SWLS、WHO-5、Ryff、I-PANAS-SF、PSS-10、BRS、BFI-2-S、MAAS、WLEIS、ECR-R）按范围录入得分并显示趋势；“我是……”自我认同；自我关系立场；逐条 Agent 可见性与核心原则预览 |
| 关系网络 | 人物与人际关系（类型、IOS 亲密度、RAS 满意度、联系节奏）；互动流水；超过节奏时生成联系建议 |
| 思维体系 | 原则、目标、反思；本人签名、口令确认后修改；按条目、公钥、有效期只读授权与撤销；版本历史；发布执行方向（可限定场景，作为本地模型的核心原则）及撤回 |
| 数字空间 | 多会话消息、离线助理反馈、知识条目、执行方向；Agent 接入：外部 Agent 令牌、导入记录、本地模型、路由规则、路由运行 |
| 物理资源 | 物料主数据（消耗品/资产、SKU、再订货点、补货量、交货周期、保修）、库位、供应商；采购订单草稿 → 下单 → 收货 → 付款 → 完结；入库、出库、盘点调整、不可覆盖的库存与现金流水 |
| 决策中心 | 统一建议队列：建议任务、纳入实体、建立关系；显示来源、执行方式与引用的核心原则；修改标题、关联执行方向后采纳或拒绝 |
| 本体关系 | 四个世界中的类、REA 范畴、评价维度与人生场景；按自我、外部、资源、数字、来源五层展示关系约束；实体浏览与手工建立关系 |
| 活动记录 | 最近 200 条操作、执行者（本人、离线助理、本地执行器、外部 Agent、只读接收者）、业务 JSON 导出 |

“添加业务演示数据”是用户主动触发的操作，只添加业务样例，不创建任何个人思维条目或自我记录。测试使用独立的临时目录，与 `data/` 无关。

## 设计依据

原文把三个世界视为交叉维度；“需要 / 欲望 / 存在”是独立的评价轴。软件保留这一点，并加入与二者独立的人生场景；任务可同时属于数字和物理世界，任何实体可关联多个评价维度和场景。思维安全级别独立于世界与场景标签，不能通过修改标签绕过权限。

- [架构与原文映射](docs/ARCHITECTURE.md)
- [安全边界与备份](docs/SECURITY.md)
- [本体及 API 约定](docs/ONTOLOGY-API.md)
- [自我本体与量表目录](docs/SELF-MODEL.md)
- [外部 AI Agent 接入](docs/AGENT-INTEGRATION.md)（[OpenAPI](docs/agent-openapi.json)）
- [本地模型与路由](docs/LOCAL-AI.md)
- [验证记录](docs/VALIDATION.md)

## 开发与验证

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;D:\prm\.tools\w64devkit\bin;$env:Path"
.\scripts\Start-Postgres.ps1 # 或事先设置 PRM_DATABASE_URL
npm.cmd ci
npm.cmd run build
cargo build --locked
$env:PRM_DATABASE_SCHEMA = $null # 测试按临时目录生成独立 schema
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
npx.cmd playwright install chromium
npm.cmd run test:e2e
```

浏览器测试会启动独立的 4318 端口服务，在 `test-data/` 创建测试账户和合成数据，输出截图到 `test-results/`。前端开发可另开终端运行 `npm.cmd run dev`，Vite 会将 `/api` 转发给 4317 端口的 Rust 服务。

## 数据与范围

业务与加密精神快照存入 PostgreSQL，使用同一个工作区 schema 和事务。`data/` 保留进程锁与旧 SQLite 原始文件；旧数据在空 PostgreSQL 工作区首次启动时自动只读导入，原文件不再更新。0.4 启动时会为既有工作区补充人生场景列并同步本体约束，原有记录保持不变（旧版心理自评显示为“旧版自评”）。完整备份应改用 `pg_dump`，不能只复制 `data/`，详见安全文档。

核心精神的“只读授权设置”绑定接收者设备公钥与选中的条目，只有本人能修改。接收者打开 `/shared` 取得公钥并提交给本人，再用授权编号访问。读取仅在本人思维库解锁期间有效。授权不包含历史版本，不能提升为写权限。

安全验证采用 Ed25519 公私钥签名、请求内容摘要、时间戳与持久化防重放记录。浏览器通过 Web Crypto 保管不可导出私钥。外部 Agent 与本地模型执行器使用各自独立、可撤销的令牌，只能调用固定形状的接口，所有输出都要本人审核。

首版是单机、单用户版本，不具备多设备同步、Windows Hello、商业 IM 联网、银行支付、完整财务会计或机器人控制功能。PRP 部分是资源与现金管理的基础，不把库存估值、现金流和人的价值相加；心理量表只记录本人提供的得分，不做诊断，也不合成单一分数。实际本地大模型需要另行安装。
