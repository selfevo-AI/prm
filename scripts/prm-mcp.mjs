// MCP (stdio) bridge for mainstream agent clients such as Claude Desktop,
// Cursor or Cline. It exposes two tools: prm_import pushes resources,
// relations, messages, notes, people or tasks into PRM as a pending import,
// and prm_import_status reports what happened to it. The connection token can
// only import; it cannot read anything else from PRM.
//
// Client configuration example:
//   {"mcpServers": {"prm": {"command": "node", "args": ["D:\\prm\\scripts\\prm-mcp.mjs"],
//     "env": {"PRM_CONNECTION_TOKEN": "prm_…", "PRM_URL": "http://127.0.0.1:4317"}}}}
import { createInterface } from "node:readline";

function loopback(value) {
  const url = new URL(value);
  if (
    url.protocol !== "http:" ||
    !["127.0.0.1", "localhost", "[::1]"].includes(url.hostname) ||
    url.username ||
    url.password
  ) {
    throw new Error("PRM_URL must be a plain HTTP loopback address.");
  }
  return url;
}
const prm = loopback(process.env.PRM_URL || "http://127.0.0.1:4317");
const token = process.env.PRM_CONNECTION_TOKEN;
const SUPPORTED = ["2025-06-18", "2025-03-26", "2024-11-05"];
const SCENES = ["self", "emotion", "life", "work", "health", "learning", "finance", "social"];
const scenes = {
  type: "array",
  items: { type: "string", enum: SCENES },
  description: "人生场景：self 自我、emotion 情感、life 生活、work 工作、health 健康、learning 学习、finance 财务、social 社交 (life scenes: self, emotion, life, work, health, learning, finance, social)",
};
const tools = [
  {
    name: "prm_import",
    title: "导入到 PRM · Import into PRM",
    description:
      "把整理好的信息交给本人的 PRM（个人资源与关系管理）。PRM 会经路由交给本地模型或离线规则处理，结果只作为待审建议，由本人决定是否纳入。可以导入资源、关系、消息、知识、人物和任务；也可以直接提交对话 messages。不会返回本人的任何数据。 / Hand organized information to the owner's PRM (personal resource & relation management). PRM routes it to a local model or offline rules; results are proposals the owner reviews. Import resources, relations, messages, notes, people and tasks, or submit chat messages. Never returns any of the owner's data.",
    inputSchema: {
      type: "object",
      properties: {
        scenes,
        conversation: { type: "string", maxLength: 200, description: "外部会话或任务的引用（可选）· external conversation or task reference (optional)" },
        items: {
          type: "array",
          maxItems: 100,
          items: {
            type: "object",
            properties: {
              ref: { type: "string", maxLength: 40, description: "本批内的编号，关系项可通过它引用其他项 · batch-local reference used by relation items" },
              kind: { type: "string", enum: ["resource", "relation", "message", "note", "person", "task"] },
              title: { type: "string", maxLength: 160 },
              body: { type: "string", maxLength: 8000 },
              scenes,
              attributes: {
                type: "object",
                additionalProperties: { type: "string", maxLength: 400 },
                description: "补充属性，例如资源的 unit、location、sku、category、minimum · extra attributes",
              },
              subject: { type: "string", description: "关系项：起点的 ref · relation item: subject ref" },
              predicate: { type: "string", enum: ["references", "uses", "supplies", "stored_at", "implements"] },
              object: { type: "string", description: "关系项：终点的 ref · relation item: object ref" },
            },
            required: ["kind"],
            additionalProperties: false,
          },
        },
        messages: {
          type: "array",
          maxItems: 100,
          items: {
            type: "object",
            properties: {
              role: { type: "string", enum: ["user", "assistant"] },
              content: { type: "string", maxLength: 8000 },
            },
            required: ["role", "content"],
            additionalProperties: false,
          },
        },
      },
      additionalProperties: false,
    },
  },
  {
    name: "prm_import_status",
    title: "查询 PRM 导入状态 · Check PRM import status",
    description:
      "查询本连接提交的导入是否已处理、有多少建议待本人审核。不提供 import_id 时列出最近的导入。 / Report whether an import from this connection is processed and how many proposals await the owner. Lists recent imports when import_id is omitted.",
    inputSchema: {
      type: "object",
      properties: { import_id: { type: "string" } },
      additionalProperties: false,
    },
  },
];

