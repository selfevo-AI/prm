import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import { ArrowDown, ArrowUp, BookMarked, Eye, Minus } from "lucide-react";
import {
  api,
  labels,
  prop,
  time,
  today,
  type Entity,
  type Instrument,
  type SelfCatalog,
} from "../api";
import {
  Dots,
  Empty,
  Scenes,
  SceneInput,
  SelectField,
  TextField,
  VisibilityToggle,
} from "../components";
import type { PageContext } from "../context";

interface Profile {
  profile: Entity;
  indicators: Entity[];
  identities: Entity[];
  self_relations: Entity[];
}
type Tab = "indicators" | "identity" | "relations" | "preview";

export function SelfPage({ ctx }: { ctx: PageContext }) {
  const [catalog, setCatalog] = useState<SelfCatalog | null>(null);
  const [data, setData] = useState<Profile | null>(null);
  const [tab, setTab] = useState<Tab>("indicators");
  const { fail } = ctx;
  const load = useCallback(async () => {
    const [nextCatalog, nextProfile] = await Promise.all([
      api<SelfCatalog>("/self/catalog"),
      api<Profile>("/self/profile"),
    ]);
    setCatalog(nextCatalog);
    setData(nextProfile);
  }, []);
  useEffect(() => {
    void load().catch(fail);
  }, [load, fail]);
  const save = async (work: () => Promise<unknown>, message: string) => {
    if (await ctx.run(work, message)) await load().catch(fail);
  };
  const toggle = (entity: Entity) =>
    void save(
      () =>
        api("/self/visibility", {
          id: entity.id,
          agent_visible: entity.properties.agent_visible !== true,
          expected_version: entity.version,
        }),
      "可见性已更新",
    );
  const observations = (data?.indicators.length || 0) + (data?.identities.length || 0) + (data?.self_relations.length || 0);
  return (
    <>
      <div className="intro-panel">
        <div>
          <span className="eyebrow">R = RESOURCE · RELATION</span>
          <h2>从与自我的关系开始管理资源</h2>
          <p>
            自我本体由本人记录：生理与心理测量是观察而不是诊断，身份认同与自我关系描述你如何看待和对待自己。
            每一条都可以单独决定是否让本地模型读取，系统不会把它们合成为单一分数。
          </p>
        </div>
        <div className="metric-summary">
          <strong>{observations}</strong>
          <span>条自我记录</span>
        </div>
      </div>
      <div className="tabs">
        {(
          [
            ["indicators", "指标观察", data?.indicators.length],
            ["identity", "自我认同", data?.identities.length],
            ["relations", "自我关系", data?.self_relations.length],
            ["preview", "Agent 可见预览", undefined],
          ] as const
        ).map(([value, label, count]) => (
          <button
            key={value}
            className={tab === value ? "active" : ""}
            onClick={() => setTab(value)}
          >
            {label}
            {count !== undefined && <span>{count}</span>}
          </button>
        ))}
      </div>
      {!catalog || !data ? (
        <Empty title="正在读取自我本体" />
      ) : tab === "indicators" ? (
        <Indicators
          catalog={catalog}
          indicators={ctx.filter(data.indicators)}
          busy={ctx.busy}
          onSave={save}
          onToggle={toggle}
        />
      ) : tab === "identity" ? (
        <Identities
          catalog={catalog}
          identities={ctx.filter(data.identities)}
          busy={ctx.busy}
          onSave={save}
          onToggle={toggle}
        />
      ) : tab === "relations" ? (
        <SelfRelations
          ctx={ctx}
          catalog={catalog}
          relations={ctx.filter(data.self_relations)}
          onSave={save}
          onToggle={toggle}
        />
      ) : (
        <AgentPreview fail={fail} />
      )}
    </>
  );
}

type Save = (work: () => Promise<unknown>, message: string) => Promise<void>;

