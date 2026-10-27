import { useCallback, useEffect, useState, type FormEvent } from "react";
import {
  ArrowRight,
  Bot,
  Cpu,
  KeyRound,
  Route,
  ShieldCheck,
  Waypoints,
} from "lucide-react";
import { api, labels, num, prop, sceneLabel, SCENES, time, type Entity } from "../api";
import { Empty, Scenes, SelectField, StatusBadge, TextField } from "../components";
import type { PageContext } from "../context";

interface Connection {
  id: string;
  name: string;
  protocol: string;
  created_at: string;
  active: boolean;
  revoked_at: string | null;
  last_used_at: string | null;
}
interface Connections {
  connections: Connection[];
  protocols: { id: string; label: string }[];
  executor_configured: boolean;
}
const itemKinds: Record<string, string> = {
  resource: "资源",
  relation: "关系",
  message: "消息",
  note: "知识",
  person: "人物",
  task: "任务",
};

export function AgentHub({ ctx }: { ctx: PageContext }) {
  const { byKind, run, busy, fail, go } = ctx;
  const [data, setData] = useState<Connections | null>(null);
  const [issued, setIssued] = useState<{ name: string; token: string; protocol: string } | null>(null);
  const load = useCallback(async () => {
    setData(await api<Connections>("/agents/connections"));
  }, []);
  useEffect(() => {
    void load().catch(fail);
  }, [load, fail]);
  const imports = byKind("agent_import");
  const runs = byKind("agent_run");
  const models = byKind("model_endpoint");
  const rules = byKind("route_rule").sort(
    (a, b) => num(b, "priority") - num(a, "priority"),
  );
  const proposals = byKind("proposal");
  const toggle = (entity: Entity) =>
    void run(
      () =>
        api(`/agents/config/${entity.id}/status`, {
          status: entity.status === "active" ? "disabled" : "active",
          expected_version: entity.version,
        }),
      entity.status === "active" ? "已停用" : "已启用",
    );
  return (
    <div className="agent-hub">
      <section className="pipeline" aria-label="Agent 处理流程">
        {[
          ["外部 Agent", "HTTP / MCP 导入"],
          ["导入信封", "原样保存，只追加"],
          ["路由", "按 Agent、场景、类型匹配"],
          ["离线规则 / 本地模型", "以自我本体为核心原则"],
          ["候选建议", "资源 · 关系 · 任务"],
          ["本人审核", "采纳后纳入本体"],
        ].map(([title, detail], index) => (
          <div key={title} className="pipeline-step">
            <strong>{title}</strong>
            <span>{detail}</span>
            {index < 5 && <ArrowRight size={14} className="pipeline-arrow" />}
          </div>
        ))}
      </section>
      <div className="hub-grid">
        <section className="panel hub-panel">
          <div className="section-heading">
            <h2>
              <KeyRound size={16} />
              外部 Agent 连接
            </h2>
            <span className="section-meta">只能导入待审信息</span>
          </div>
          <ConnectionForm
            protocols={data?.protocols || []}
            busy={busy}
            onIssued={async (body) => {
              try {
                const created = await api<{ token: string; name: string; protocol: string }>(
                  "/agents/connections",
                  body,
                );
                setIssued(created);
                await load();
                await ctx.refresh();
              } catch (reason) {
                fail(reason);
              }
            }}
          />
          {issued && (
            <div className="token-box" role="note">
              <strong>“{issued.name}”的令牌只显示这一次</strong>
              <code>{issued.token}</code>
              <p className="muted small-text">
                {issued.protocol === "mcp"
                  ? "在 Claude Desktop、Cursor 等客户端的 MCP 配置中运行 node scripts/prm-mcp.mjs，并把它设为环境变量 PRM_CONNECTION_TOKEN。"
                  : "HTTP 调用：POST /api/agent/import，请求头 Authorization: Bearer <令牌> 与 X-PRM-Request: 1；接口说明见 docs/agent-openapi.json。"}
              </p>
              <button className="text-button" onClick={() => setIssued(null)}>
                我已保存，隐藏令牌
              </button>
            </div>
          )}
          {data?.connections.length ? (
            <ul className="hub-list">
              {data.connections.map((c) => (
                <li key={c.id}>
                  <div>
                    <strong>{c.name}</strong>
                    <span className="muted small-text">
                      {data.protocols.find((p) => p.id === c.protocol)?.label || c.protocol}
                      {" · "}
                      {c.last_used_at ? `最近使用 ${time(c.last_used_at)}` : "尚未使用"}
                    </span>
                  </div>
                  {c.active ? (
                    <button
                      className="button secondary"
                      disabled={busy}
                      aria-label={`撤销${c.name}的令牌`}
                      onClick={() =>
                        void run(() => api("/agents/connections/revoke", { id: c.id }), "令牌已撤销").then(
                          () => load(),
                        )
                      }
                    >
                      撤销
                    </button>
                  ) : (
                    <StatusBadge status="revoked" />
                  )}
                </li>
              ))}
            </ul>
          ) : (
            <p className="muted small-text">还没有外部 Agent 连接。</p>
          )}
        </section>
        <section className="panel hub-panel">
          <div className="section-heading">
            <h2>
              <Cpu size={16} />
              本地模型
            </h2>
            <span className="section-meta">只允许本机回环地址</span>
          </div>
          <ModelForm
            busy={busy}
            onCreate={(body) => run(() => api("/agents/models", body), "本地模型已登记")}
          />
          {models.length ? (
            <ul className="hub-list">
              {models.map((m) => (
                <li key={m.id}>
                  <div>
                    <strong>{m.title}</strong>
                    <span className="muted small-text">
                      {labels[prop(m, "provider")]} · {prop(m, "model")} · {prop(m, "base_url")}
                    </span>
                  </div>
                  <button
                    className="button secondary"
                    disabled={busy}
                    aria-label={`${m.status === "active" ? "停用" : "启用"}${m.title}`}
                    onClick={() => toggle(m)}
                  >
                    {m.status === "active" ? "停用" : "启用"}
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="muted small-text">尚未登记本地模型，所有导入由离线规则处理。</p>
          )}
          <div className={`executor-state ${data?.executor_configured ? "ready" : ""}`}>
            <ShieldCheck size={15} />
            {data?.executor_configured
              ? "执行器令牌已配置：运行 node scripts/prm-model-worker.mjs 领取排队任务"
              : "未配置执行器：设置 PRM_AGENT_TOKEN 后重启服务（或使用 Start-PRM.ps1 -ModelWorker）"}
          </div>
        </section>
      </div>
      <section className="panel hub-panel">
        <div className="section-heading">
          <h2>
            <Route size={16} />
            路由规则
          </h2>
          <span className="section-meta">优先级高者先匹配；都不匹配时使用离线规则</span>
        </div>
        <RouteForm
          busy={busy}
          connections={data?.connections.filter((c) => c.active) || []}
          models={models.filter((m) => m.status === "active")}
          onCreate={(body) => run(() => api("/agents/routes", body), "路由规则已添加")}
        />
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>规则</th>
                <th>优先级</th>
                <th>匹配条件</th>
                <th>执行方式</th>
                <th>状态</th>
              </tr>
            </thead>
            <tbody>
              {rules.map((r) => (
                <tr key={r.id}>
                  <td>{r.title}</td>
                  <td>{num(r, "priority")}</td>
                  <td>
                    {[
                      prop(r, "agent_name") && `Agent：${prop(r, "agent_name")}`,
                      prop(r, "scene") && `场景：${sceneLabel(prop(r, "scene"))}`,
                      prop(r, "item_kind") && `类型：${itemKinds[prop(r, "item_kind")]}`,
                    ]
                      .filter(Boolean)
                      .join("；") || "全部导入"}
                  </td>
                  <td>{prop(r, "executor_title")}</td>
                  <td>
                    <button
                      className="text-button"
                      disabled={busy}
                      onClick={() => toggle(r)}
                      aria-label={`${r.status === "active" ? "停用" : "启用"}规则${r.title}`}
                    >
                      <StatusBadge status={r.status} />
                    </button>
                  </td>
                </tr>
              ))}
              <tr>
                <td>默认</td>
                <td>—</td>
                <td>未匹配任何规则</td>
                <td>离线规则</td>
                <td>
                  <StatusBadge status="active" />
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
      <section className="panel hub-panel">
        <div className="section-heading">
          <h2>
            <Waypoints size={16} />
            导入记录
            <span className="count">{imports.length}</span>
          </h2>
          <button className="text-button" onClick={() => go("decisions")}>
            到决策中心审核
            <ArrowRight size={14} />
          </button>
        </div>
        {imports.length ? (
          <div className="import-list">
            {imports.map((i) => {
              const items = (i.properties.items as Record<string, unknown>[] | undefined) || [];
              const route = (i.properties.route || {}) as Record<string, string>;
              const mine = proposals.filter((p) => prop(p, "source_id") === i.id);
              return (
                <details key={i.id} className="import-item">
                  <summary>
                    <Bot size={16} />
                    <span>
                      <strong>{i.title}</strong>
                      <small className="muted">
                        {time(String(i.properties.received_at || i.created_at))} · {items.length} 项 ·{" "}
                        {route.executor === "model" ? `本地模型 ${String(route.model)}` : "离线规则"}
                        {mine.length > 0 && ` · ${mine.filter((p) => p.status === "pending").length}/${mine.length} 条待审`}
                      </small>
                    </span>
                    <Scenes scenes={i.scenes} />
                    <StatusBadge status={i.status} />
                  </summary>
                  {route.note && <p className="muted small-text">{route.note}</p>}
                  <ul>
                    {items.map((item, index) => (
                      <li key={index}>
                        <span className="tag">{itemKinds[String(item.kind)] || String(item.kind)}</span>
                        {String(item.title || "") ||
                          `${String(item.subject)} —${labels[String(item.predicate)] || String(item.predicate)}→ ${String(item.object)}`}
                        {item.body ? <span className="muted"> · {String(item.body).slice(0, 120)}</span> : null}
                      </li>
                    ))}
                  </ul>
                </details>
              );
            })}
          </div>
        ) : (
          <Empty title="还没有外部 Agent 导入" />
        )}
      </section>
      {runs.length > 0 && (
        <section className="panel hub-panel">
          <div className="section-heading">
            <h2>
              <Cpu size={16} />
              路由运行
            </h2>
            <span className="section-meta">最近 {Math.min(runs.length, 20)} 次</span>
          </div>
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>时间</th>
                  <th>执行方式</th>
                  <th>尝试</th>
                  <th>结果</th>
                  <th>状态</th>
                </tr>
              </thead>
              <tbody>
                {runs.slice(0, 20).map((r) => (
                  <tr key={r.id}>
                    <td className="muted nowrap">{time(r.created_at)}</td>
                    <td>{prop(r, "executor") === "model" ? `本地模型 ${prop(r, "model")}` : "离线规则"}</td>
                    <td>{num(r, "attempts") || (r.status === "completed" ? 1 : 0)}</td>
                    <td>
                      {prop(r, "error") ||
                        `${num(r, "proposals_created")} 条候选${
                          Array.isArray(r.properties.rejected) && r.properties.rejected.length
                            ? `，${r.properties.rejected.length} 条未通过校验`
                            : ""
                        }`}
                    </td>
                    <td>
                      <StatusBadge status={r.status} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      )}
    </div>
  );
}

function ConnectionForm({
  protocols,
  busy,
  onIssued,
}: {
  protocols: { id: string; label: string }[];
  busy: boolean;
  onIssued: (body: { name: string; protocol: string }) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [protocol, setProtocol] = useState("mcp");
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    await onIssued({ name, protocol });
    setName("");
  };
  return (
    <form className="inline-form" onSubmit={submit}>
      <TextField label="连接名称" value={name} onChange={setName} maxLength={80} placeholder="例如：Claude Desktop" />
      <SelectField
        label="接入方式"
        value={protocol}
        onChange={setProtocol}
        options={protocols.map((p) => ({ value: p.id, label: p.label }))}
      />
      <button className="button primary" disabled={busy || !name.trim()}>
        签发令牌
      </button>
    </form>
  );
}

function ModelForm({
  busy,
  onCreate,
}: {
  busy: boolean;
  onCreate: (body: Record<string, string>) => Promise<boolean>;
}) {
  const [title, setTitle] = useState("");
  const [provider, setProvider] = useState("ollama");
  const [baseUrl, setBaseUrl] = useState("http://127.0.0.1:11434");
  const [model, setModel] = useState("");
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (await onCreate({ title, provider, base_url: baseUrl, model })) {
      setTitle("");
      setModel("");
    }
  };
  return (
    <form className="inline-form" onSubmit={submit}>
      <TextField label="模型名称" value={title} onChange={setTitle} maxLength={80} placeholder="例如：本地 Qwen" />
      <SelectField
        label="服务类型"
        value={provider}
        onChange={(value) => {
          setProvider(value);
          setBaseUrl(value === "ollama" ? "http://127.0.0.1:11434" : "http://127.0.0.1:1234/v1");
        }}
        options={[
          { value: "ollama", label: "Ollama" },
          { value: "openai_compatible", label: "OpenAI 兼容" },
        ]}
      />
      <TextField label="本机地址" value={baseUrl} onChange={setBaseUrl} maxLength={200} />
      <TextField label="模型标识" value={model} onChange={setModel} maxLength={120} placeholder="例如：qwen2.5:7b" />
      <button className="button primary" disabled={busy}>
        登记模型
      </button>
    </form>
  );
}

function RouteForm({
  busy,
  connections,
  models,
  onCreate,
}: {
  busy: boolean;
  connections: Connection[];
  models: Entity[];
  onCreate: (body: Record<string, unknown>) => Promise<boolean>;
}) {
  const [title, setTitle] = useState("");
  const [priority, setPriority] = useState("10");
  const [agent, setAgent] = useState("");
  const [scene, setScene] = useState("");
  const [kind, setKind] = useState("");
  const [executor, setExecutor] = useState("offline-rules");
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const ok = await onCreate({
      title,
      priority: Number(priority),
      agent_id: agent,
      scene,
      item_kind: kind,
      executor,
    });
    if (ok) setTitle("");
  };
  return (
    <form className="inline-form route-form" onSubmit={submit}>
      <TextField label="规则名称" value={title} onChange={setTitle} maxLength={80} placeholder="例如：工作导入走本地模型" />
      <TextField label="优先级" type="number" min={0} max={1000} step={1} value={priority} onChange={setPriority} />
      <SelectField
        label="来源 Agent"
        value={agent}
        onChange={setAgent}
        placeholder="任意"
        options={connections.map((c) => ({ value: c.id, label: c.name }))}
      />
      <SelectField
        label="场景"
        value={scene}
        onChange={setScene}
        placeholder="任意"
        options={SCENES.map((s) => ({ value: s, label: sceneLabel(s) }))}
      />
      <SelectField
        label="导入类型"
        value={kind}
        onChange={setKind}
        placeholder="任意"
        options={Object.entries(itemKinds).map(([value, label]) => ({ value, label }))}
      />
      <SelectField
        label="执行方式"
        value={executor}
        onChange={setExecutor}
        options={[
          { value: "offline-rules", label: "离线规则" },
          ...models.map((m) => ({ value: m.id, label: `本地模型 · ${m.title}` })),
        ]}
      />
      <button className="button primary" disabled={busy || !title.trim()}>
        添加规则
      </button>
    </form>
  );
}