async function request(path, body) {
    if (!token) throw new Error("未设置 PRM_CONNECTION_TOKEN：请在 PRM「数字空间 · Agent 接入」中签发令牌。 / PRM_CONNECTION_TOKEN is not set; issue a token in PRM under Digital space · Agent connections.");
  const response = await fetch(new URL(path, prm), {
    method: body ? "POST" : "GET",
    redirect: "error",
    signal: AbortSignal.timeout(15_000),
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
      "X-PRM-Request": "1",
    },
    ...(body ? { body: JSON.stringify(body) } : {}),
  });
  const data = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(data.error || `PRM 返回 ${response.status}`);
  return data;
}
async function callTool(params) {
  try {
    const args = params?.arguments ?? {};
    if (params?.name === "prm_import") {
      const result = await request("/api/agent/import", args);
      const text =
        result.status === "queued"
          ? `已导入（${result.import_id}），正在等待本地模型处理；结果将由本人审核。 / Imported (${result.import_id}); queued for the local model, results await the owner's review.`
          : `已导入（${result.import_id}），生成 ${result.proposals_created} 条待本人审核的建议。 / Imported (${result.import_id}); ${result.proposals_created} proposals await the owner's review.`;
      return { content: [{ type: "text", text }], structuredContent: result };
    }
    if (params?.name === "prm_import_status") {
      const path = args.import_id
        ? `/api/agent/imports/${encodeURIComponent(String(args.import_id))}`
        : "/api/agent/imports";
      const result = await request(path);
      return { content: [{ type: "text", text: JSON.stringify(result) }], structuredContent: result };
    }
    return { content: [{ type: "text", text: `未知工具：${params?.name}` }], isError: true };
  } catch (error) {
    return { content: [{ type: "text", text: String(error.message || error) }], isError: true };
  }
}
function send(message) {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}
async function handle(message) {
  const respond = (result) => message.id !== undefined && send({ jsonrpc: "2.0", id: message.id, result });
  const error = (code, text) =>
    message.id !== undefined && send({ jsonrpc: "2.0", id: message.id, error: { code, message: text } });
  switch (message.method) {
    case "initialize": {
      const requested = message.params?.protocolVersion;
      respond({
        protocolVersion: SUPPORTED.includes(requested) ? requested : SUPPORTED[0],
        capabilities: { tools: { listChanged: false } },
        serverInfo: { name: "prm", title: "PRM 个人资源与关系 · Personal resources & relations", version: "0.5.0" },
        instructions: "用 prm_import 把资源、关系与待办交给本人的 PRM；PRM 只接收待审信息，不会返回本人的数据。 / Use prm_import to hand resources, relations and to-dos to the owner's PRM; it only accepts reviewable imports and never returns the owner's data.",
      });
      return;
    }
    case "notifications/initialized":
    case "notifications/cancelled":
      return;
    case "ping":
      respond({});
      return;
    case "tools/list":
      respond({ tools });
      return;
    case "tools/call":
      respond(await callTool(message.params));
      return;
    default:
      error(-32601, `Method not found: ${message.method}`);
  }
}
const lines = createInterface({ input: process.stdin });
lines.on("line", (line) => {
  if (!line.trim()) return;
  let message;
  try {
    message = JSON.parse(line);
  } catch {
    send({ jsonrpc: "2.0", id: null, error: { code: -32700, message: "Parse error" } });
    return;
  }
  for (const item of Array.isArray(message) ? message : [message])
    handle(item).catch((reason) => console.error(reason));
});
