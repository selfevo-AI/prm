# 外部 AI Agent 接入

外部 Agent 可以把整理好的信息交给 PRM：资源（物品、设备、耗材）、关系（谁供应什么、某事关联某物）、消息、知识、人物和任务。PRM 把它们作为“导入信封”原样保存，经路由交给离线规则或本地模型处理，结果只作为待本人审核的建议。连接令牌**只能导入**，不能读取 PRM 中的任何数据。

## 1. 签发连接令牌

在“数字空间 · Agent 接入 · 外部 Agent 连接”填写名称（例如 Claude Desktop、Dify）并选择接入方式，点击“签发令牌”。令牌以 `prm_` 开头，只显示一次；PRM 只保存它的哈希。不再使用时点击“撤销”，令牌立即失效。

## 2. HTTP 接口（Dify、n8n、LangChain、自建 Agent）

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

查询状态：`GET /api/agent/imports`（最近 50 个）或 `GET /api/agent/imports/{id}`，返回处理状态与待审/已采纳/已拒绝的建议数量，不返回任何内容。

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

签发令牌时把接入方式选为“MCP 工具桥”，便于在连接列表中区分。

## 4. 处理与审核

导入后：

- 未匹配路由规则时，离线规则立即生成候选（状态“已生成候选”）。
- 匹配到本地模型规则时，导入进入队列，由本地模型执行器在“自我本体核心原则”下处理（见 [LOCAL-AI.md](LOCAL-AI.md)）。

候选出现在“决策中心”，来源筛选选“外部 Agent 导入”。每条候选显示动作（建议任务 / 纳入实体 / 建立关系）、属性、理由、引用的核心原则和来源导入。引用同一导入中其他候选的关系，需要先采纳被引用的那一条。拒绝不会删除导入，导入记录始终保留在“Agent 接入 · 导入记录”中。