function Indicators({
  catalog,
  indicators,
  busy,
  onSave,
  onToggle,
}: {
  catalog: SelfCatalog;
  indicators: Entity[];
  busy: boolean;
  onSave: Save;
  onToggle: (entity: Entity) => void;
}) {
  const [domain, setDomain] = useState<"body" | "psychology" | "custom">("psychology");
  const instruments = catalog.instruments.filter((i) => i.domain === domain);
  const [instrumentId, setInstrumentId] = useState("rses");
  const instrument: Instrument | undefined =
    instruments.find((i) => i.id === instrumentId) || instruments[0];
  const [scores, setScores] = useState<Record<string, string>>({});
  const [customLabel, setCustomLabel] = useState("");
  const [customUnit, setCustomUnit] = useState("");
  const [customDomain, setCustomDomain] = useState("body");
  const [customValue, setCustomValue] = useState("");
  const [observedAt, setObservedAt] = useState(today());
  const [notes, setNotes] = useState("");
  const [scenes, setScenes] = useState<string[]>([]);
  const [agentVisible, setAgentVisible] = useState(false);
  const [error, setError] = useState("");
  const families = [...new Set(instruments.map((i) => i.family))];
  const switchDomain = (next: typeof domain) => {
    setDomain(next);
    setScores({});
    setError("");
    const first = catalog.instruments.find((i) => i.domain === next);
    if (first) setInstrumentId(first.id);
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError("");
    const filled =
      domain === "custom"
        ? [{ metric: "custom", value: Number(customValue) }]
        : (instrument?.metrics || [])
            .filter((m) => (scores[m.id] ?? "").trim() !== "")
            .map((m) => ({ metric: m.id, value: Number(scores[m.id]) }));
    if (!filled.length || filled.some((s) => !Number.isFinite(s.value))) {
      setError("请至少填写一项有效得分");
      return;
    }
    await onSave(
      () =>
        api("/self/assessments", {
          instrument: domain === "custom" ? "custom" : instrument!.id,
          observed_at: observedAt,
          scores: filled,
          ...(domain === "custom"
            ? { custom_label: customLabel, custom_unit: customUnit, custom_domain: customDomain }
            : {}),
          notes,
          agent_visible: agentVisible,
          scenes,
        }),
      "测评已记录",
    );
    setScores({});
    setCustomValue("");
    setNotes("");
  };
  const groups = useMemo(() => {
    const map = new Map<string, Entity[]>();
    for (const item of indicators) {
      const key = prop(item, "metric") === "custom" ? `custom:${item.title}` : prop(item, "metric") || item.title;
      map.set(key, [...(map.get(key) || []), item]);
    }
    for (const list of map.values())
      list.sort((a, b) => prop(b, "observed_at").localeCompare(prop(a, "observed_at")));
    return [...map.entries()];
  }, [indicators]);
  return (
    <div className="self-grid">
      <form className="panel self-form" onSubmit={submit}>
        <div className="section-heading">
          <h2>记录一次测评</h2>
          <span className="section-meta">本人控制</span>
        </div>
        <div className="segmented">
          {(
            [
              ["body", "生理自我"],
              ["psychology", "心理自我认定"],
              ["custom", "自定义"],
            ] as const
          ).map(([value, label]) => (
            <button
              type="button"
              key={value}
              className={domain === value ? "active" : ""}
              onClick={() => switchDomain(value)}
            >
              {label}
            </button>
          ))}
        </div>
        {domain === "custom" ? (
          <>
            <div className="form-grid">
              <TextField label="指标名称" value={customLabel} onChange={setCustomLabel} maxLength={40} />
              <TextField label="单位" value={customUnit} onChange={setCustomUnit} maxLength={20} required={false} />
            </div>
            <div className="form-grid">
              <TextField label="数值" type="number" step="any" value={customValue} onChange={setCustomValue} />
              <SelectField
                label="归入"
                value={customDomain}
                onChange={setCustomDomain}
                options={[
                  { value: "body", label: "生理自我" },
                  { value: "psychology", label: "心理自我认定" },
                ]}
              />
            </div>
          </>
        ) : (
          <>
            <label className="field">
              量表或指标组
              <select
                aria-label="量表或指标组"
                value={instrument?.id || ""}
                onChange={(event) => {
                  setInstrumentId(event.target.value);
                  setScores({});
                }}
              >
                {families.map((family) => (
                  <optgroup key={family} label={family}>
                    {instruments
                      .filter((i) => i.family === family)
                      .map((i) => (
                        <option key={i.id} value={i.id}>
                          {i.abbr === i.name ? i.name : `${i.abbr} · ${i.name}`}
                        </option>
                      ))}
                  </optgroup>
                ))}
              </select>
            </label>
            {instrument && (
              <div className="instrument-card">
                <strong>{instrument.construct}</strong>
                <span>{instrument.response}</span>
                {instrument.citation && (
                  <small>
                    <BookMarked size={12} /> {instrument.citation}
                  </small>
                )}
              </div>
            )}
            <div className="form-grid">
              {instrument?.metrics.map((metric) => (
                <TextField
                  key={metric.id}
                  label={`${metric.label}（${metric.min}–${metric.max}${metric.unit && !metric.unit.includes("–") && metric.unit !== "分" && metric.unit !== "均分" ? ` ${metric.unit}` : ""}）`}
                  type="number"
                  step="any"
                  min={metric.min}
                  max={metric.max}
                  required={false}
                  value={scores[metric.id] || ""}
                  onChange={(value) => setScores({ ...scores, [metric.id]: value })}
                />
              ))}
            </div>
          </>
        )}
        <div className="form-grid">
          <TextField label="观察日期" type="date" value={observedAt} onChange={setObservedAt} />
          <TextField label="备注" value={notes} onChange={setNotes} maxLength={4000} required={false} />
        </div>
        <SceneInput scenes={scenes} onChange={setScenes} legend="人生场景（不选则按量表默认）" />
        <label className="check-row">
          <input type="checkbox" checked={agentVisible} onChange={(e) => setAgentVisible(e.target.checked)} />
          允许本地模型读取这次测评
        </label>
        {error && <p className="form-error" role="alert">{error}</p>}
        <button className="button primary" disabled={busy}>
          {busy ? "保存中…" : "记录测评"}
        </button>
        <p className="muted small-text catalog-notice">{catalog.notice}</p>
      </form>
      <div className="panel self-list">
        <div className="section-heading">
          <h2>观察记录</h2>
          <span className="section-meta">按指标分组，最新在前</span>
        </div>
        {!groups.length && <Empty title="还没有自我观察" />}
        {groups.map(([key, records]) => (
          <IndicatorGroup key={key} records={records} busy={busy} onToggle={onToggle} />
        ))}
      </div>
    </div>
  );
}

