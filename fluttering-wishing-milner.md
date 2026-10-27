# PRM 0.4 重构方案：Resource × Relation 本体管家

## Context
用户要求在现有框架上重构 PRM：R = Resource + Relation；自我本体同时包含与外界、与自我的关系，并加入生理与主流心理学自我认定指标；数字界提供主流 AI Agent 导入接口，经 router 交给本地模型、以自我本体为核心原则处理，结果统一纳入 PRM；物理界按 ERP 思路设计；整体按人生场景（自我/情感/生活/工作/健康/学习/财务/社交）组织。
用户已确认：HTTP 导入 API + OpenAPI、MCP 工具桥；心理指标 = 量表目录 + 录入得分（不内置题目）；8 个场景；独立执行器进程（服务不对外发请求）。

现状缺口：心理指标是 0–10 自由打分；无人际关系层与场景维度；/api/agent/router 只建 queued 记录无人处理；agent_import 本人不可见不可审；只有单一 env 令牌；供应商/采购订单无 UI、无状态机/收货/付款；本体定义分散 4 处；App.tsx 2600 行单文件。

## 设计要点
1. 本体注册表（src/ontology.rs）为唯一来源：ClassDef{id,label,category,worlds,scenes,authority}、PredicateDef{id,label,layer,owner_settable,agent_proposable,signatures}；worlds()/relation_valid()/schema() 由其派生；启动时 sync_sql() 同步 ontology_classes 与 relations_predicate_check（002 SQL 去掉硬编码列表）。范畴采用 REA：主体/资源/事件/承诺 + 关系/观察。
2. 场景 scenes：entities 新列（ADD COLUMN IF NOT EXISTS，默认 '[]'）；各类有默认场景；所有创建接口可选 scenes；UI 全局场景筛选。
3. 自我本体（src/self_model.rs）：静态量表目录（生理 5 组；心理 RSES、SCCS、CSES、GSES、SCS-SF、BPNSFS、SWLS、WHO-5、Ryff PWB、I-PANAS-SF、PSS-10、BRS、BFI-2-S、MAAS、WLEIS、ECR-R；关系 IOS、RAS）仅元数据+出处；POST /api/self/assessments 按范围校验；identity（TST「我是…」，类别/中心度）；self_relation 立场词表（接纳/关怀/苛责/信任/理想自我/应然自我/边界/成长，Higgins）；可见性开关；Agent 可见预览。永不合成总分。
4. 关系网络（src/relationships.rs，个人 CRM）：person、relationship（类型、IOS 亲密度、RAS 满意度、联系节奏、最近联系）、interaction 事件流水；involves/occurs_in；分析规则：联系到期 → 任务建议。
5. Agent 网关（src/agents/）：Actor 拆分 Executor（env PRM_AGENT_TOKEN，本地执行器）与 External(id)（本人签发的每 Agent 令牌，表 agent_credentials，只存哈希、只显示一次、可撤销）。External 只能 POST /api/agent/import（items 或 OpenAI 风格 messages）与查询自己导入状态。router：model_endpoint（仅回环 http）+ route_rule（优先级、按 Agent/场景/类型匹配），默认 offline-rules。离线规则同步结构化转换；模型路由排队，执行器 claim（租约 300s、最多 3 次）→ 取得 principles_context（已发布方向 + Agent 可见自我/关系，绝不含思维库）→ 调本地模型 → result。候选统一转为 proposal（create_task / create_entity / create_relation，引用同批候选需先采纳），在决策中心本人审核后纳入本体，derived_from 溯源。
6. ERP（src/erp.rs，REA）：物料主数据（类型 消耗品/资产、再订货点、补货量、交货周期、库位、供应商、保修）；location 层级；采购订单状态机 draft→ordered→partially_received→received→closed/cancelled；订单行仅草稿可加并重算总额；按行收货生成 receipt 流水（receives、records）并更新已收数量；付款生成现金流水（pays，不超过未付）；流水类型 receipt/issue/adjustment；保修到期建议。
7. 前端：App.tsx 拆为 web/pages/*、web/dialogs/*；新增「关系网络」页、数字空间「Agent 接入」、ERP 标签、通用建议审核、场景筛选；保留 E2E 依赖的可访问名称。
8. 脚本：scripts/prm-model-worker.mjs（替换 local-agent.mjs，Ollama/OpenAI 兼容）、scripts/prm-mcp.mjs（无依赖 MCP stdio：prm_import / prm_import_status）、docs/agent-openapi.json；Start-PRM.ps1 -ModelWorker。
9. 版本 0.4.0；文档 README/ARCHITECTURE/ONTOLOGY-API/SECURITY/LOCAL-AI/新增 AGENT-INTEGRATION、SELF-MODEL/VALIDATION；CLAUDE.md。

## 实施顺序（每步编译+测试）
A 后端：ontology 注册表、store(scenes/按 id 查询)、lib(Actor/路由)、erp、self_model、relationships、agents、analysis(候选/通用审批)、operations 精简 → cargo build/clippy/test
B 测试：tests/common 共享 harness；新增自我/关系/Agent/ERP/本体一致性测试
C 前端拆分与新页面 → npm run build
D 脚本（执行器、MCP、OpenAPI、Start-PRM）
E E2E 扩展 → npm run test:e2e
F 文档与 CLAUDE.md

## 验证
Start-Postgres.ps1 后 cargo test --all-targets、clippy -D warnings、fmt；npm run build；npm run test:e2e；用 node 实测 MCP 桥与执行器（无真实模型时用本地 mock OpenAI 兼容服务）。
