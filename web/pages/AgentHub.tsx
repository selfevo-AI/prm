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
import { api, label, num, prop, sceneLabel, SCENES, time, type Entity } from "../api";
import { Empty, Scenes, SelectField, StatusBadge, SceneInput, TextField } from "../components";
import type { PageContext } from "../context";
import { t } from "../i18n";

interface Connection {
  id: string;
  name: string;
  protocol: string;
  platform: string;
  platform_label: string;
  scenes: string[];
  agent_id: string;
  relationship_id: string;
  relationship_status: string;
  created_at: string;
  active: boolean;
  revoked_at: string | null;
  last_used_at: string | null;
}
interface PlatformPreset {
  id: string;
  label: string;
  protocol: string;
  docs: string;
  hint: string;
}
interface RelationType {
  id: string;
  label: string;
  scene: string;
}
interface Connections {
  connections: Connection[];
  protocols: { id: string; label: string }[];
  platforms: PlatformPreset[];
  agent_relationship_types: RelationType[];
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
  const { byKind, run, busy, fail, go, notify } = ctx;
  const [data, setData] = useState<Connections | null>(null);
  const [issued, setIssued] = useState<{
    name: string;
    token: string;
    protocol: string;
    platform: string;
    snippet: string;
  } | null>(null);
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
      entity.status === "active" ? t("已停用") : t("已启用"),
    );
  const platform = (id: string) =>
    data?.platforms.find((p) => p.id === id);
  const preset = issued ? platform(issued.platform) : undefined;
  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      notify(t("已复制接入片段"));
    } catch {
      notify(t("复制失败，请手动选择文本"));
    }
  };
  return (
    <div className="agent-hub">
      <section className="pipeline" aria-label={t("Agent 处理流程")}>
        {[
          ["外部 Agent", "HTTP / MCP 导入"],
          ["导入信封", "原样保存，只追加"],
          ["路由", "按 Agent、场景、类型匹配"],
          ["离线规则 / 本地模型", "以自我本体为核心原则"],
          ["候选建议", "资源 · 关系 · 任务"],
          ["本人审核", "采纳后纳入本体"],
        ].map(([title, detail], index) => (
          <div key={title} className="pipeline-step">
            <strong>{t(title)}</strong>
            <span>{t(detail)}</span>
            {index < 5 && <ArrowRight size={14} className="pipeline-arrow" />}
          </div>
        ))}
      </section>
      <div className="hub-grid">
        <section className="panel hub-panel">
          <div className="section-heading">
            <h2>
              <KeyRound size={16} />
              {t("外部 Agent 连接")}
            </h2>
            <span className="section-meta">{t("只能导入待审信息")}</span>
          </div>
          <ConnectionForm
            platforms={data?.platforms || []}
            protocols={data?.protocols || []}
            types={data?.agent_relationship_types || []}
            busy={busy}
            onIssued={async (body) => {
              try {
                const created = await api<{
                  token: string;
                  name: string;
                  protocol: string;
                  platform: string;
                  snippet: string;
                }>("/agents/connections", body);
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
              <strong>
                {t("“{name}”的令牌只显示这一次", { name: issued.name })}
              </strong>
              <code>{issued.token}</code>
              <p className="muted small-text">
                {preset ? t(preset.hint) : t("把它配置到外部 Agent 的导入设置中。")}
                {preset?.docs ? (
                  <>
                    {" "}
                    <a href={preset.docs} target="_blank" rel="noreferrer">
                      {t("平台文档")}
                    </a>
                  </>
                ) : null}
              </p>
              <pre className="snippet-text">{issued.snippet}</pre>
              <div className="token-actions">
                <button
                  className="text-button"
                  onClick={() => void copy(issued.snippet)}
                >
                  {t("复制接入片段")}
                </button>
                <button className="text-button" onClick={() => setIssued(null)}>
                  {t("我已保存，隐藏令牌")}
                </button>
              </div>
            </div>
          )}
          {data?.connections.length ? (
            <ul className="hub-list">
              {data.connections.map((c) => (
                <li key={c.id}>
                  <div>
                    <strong>{c.name}</strong>
                    <span className="muted small-text">
                      {(c.platform_label && t(c.platform_label)) ||
                        label(c.platform) ||
                        t("自定义 / 通用")}
                      {" · "}
                      {label(c.protocol)}
                      {" · "}
                      {c.last_used_at
                        ? t("最近使用 {at}", { at: time(c.last_used_at) })
                        : t("尚未使用")}
                    </span>
                    <span className="connection-scenes">
                      <Scenes scenes={c.scenes} />
                      {c.relationship_id && (
                        <button
                          className="text-button"
                          onClick={() => go("relations")}
                        >
                          {t("在关系网络中查看")}
                        </button>
                      )}
                    </span>
                  </div>
                  {c.active ? (
                    <button
                      className="button secondary"
                      disabled={busy}
                      aria-label={t("撤销{name}的令牌", { name: c.name })}
                      onClick={() =>
                        void run(
                          () => api("/agents/connections/revoke", { id: c.id }),
                          t("令牌已撤销"),
                        ).then(() => load())
                      }
                    >
                      {t("撤销")}
                    </button>
                  ) : (
                    <StatusBadge status="revoked" />
                  )}
                </li>
              ))}
            </ul>
          ) : (
            <p className="muted small-text">{t("还没有外部 Agent 连接。")}</p>
          )}
        </section>
        <section className="panel hub-panel">
          <div className="section-heading">
            <h2>
              <Cpu size={16} />
              {t("本地模型")}
            </h2>
            <span className="section-meta">{t("只允许本机回环地址")}</span>
          </div>
          <ModelForm
            busy={busy}
            onCreate={(body) =>
              run(() => api("/agents/models", body), t("本地模型已登记"))
            }
          />
          {models.length ? (
            <ul className="hub-list">
              {models.map((m) => (
                <li key={m.id}>
                  <div>
                    <strong>{m.title}</strong>
                    <span className="muted small-text">
                      {label(prop(m, "provider"))} · {prop(m, "model")} · {prop(m, "base_url")}
                    </span>
                  </div>
                  <button
                    className="button secondary"
                    disabled={busy}
                    aria-label={t("{action}：{name}", {
                      action: t(m.status === "active" ? "停用" : "启用"),
                      name: m.title,
                    })}
                    onClick={() => toggle(m)}
                  >
                    {t(m.status === "active" ? "停用" : "启用")}
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="muted small-text">
              {t("尚未登记本地模型，所有导入由离线规则处理。")}
            </p>
          )}
          <div className={`executor-state ${data?.executor_configured ? "ready" : ""}`}>
            <ShieldCheck size={15} />
            {data?.executor_configured
              ? t("执行器令牌已配置：运行 node scripts/prm-model-worker.mjs 领取排队任务")
              : t("未配置执行器：设置 PRM_AGENT_TOKEN 后重启服务（或使用 Start-PRM.ps1 -ModelWorker）")}
          </div>
        </section>
      </div>
      <section className="panel hub-panel">
        <div className="section-heading">
          <h2>
            <Route size={16} />
            {t("路由规则")}
          </h2>
          <span className="section-meta">
            {t("优先级高者先匹配；都不匹配时使用离线规则")}
          </span>
        </div>
        <RouteForm
          busy={busy}
          connections={data?.connections.filter((c) => c.active) || []}
          models={models.filter((m) => m.status === "active")}
          onCreate={(body) => run(() => api("/agents/routes", body), t("路由规则已添加"))}
        />
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t("规则")}</th>
                <th>{t("优先级")}</th>
                <th>{t("匹配条件")}</th>
                <th>{t("执行方式")}</th>
                <th>{t("状态")}</th>
              </tr>
            </thead>
            <tbody>
              {rules.map((r) => (
                <tr key={r.id}>
                  <td>{r.title}</td>
                  <td>{num(r, "priority")}</td>
                  <td>
                    {[
                      prop(r, "agent_name") &&
                        t("Agent：{value}", { value: prop(r, "agent_name") }),
                      prop(r, "scene") &&
                        t("场景：{value}", { value: sceneLabel(prop(r, "scene")) }),
                      prop(r, "item_kind") &&
                        t("类型：{value}", { value: t(itemKinds[prop(r, "item_kind")]) }),
                    ]
                      .filter(Boolean)
                      .join(t("；")) || t("全部导入")}
                  </td>
                  <td>{prop(r, "executor_title")}</td>
                  <td>
                    <button
                      className="text-button"
                      disabled={busy}
                      onClick={() => toggle(r)}
                      aria-label={t("{action}：{name}", {
                        action: t(r.status === "active" ? "停用" : "启用"),
                        name: r.title,
                      })}
                    >
                      <StatusBadge status={r.status} />
                    </button>
                  </td>
                </tr>
              ))}
              <tr>
                <td>{t("默认")}</td>
                <td>—</td>
                <td>{t("未匹配任何规则")}</td>
                <td>{t("离线规则")}</td>
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
            {t("导入记录")}
            <span className="count">{imports.length}</span>
          </h2>
          <button className="text-button" onClick={() => go("decisions")}>
            {t("到决策中心审核")}
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
                        {time(String(i.properties.received_at || i.created_at))} ·{" "}
                        {t("{count} 项", { count: items.length })} ·{" "}
                        {route.executor === "model"
                          ? t("本地模型 {name}", { name: String(route.model) })
                          : t("离线规则")}
                        {mine.length > 0 &&
                          ` · ${t("{pending}/{total} 条待审", {
                            pending: mine.filter((p) => p.status === "pending").length,
                            total: mine.length,
                          })}`}
                      </small>
                    </span>
                    <Scenes scenes={i.scenes} />
                    <StatusBadge status={i.status} />
                  </summary>
                  {route.note && <p className="muted small-text">{t(route.note)}</p>}
                  <ul>
                    {items.map((item, index) => (
                      <li key={index}>
                        <span className="tag">{t(itemKinds[String(item.kind)] || String(item.kind))}</span>
                        {String(item.title || "") ||
                          `${String(item.subject)} —${label(String(item.predicate))}→ ${String(item.object)}`}
                        {item.body ? <span className="muted"> · {String(item.body).slice(0, 120)}</span> : null}
                      </li>
                    ))}
                  </ul>
                </details>
              );
            })}
          </div>
        ) : (
          <Empty title={t("还没有外部 Agent 导入")} />
        )}
      </section>
      {runs.length > 0 && (
        <section className="panel hub-panel">
          <div className="section-heading">
            <h2>
              <Cpu size={16} />
              {t("路由运行")}
            </h2>
            <span className="section-meta">
              {t("最近 {count} 次", { count: Math.min(runs.length, 20) })}
            </span>
          </div>
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>{t("时间")}</th>
                  <th>{t("执行方式")}</th>
                  <th>{t("尝试")}</th>
                  <th>{t("结果")}</th>
                  <th>{t("状态")}</th>
                </tr>
              </thead>
              <tbody>
                {runs.slice(0, 20).map((r) => {
                  const rejected = Array.isArray(r.properties.rejected)
                    ? r.properties.rejected.length
                    : 0;
                  return (
                    <tr key={r.id}>
                      <td className="muted nowrap">{time(r.created_at)}</td>
                      <td>
                        {prop(r, "executor") === "model"
                          ? t("本地模型 {name}", { name: prop(r, "model") })
                          : t("离线规则")}
                      </td>
                      <td>{num(r, "attempts") || (r.status === "completed" ? 1 : 0)}</td>
                      <td>
                        {prop(r, "error") ||
                          t("{count} 条候选", { count: num(r, "proposals_created") }) +
                            (rejected
                              ? t("，{count} 条未通过校验", { count: rejected })
                              : "")}
                      </td>
                      <td>
                        <StatusBadge status={r.status} />
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </section>
      )}
    </div>
  );
}

function ConnectionForm({
  platforms,
  protocols,
  types,
  busy,
  onIssued,
}: {
  platforms: PlatformPreset[];
  protocols: { id: string; label: string }[];
  types: RelationType[];
  busy: boolean;
  onIssued: (body: Record<string, unknown>) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [platform, setPlatform] = useState("custom");
  const [protocol, setProtocol] = useState("http");
  const [type, setType] = useState("ai_assistant");
  const [scenes, setScenes] = useState<string[]>([]);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    await onIssued({
      name,
      protocol,
      platform,
      relationship_type: type,
      scenes,
    });
    setName("");
  };
  return (
    <form className="inline-form connection-form" onSubmit={submit}>
      <SelectField
        label="接入平台"
        value={platform}
        onChange={(value) => {
          setPlatform(value);
          const preset = platforms.find((p) => p.id === value);
          if (preset) setProtocol(preset.protocol);
        }}
        options={platforms.map((p) => ({ value: p.id, label: p.label }))}
      />
      <TextField
        label="连接名称"
        value={name}
        onChange={setName}
        maxLength={80}
        placeholder="例如：Claude Desktop"
      />
      <SelectField
        label="接入方式"
        value={protocol}
        onChange={setProtocol}
        options={protocols.map((p) => ({ value: p.id, label: p.label }))}
      />
      <SelectField
        label="关系类型"
        value={type}
        onChange={(value) => {
          setType(value);
          const picked = types.find((x) => x.id === value);
          if (picked && scenes.length === 0) setScenes([picked.scene]);
        }}
        options={types.map((x) => ({ value: x.id, label: x.label }))}
      />
      <SceneInput
        scenes={scenes}
        onChange={setScenes}
        legend="关系场景（不选则按关系类型默认）"
      />
      <button className="button primary" disabled={busy || !name.trim()}>
        {t("签发令牌")}
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
        {t("登记模型")}
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
        options={Object.entries(itemKinds).map(([value, itemLabel]) => ({ value, label: itemLabel }))}
      />
      <SelectField
        label="执行方式"
        value={executor}
        onChange={setExecutor}
        options={[
          { value: "offline-rules", label: "离线规则" },
          ...models.map((m) => ({ value: m.id, label: `${t("本地模型")} · ${m.title}` })),
        ]}
      />
      <button className="button primary" disabled={busy || !title.trim()}>
        {t("添加规则")}
      </button>
    </form>
  );
}