function IndicatorGroup({
  records,
  busy,
  onToggle,
}: {
  records: Entity[];
  busy: boolean;
  onToggle: (entity: Entity) => void;
}) {
  const [latest, previous] = records;
  const value = Number(latest.properties.value);
  const before = previous ? Number(previous.properties.value) : NaN;
  const higher = prop(latest, "higher");
  const delta = Number.isFinite(value) && Number.isFinite(before) ? value - before : 0;
  const tone = delta === 0 || higher === "neutral" ? "" : (delta > 0) === (higher === "better") ? "green" : "coral";
  const range = latest.properties.range as number[] | null;
  const legacy = !prop(latest, "instrument");
  return (
    <article className="self-indicator">
      <div>
        <strong>{legacy ? labels[prop(latest, "metric")] || latest.title : latest.title}</strong>
        <span className="tag">{latest.kind === "body_metric" ? "生理" : "心理"}</span>
        {legacy && <span className="tag muted-tag">旧版自评</span>}
      </div>
      <p>
        <b>{String(latest.properties.value ?? "—")}</b> {prop(latest, "unit")}
        {range && <span className="muted"> · 范围 {range[0]}–{range[1]}</span>}
        {delta !== 0 && (
          <span className={`trend ${tone}`}>
            {delta > 0 ? <ArrowUp size={13} /> : <ArrowDown size={13} />}
            {Math.abs(Math.round(delta * 100) / 100)}
          </span>
        )}
        {previous && delta === 0 && (
          <span className="trend">
            <Minus size={13} />
          </span>
        )}
      </p>
      <small>
        {prop(latest, "observed_at").slice(0, 10)} · 共 {records.length} 次
        {records.length > 1 && ` · 上次 ${String(previous.properties.value)}`}
      </small>
      <div className="indicator-actions">
        <Scenes scenes={latest.scenes} />
        <VisibilityToggle entity={latest} busy={busy} onToggle={onToggle} />
      </div>
    </article>
  );
}

