import { useState } from "react";
import { ArrowRight, Boxes, Check, FileText, Network, Sparkles, X } from "lucide-react";
import { api, label, prop, time, type Entity } from "../api";
import { Empty, Scenes, StatusBadge } from "../components";
import { find, type PageContext } from "../context";
import { t } from "../i18n";

const icons = { create_task: Sparkles, create_entity: Boxes, create_relation: Network };
/** Human-readable provider of a proposal. */
function provider(entity: Entity) {
  const value = prop(entity, "provider");
  if (value === "offline-rules") return t("离线规则");
  if (value.startsWith("local:")) return t("本地模型 · {name}", { name: value.slice(6) });
  return t("本地模型");
}
const entityFields: Record<string, [string, string][]> = {
  resource: [
    ["unit", "单位"],
    ["location", "位置"],
    ["minimum", "最低库存"],
    ["sku", "SKU"],
    ["category", "分类"],
  ],
  note: [["body", "内容"]],
  person: [["notes", "备注"]],
  supplier: [
    ["contact", "联系方式"],
    ["tax_id", "税号"],
  ],
  location: [
    ["code", "编码"],
    ["notes", "备注"],
  ],
};

export function Decisions({ ctx }: { ctx: PageContext }) {
  const { byKind, filter, onDialog, run, busy, entities } = ctx;
  const [tab, setTab] = useState("pending");
  const [origin, setOrigin] = useState("");
  const proposals = byKind("proposal");
  const list = filter(
    proposals.filter(
      (e) =>
        (tab === "pending" ? e.status === "pending" : e.status !== "pending") &&
        (!origin ||
          (origin === "import"
            ? prop(e, "source_kind") === "agent_import"
            : prop(e, "source_kind") !== "agent_import")),
    ),
  );
  const reject = (entity: Entity) =>
    void run(
      () =>
        api(`/proposals/${entity.id}/decide`, {
          decision: "reject",
          expected_version: entity.version,
        }),
      t("建议已拒绝"),
    );
  return (
    <>
      <div className="tabs">
        <button
          className={tab === "pending" ? "active" : ""}
          onClick={() => setTab("pending")}
        >
          {t("待确认")}
          <span>{proposals.filter((e) => e.status === "pending").length}</span>
        </button>
        <button
          className={tab === "history" ? "active" : ""}
          onClick={() => setTab("history")}
        >
          {t("处理记录")}
        </button>
        <label className="tab-end tab-filter">
          {t("来源")}
          <select aria-label={t("建议来源")} value={origin} onChange={(e) => setOrigin(e.target.value)}>
            <option value="">{t("全部")}</option>
            <option value="analysis">{t("工作区分析")}</option>
            <option value="import">{t("外部 Agent 导入")}</option>
          </select>
        </label>
      </div>
      {list.length ? (
        <div className="proposal-list">
          {list.map((e) => {
            const action = (prop(e, "action") || "create_task") as keyof typeof icons;
            const Icon = icons[action] || Sparkles;
            const source = find(entities, prop(e, "source_id"));
            const spec = (e.properties.entity || {}) as { kind?: string; properties?: Record<string, unknown> };
            const relation = (e.properties.relation || {}) as Record<string, string>;
            const principles = (e.properties.principles as { id: string; title: string }[] | undefined) || [];
            return (
              <article className="proposal" key={e.id}>
                <div className="proposal-top">
                  <span className="world-icon digital">
                    <Icon size={21} />
                  </span>
                  <div>
                    <h2>{e.title}</h2>
                    <span className="muted small-text">
                      {provider(e)} · {time(e.created_at)}
                    </span>
                  </div>
                  <StatusBadge status={e.status} />
                </div>
                <div className="proposal-kind">
                  <span className="tag">
                    {action === "create_entity"
                      ? t("纳入{kind}", { kind: spec.kind ? label(spec.kind) : t("实体") })
                      : label(action)}
                  </span>
                  <Scenes scenes={e.scenes} />
                </div>
                {action === "create_entity" && spec.properties && (
                  <dl className="proposal-fields">
                    {(entityFields[spec.kind || ""] || [])
                      .filter(([key]) => String(spec.properties?.[key] ?? "") !== "")
                      .map(([key, name]) => (
                        <div key={key}>
                          <dt>{t(name)}</dt>
                          <dd>{String(spec.properties?.[key])}</dd>
                        </div>
                      ))}
                  </dl>
                )}
                {action === "create_relation" && (
                  <div className="relation-row proposal-relation">
                    <span>{relation.subject_title}</span>
                    <span className="predicate">
                      {label(relation.predicate)}
                      <ArrowRight size={13} />
                    </span>
                    <span>{relation.object_title}</span>
                  </div>
                )}
                <p className="preserve-text">{prop(e, "rationale")}</p>
                {principles.length > 0 && (
                  <div className="principles">
                    {t("依据的核心原则：")}
                    {principles.map((p) => (
                      <span key={p.id} className="tag">
                        {p.title}
                      </span>
                    ))}
                  </div>
                )}
                <div className="proposal-evidence">
                  <FileText size={15} />
                  <span>
                    {prop(e, "source_kind") === "agent_import" ? t("来源") : t("依据")}
                  </span>
                  <button
                    className="text-button"
                    onClick={() => {
                      if (source) onDialog({ type: "detail", entity: source });
                    }}
                  >
                    {source?.title || t("原始记录")}
                  </button>
                  {prop(e, "source_kind") !== "agent_import" && (
                    <span className="mono">v{prop(e, "source_version")}</span>
                  )}
                </div>
                {e.status === "pending" && (
                  <footer>
                    <span className="small-text muted">
                      {action === "create_task"
                        ? t("采纳后创建任务")
                        : action === "create_entity"
                          ? t("采纳后写入本体")
                          : t("采纳后建立关系")}
                    </span>
                    <button
                      className="button secondary"
                      disabled={busy}
                      onClick={() => reject(e)}
                    >
                      <X size={15} />
                      {t("拒绝")}
                    </button>
                    <button
                      className="button primary"
                      disabled={busy}
                      onClick={() => onDialog({ type: "approve", entity: e })}
                    >
                      <Check size={16} />
                      {t("审阅并采纳")}
                    </button>
                  </footer>
                )}
              </article>
            );
          })}
        </div>
      ) : (
        <Empty
          title={tab === "pending" ? t("当前没有待确认的建议") : t("尚无处理记录")}
        />
      )}
    </>
  );
}
