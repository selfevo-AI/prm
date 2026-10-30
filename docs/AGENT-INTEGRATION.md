# 外部 AI Agent 接入

外部 Agent 可以把整理好的信息交给 PRM：资源（物品、设备、耗材）、关系（谁供应什么、某事关联某物）、消息、知识、人物和任务。PRM 把它们作为“导入信封”原样保存，经路由交给离线规则或本地模型处理，结果只作为待本人审核的建议。连接令牌**只能导入**，不能读取 PRM 中的任何数据。

主流 Agent 都可以接入：接入平台预设包含 Grok（xAI）、Claude（Anthropic）、ChatGPT（OpenAI）、Gemini（Google）、Microsoft Copilot、Muse（muse.ai）、DeepSeek、通义千问、Kimi、Dify、n8n、LangChain / LangGraph、Cursor、Cline，以及“自定义 / 通用”。任何能发起本机 HTTP 请求或使用 MCP 的 Agent 都能按下面的方式工作。

## 1. 接入即关系

在“数字空间 · Agent 接入 · 外部 Agent 连接”中选择接入平台、填写连接名称、接入方式、关系类型（AI 助理 / AI 伙伴 / AI 导师）与关系场景（生活、工作、学习等；不选则按关系类型默认）。点击“签发令牌”会同时完成三件事：

1. 该 Agent 登记为数字界实体（记录平台、接入方式与备注）；
2. 建立**本人与这个 AI Agent 的关系**——与 AI Agent 的关系是关系网络的一部分，可以在“关系网络”中查看、调整亲密度与满意度、记录互动，并逐条决定是否让本地模型读取；
3. 生成以 `prm_` 开头的一次性令牌（只显示一次，PRM 只保存它的哈希），并给出可复制的接入片段。

不再使用时点击“撤销”，令牌立即失效；关系本身保留在关系网络中，可继续维护。更早版本创建的连接没有绑定实体与关系，可重新接入一次以获得完整的关系记录。

## 2. HTTP 接口（Grok 机器人、Dify、n8n、LangChain、自建 Agent）

接口说明见 [agent-openapi.json](agent-openapi.json)（OpenAPI 3.1），可以导入到支持 OpenAPI 工具的平台。服务只监听本机回环地址，因此调用方必须运行在同一台电脑上。

```powershell
$token = "prm_..."
$body = @'
{
  "scenes": ["work"],
  "items": [
    {"ref": "r1", "kind": "resource", "title": "会议室投影仪", "attributes": {"unit": "台", "location": "会议室"}},
    {"ref": "p1", "kind": "person", "title": "王经理", "body": "项目对接人"},
    {"kind": "relation", "subject": "p1", "predicate": "references", "object": "r1"},
    {"kind": "task", "title": "周五前提交周报"}
  ]
}
'@
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:4317/api/agent/import `
  -Headers @{ Authorization = "Bearer $token"; "X-PRM-Request" = "1" } `
  -ContentType "application/json; charset=utf-8" -Body ([Text.Encoding]::UTF8.GetBytes($body))
```

也可以直接提交 OpenAI 风格的对话：

```json
{"messages": [{"role": "user", "content": "提醒我周三带体检报告"}, {"role": "assistant", "content": "好的"}]}
```

只有 `user` 与 `assistant` 消息会被导入，并转成待办候选。`items` 与 `messages` 二选一；单次最多 100 项，请求体不超过 128 KiB。

查询状态：`GET /api/agent/imports`（最近 50 个）或 `GET /api/agent/imports/{id}`，返回处理状态与待审/已采纳/已拒绝的建议数量，不返回任何内容。每个导入都会通过 `from_agent` 关系追溯到对应的 Agent 实体。

## 3. MCP 工具桥（Claude Desktop、Cursor、Cline 等）

`scripts/prm-mcp.mjs` 是不依赖第三方包的 MCP stdio 服务，提供两个工具：

- `prm_import`：与 HTTP 导入相同的参数（`items` 或 `messages`、`scenes`、`conversation`）。
- `prm_import_status`：查询导入状态。

在客户端的 MCP 配置中加入（路径按实际位置调整）：

```json
{
  "mcpServers": {
    "prm": {
      "command": "node",
      "args": ["D:\\prm\\scripts\\prm-mcp.mjs"],
      "env": {"PRM_CONNECTION_TOKEN": "prm_...", "PRM_URL": "http://127.0.0.1:4317"}
    }
  }
}
```

签发令牌时把接入方式选为“MCP 工具桥”，便于在连接列表中区分；签发后界面会给出填入令牌的完整配置片段，可直接复制。

## 4. 处理与审核

导入后：

- 未匹配路由规则时，离线规则立即生成候选（状态“已生成候选”）。
- 匹配到本地模型规则时，导入进入队列，由本地模型执行器在“自我本体核心原则”下处理（见 [LOCAL-AI.md](LOCAL-AI.md)）。

候选出现在“决策中心”，来源筛选选“外部 Agent 导入”。每条候选显示动作（建议任务 / 纳入实体 / 建立关系）、属性、理由、引用的核心原则和来源导入。引用同一导入中其他候选的关系，需要先采纳被引用的那一条。拒绝不会删除导入，导入记录始终保留在“Agent 接入 · 导入记录”中。