function Identities({
  catalog,
  identities,
  busy,
  onSave,
  onToggle,
}: {
  catalog: SelfCatalog;
  identities: Entity[];
  busy: boolean;
  onSave: Save;
  onToggle: (entity: Entity) => void;
}) {
  const [statement, setStatement] = useState("");
  const [category, setCategory] = useState("social");
  const [centrality, setCentrality] = useState("3");
  const [scenes, setScenes] = useState<string[]>([]);
  const [agentVisible, setAgentVisible] = useState(false);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    await onSave(
      () =>
        api("/self/identities", {
          statement: statement.trim().startsWith("我") ? statement.trim() : `我是${statement.trim()}`,
          category,
          centrality: Number(centrality),
          scenes,
          agent_visible: agentVisible,
        }),
      "自我认同已记录",
    );
    setStatement("");
  };
  return (
    <div className="self-grid">
      <form className="panel self-form" onSubmit={submit}>
        <div className="section-heading">
          <h2>“我是……”</h2>
          <span className="section-meta">二十句陈述测验（Kuhn & McPartland, 1954）</span>
        </div>
        <TextField label="我是……" value={statement} onChange={setStatement} placeholder="例如：我是一名终身学习者" />
        <div className="form-grid">
          <SelectField
            label="类别"
            value={category}
            onChange={setCategory}
            options={catalog.identity_categories.map((c) => ({ value: c.id, label: c.label }))}
          />
          <SelectField
            label="重要程度（1–5）"
            value={centrality}
            onChange={setCentrality}
            options={["1", "2", "3", "4", "5"].map((v) => ({ value: v, label: v }))}
          />
        </div>
        <p className="muted small-text field-help">
          {catalog.identity_categories.find((c) => c.id === category)?.description}
        </p>
        <SceneInput scenes={scenes} onChange={setScenes} />
        <label className="check-row">
          <input type="checkbox" checked={agentVisible} onChange={(e) => setAgentVisible(e.target.checked)} />
          作为核心设定提供给本地模型
        </label>
        <button className="button primary" disabled={busy}>记录自我认同</button>
      </form>
      <div className="panel self-list">
        {!identities.length && <Empty title="还没有自我认同陈述" />}
        {catalog.identity_categories.map((c) => {
          const list = identities.filter((i) => prop(i, "category") === c.id);
          if (!list.length) return null;
          return (
            <section key={c.id} className="identity-group">
              <h3>{c.label}</h3>
              {list.map((item) => (
                <article key={item.id} className="self-indicator">
                  <div>
                    <strong>{item.title}</strong>
                  </div>
                  <p>
                    <Dots value={Number(item.properties.centrality)} max={5} label="重要程度" />
                  </p>
                  <div className="indicator-actions">
                    <Scenes scenes={item.scenes} />
                    <VisibilityToggle entity={item} busy={busy} onToggle={onToggle} />
                  </div>
                </article>
              ))}
            </section>
          );
        })}
      </div>
    </div>
  );
}

function SelfRelations({
  ctx,
  catalog,
  relations,
  onSave,
  onToggle,
}: {
  ctx: PageContext;
  catalog: SelfCatalog;
  relations: Entity[];
  onSave: Save;
  onToggle: (entity: Entity) => void;
}) {
  const [stance, setStance] = useState("acceptance");
  const [title, setTitle] = useState("");
  const [intensity, setIntensity] = useState("3");
  const [target, setTarget] = useState("");
  const [notes, setNotes] = useState("");
  const [scenes, setScenes] = useState<string[]>([]);
  const [agentVisible, setAgentVisible] = useState(false);
  const targets = ctx.entities.filter((e) =>
    ["task", "resource", "directive", "note", "identity", "relationship", "person"].includes(e.kind),
  );
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    await onSave(
      () =>
        api("/self/relations", {
          title,
          stance,
          intensity: Number(intensity),
          target_id: target,
          notes,
          scenes,
          agent_visible: agentVisible,
        }),
      "自我关系已记录",
    );
    setTitle("");
    setNotes("");
    setTarget("");
  };
  const stanceLabel = (id: string) => catalog.stances.find((s) => s.id === id)?.label || id;
  return (
    <div className="self-grid">
      <form className="panel self-form" onSubmit={submit}>
        <div className="section-heading">
          <h2>与自我的关系</h2>
          <span className="section-meta">R = Relation</span>
        </div>
        <SelectField
          label="立场"
          value={stance}
          onChange={setStance}
          options={catalog.stances.map((s) => ({ value: s.id, label: s.label }))}
        />
        <p className="muted small-text field-help">{catalog.stances.find((s) => s.id === stance)?.description}</p>
        <TextField label="我的陈述" value={title} onChange={setTitle} placeholder="例如：失败时我愿意先照顾好自己" />
        <div className="form-grid">
          <SelectField
            label="强度（1–5）"
            value={intensity}
            onChange={setIntensity}
            options={["1", "2", "3", "4", "5"].map((v) => ({ value: v, label: v }))}
          />
          <SelectField
            label="关联对象（可选）"
            value={target}
            onChange={setTarget}
            placeholder="暂不关联"
            options={targets.map((e) => ({ value: e.id, label: `${labels[e.kind] || e.kind} · ${e.title}` }))}
          />
        </div>
        <TextField label="备注" value={notes} onChange={setNotes} maxLength={4000} required={false} />
        <SceneInput scenes={scenes} onChange={setScenes} />
        <label className="check-row">
          <input type="checkbox" checked={agentVisible} onChange={(e) => setAgentVisible(e.target.checked)} />
          作为核心设定提供给本地模型
        </label>
        <button className="button primary" disabled={ctx.busy}>保存自我关系</button>
      </form>
      <div className="panel self-list">
        <div className="section-heading">
          <h2>近期自我关系</h2>
          <span className="section-meta">本人维护</span>
        </div>
        {!relations.length && <Empty title="还没有记录与自我的关系" />}
        {relations.map((item) => {
          const linked = ctx.entities.find((e) => e.id === prop(item, "target_id"));
          return (
            <article className="self-indicator" key={item.id}>
              <div>
                <span className="tag">{stanceLabel(prop(item, "stance")) || "自我关系"}</span>
                <strong>{item.title}</strong>
              </div>
              <p>
                {item.properties.intensity !== undefined && (
                  <Dots value={Number(item.properties.intensity)} max={5} label="强度" />
                )}
                {linked && <span className="muted"> · 关联 {linked.title}</span>}
                {!item.properties.stance && prop(item, "relation") && <span className="muted"> · {prop(item, "relation")}</span>}
              </p>
              <small>{time(item.created_at)}</small>
              <div className="indicator-actions">
                <Scenes scenes={item.scenes} />
                <VisibilityToggle entity={item} busy={ctx.busy} onToggle={onToggle} />
              </div>
            </article>
          );
        })}
      </div>
    </div>
  );
}

