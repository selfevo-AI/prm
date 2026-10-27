// PRM local model executor. Claims routed imports from the PRM server, asks a
// model on this machine (Ollama or an OpenAI-compatible server) to turn them
// into candidates under the owner's self-ontology principles, and returns the
// candidates for owner review. It never logs imported content.
//
//   PRM_AGENT_TOKEN=<same token as the server> node scripts/prm-model-worker.mjs [--once] [--interval 5]
import { createHash } from "node:crypto";
import { hostname } from "node:os";

function loopback(value) {
  const url = new URL(value);
  if (
    url.protocol !== "http:" ||
    !["127.0.0.1", "localhost", "[::1]"].includes(url.hostname) ||
    url.username ||
    url.password ||
    url.search ||
    url.hash
  ) {
    throw new Error("Only plain HTTP loopback endpoints are allowed.");
  }
  return url;
}
const prm = loopback(process.env.PRM_URL || "http://127.0.0.1:4317");
const token = process.env.PRM_AGENT_TOKEN;
if (!token || token.length < 32)
  throw new Error("Set PRM_AGENT_TOKEN to the executor token the PRM server was started with.");
const once = process.argv.includes("--once");
const intervalIndex = process.argv.indexOf("--interval");
const interval = Math.max(1, Number(intervalIndex > 0 ? process.argv[intervalIndex + 1] : 5) || 5) * 1000;
const worker = (process.env.PRM_WORKER_NAME || `prm-model-worker@${hostname()}`).slice(0, 80);

async function request(path, body) {
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
  if (!response.ok) throw new Error(`PRM ${path} returned ${response.status}${data.error ? `: ${data.error}` : ""}`);
  return data;
}

function systemPrompt(job) {
  const p = job.principles;
  return [
    "你是 PRM（个人资源与关系管理）的本地分析执行器，运行在本人的电脑上。",
    "任务：把外部 Agent 导入的信息整理成结构化候选（资源、关系、任务），由本人审核后再纳入本体。",
    "固定规则：",
    ...p.rules.map((rule) => `- ${rule}`),
    "本人的核心原则（必须遵循，并在 rationale 中说明候选与原则的关系；引用时把 id 写进 principles）：",
    JSON.stringify(p.core_principles),
    "本人允许你参考的自我本体与关系（仅作背景，不做评判）：",
    JSON.stringify({ self: p.self, relationships: p.relationships }),
    "可以提出的本体类型与关系：",
    JSON.stringify(p.ontology),
    "输出：只返回一个 JSON 对象 {\"summary\": 字符串, \"candidates\": 数组}。候选字段：",
    JSON.stringify(job.output_contract.candidate),
    "用中文书写。没有值得纳入的内容时返回空的 candidates。",
  ].join("\n");
}

const strings = ["ref", "action", "kind", "title", "body", "subject", "predicate", "object", "rationale"];
const lists = ["scenes", "values", "principles"];
/** Keep only the contract's fields, with the contract's types. */
function sanitize(value) {
  const candidates = (Array.isArray(value?.candidates) ? value.candidates : [])
    .slice(0, 50)
    .filter((c) => c && typeof c === "object")
    .map((c) => {
      const clean = {};
      for (const key of strings) if (c[key] !== undefined && c[key] !== null) clean[key] = String(c[key]);
      for (const key of lists)
        if (Array.isArray(c[key])) clean[key] = c[key].filter((x) => typeof x === "string").slice(0, 10);
      if (c.properties && typeof c.properties === "object" && !Array.isArray(c.properties))
        clean.properties = c.properties;
      if (!clean.action) clean.action = "create_task";
      if (!clean.rationale) clean.rationale = "模型未给出理由，请本人核实。";
      return clean;
    });
  return { summary: String(value?.summary || "").slice(0, 2000), candidates };
}
function parse(content) {
  const text = String(content || "").trim();
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start < 0 || end < start) throw new Error("The local model did not return a JSON object.");
  return sanitize(JSON.parse(text.slice(start, end + 1)));
}
async function callModel(job) {
  const { provider, base_url: baseUrl, model } = job.model;
  loopback(baseUrl);
  const messages = [
    { role: "system", content: systemPrompt(job) },
    { role: "user", content: JSON.stringify({ untrusted_import: job.import }) },
  ];
  const ollama = provider === "ollama";
  const response = await fetch(`${baseUrl.replace(/\/+$/, "")}${ollama ? "/api/chat" : "/chat/completions"}`, {
    method: "POST",
    redirect: "error",
    signal: AbortSignal.timeout(Math.max(30, (job.lease_seconds || 300) - 30) * 1000),
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(
      ollama
        ? { model, messages, stream: false, format: "json", options: { temperature: 0.2 } }
        : { model, messages, temperature: 0.2, response_format: { type: "json_object" } },
    ),
  });
  if (!response.ok) throw new Error(`Local model returned ${response.status}.`);
  const result = await response.json();
  return parse(ollama ? result.message?.content : result.choices?.[0]?.message?.content);
}

async function runOnce() {
  const { job } = await request("/api/agent/jobs/claim", { worker });
  if (!job) return false;
  const tag = createHash("sha256").update(job.run_id).digest("hex").slice(0, 8);
  try {
    const result = await callModel(job);
    const saved = await request(`/api/agent/jobs/${job.run_id}/result`, { lease: job.lease, ...result });
    console.log(`${tag}: ${saved.proposals_created} candidate(s) awaiting owner review, ${saved.rejected.length} rejected by validation`);
  } catch (error) {
    console.error(`${tag}: ${error.message}`);
    await request(`/api/agent/jobs/${job.run_id}/fail`, {
      lease: job.lease,
      error: String(error.message || "unknown error").slice(0, 500),
    }).catch((reason) => console.error(`${tag}: could not report failure: ${reason.message}`));
  }
  return true;
}

for (;;) {
  try {
    while (await runOnce()) {
      // Drain the queue before sleeping.
    }
  } catch (error) {
    if (once) throw error;
    console.error(error.message);
  }
  if (once) break;
  await new Promise((resolve) => setTimeout(resolve, interval));
}