interface Preview {
  core_principles: { id: string; title: string; requirement: string; scenes: string[] }[];
  self: {
    identity: { id: string; statement: string }[];
    self_relations: { id: string; statement: string; stance: string }[];
    indicators: { metric: string; label: string; value: unknown; unit: string; observed_at: string }[];
  };
  relationships: { id: string; title: string; closeness: number }[];
  rules: string[];
}
function AgentPreview({ fail }: { fail: (reason: unknown) => void }) {
  const [preview, setPreview] = useState<Preview | null>(null);
  useEffect(() => {
    void api<Preview>("/self/agent-preview").then(setPreview).catch(fail);
  }, [fail]);
  if (!preview) return <Empty title="正在生成预览" />;
  const section = (title: string, items: string[], empty: string) => (
    <section className="preview-section">
      <h3>
        {title}
        <span className="count">{items.length}</span>
      </h3>
      {items.length ? (
        <ul>
          {items.map((item, i) => (
            <li key={i}>{item}</li>
          ))}
        </ul>
      ) : (
        <p className="muted small-text">{empty}</p>
      )}
    </section>
  );
  return (
    <div className="panel agent-preview">
      <div className="privacy-note">
        <Eye size={18} />
        <span>
          这是路由交给本地模型的“自我本体核心原则”的完整内容：只包括你发布的执行方向和逐条标记为可见的自我记录。
          思维库无论是否解锁都不会出现在这里。
        </span>
      </div>
      {section(
        "核心原则（已发布的执行方向）",
        preview.core_principles.map((p) => `${p.title}：${p.requirement}`),
        "尚未发布执行方向。可在思维体系中把原则发布为执行方向。",
      )}
      {section("自我认同", preview.self.identity.map((i) => i.statement), "没有标记为可见的自我认同")}
      {section("自我关系", preview.self.self_relations.map((r) => r.statement), "没有标记为可见的自我关系")}
      {section(
        "指标（每项最新一次）",
        preview.self.indicators.map((i) => `${i.label}：${String(i.value)} ${i.unit || ""}（${String(i.observed_at).slice(0, 10)}）`),
        "没有标记为可见的指标",
      )}
      {section("人际关系", preview.relationships.map((r) => r.title), "没有标记为可见的人际关系")}
      {section("固定规则", preview.rules, "")}
      <details className="raw-json">
        <summary>查看原始 JSON</summary>
        <pre>{JSON.stringify(preview, null, 2)}</pre>
      </details>
    </div>
  );
}
