import { useEffect, useState } from "react";
import {
  Bot,
  CalendarClock,
  MessageCircleHeart,
  SlidersHorizontal,
  UserRound,
} from "lucide-react";
import { api, daysSince, label, num, prop, type Entity, type SelfCatalog } from "../api";
import { Dots, Empty, Scenes, StatusBadge, VisibilityToggle } from "../components";
import type { PageContext } from "../context";
import { t } from "../i18n";

const valence: Record<string, string> = {
  "-2": "很消极",
  "-1": "偏消极",
  "0": "平常",
  "1": "愉快",
  "2": "很愉快",
};
const isAgentRelation = (r: Entity) => prop(r, "party_kind") === "ai_agent";
const partyName = (r: Entity) => prop(r, "agent_name") || prop(r, "person_name") || r.title;

export function Relations({ ctx }: { ctx: PageContext }) {
  const [tab, setTab] = useState("relationships");
  const [catalog, setCatalog] = useState<SelfCatalog | null>(null);
  const { byKind, filter, onDialog, run, busy, fail, go } = ctx;
  useEffect(() => {
    void api<SelfCatalog>("/self/catalog").then(setCatalog).catch(fail);
  }, [fail]);
  const relationships = filter(byKind("relationship"));
  const interactions = filter(byKind("interaction")).sort((a, b) =>
    prop(b, "date").localeCompare(prop(a, "date")),
  );
  const people = filter(byKind("person"));
  const agents = filter(byKind("ai_agent"));
  const typeLabel = (id: string) =>
    catalog?.relationship_types.find((entry) => entry.id === id)?.label || id;
  const channelLabel = (id: string) =>
    catalog?.interaction_channels.find((entry) => entry.id === id)?.label || id;
  const toggle = (entity: Entity) =>
    void run(
      () =>
        api("/self/visibility", {
          id: entity.id,
          agent_visible: entity.properties.agent_visible !== true,
          expected_version: entity.version,
        }),
      t("可见性已更新"),
    );
  const agentRelation = (agent: Entity) =>
    byKind("relationship").find((r) => prop(r, "agent_id") === agent.id);
  return (
    <>
      <div className="intro-panel">
        <div>
          <span className="eyebrow">R = RELATION</span>
          <h2>{t("与他人、与 AI Agent 的关系，也是自我本体的一部分")}</h2>
          <p>
            {t(
              "每段关系记录类型、亲密度（IOS 他人融入自我量表，Aron 等，1992）、满意度（RAS 关系评估量表，Hendrick，1988）和你希望的联系节奏；互动按事件只追加记录。AI Agent 的关系按接入方式与人生场景建立，在“数字空间 · Agent 接入”中签发令牌。",
            )}
          </p>
        </div>
        <div className="metric-summary">
          <strong>{byKind("relationship").filter((r) => r.status === "active").length}</strong>
          <span>{t("段进行中的关系")}</span>
        </div>
      </div>
      <div className="tabs">
        {(
          [
            ["relationships", "关系", relationships.length],
            ["interactions", "互动记录", interactions.length],
            ["people", "人物", people.length],
            ["agents", "AI Agent", agents.length],
          ] as const
        ).map(([value, labelText, count]) => (
          <button
            key={value}
            className={tab === value ? "active" : ""}
            onClick={() => setTab(value)}
          >
            {t(labelText)}
            <span>{count}</span>
          </button>
        ))}
      </div>
      {tab === "relationships" &&
        (relationships.length ? (
          <div className="relationship-grid">
            {relationships.map((r) => {
              const interval = num(r, "contact_interval_days");
              const last = prop(r, "last_contact_at");
              const since = daysSince(last || r.created_at);
              const due = r.status === "active" && interval > 0 && since >= interval;
              const satisfaction = r.properties.satisfaction;
              const agent = isAgentRelation(r);
              const name = partyName(r);
              return (
                <article className="relationship-card" key={r.id}>
                  <header>
                    <span className={`world-icon ${agent ? "digital" : "self"}`}>
                      {agent ? <Bot size={20} /> : <UserRound size={20} />}
                    </span>
                    <div>
                      <h2>{name}</h2>
                      <span className="tag">{t(typeLabel(prop(r, "type")))}</span>
                      {agent && prop(r, "platform") && (
                        <span className="tag">{label(prop(r, "platform"))}</span>
                      )}
                    </div>
                    <StatusBadge status={r.status} />
                  </header>
                  <dl>
                    <dt>{t("亲密度")}</dt>
                    <dd>
                      <Dots value={num(r, "closeness")} max={7} label="亲密度" />
                    </dd>
                    <dt>{t("满意度")}</dt>
                    <dd>
                      {typeof satisfaction === "number" ? `${satisfaction} / 5` : t("未评估")}
                    </dd>
                    <dt>{t("联系节奏")}</dt>
                    <dd>
                      {interval > 0 ? t("每 {days} 天", { days: interval }) : t("不提醒")}
                      {last
                        ? t(" · 上次 {date}", { date: last })
                        : t(" · 尚无互动")}
                      {due && <span className="due-badge">{t("已 {days} 天", { days: since })}</span>}
                    </dd>
                  </dl>
                  {prop(r, "notes") && <p className="muted small-text">{prop(r, "notes")}</p>}
                  <footer>
                    <Scenes scenes={r.scenes} />
                    <VisibilityToggle entity={r} busy={busy} onToggle={toggle} />
                    <button
                      className="button secondary"
                      aria-label={t("调整与{name}的关系", { name })}
                      onClick={() => onDialog({ type: "relationship_update", entity: r })}
                    >
                      <SlidersHorizontal size={14} />
                      {t("调整")}
                    </button>
                    <button
                      className="button primary"
                      aria-label={t("记录与{name}的互动", { name })}
                      onClick={() => onDialog({ type: "interaction", entity: r })}
                    >
                      <MessageCircleHeart size={14} />
                      {t("记录互动")}
                    </button>
                  </footer>
                </article>
              );
            })}
          </div>
        ) : (
          <Empty title={t("还没有记录人际关系")}>
            <button className="text-button" onClick={() => onDialog({ type: "relationship" })}>
              {t("新建第一段关系")}
            </button>
          </Empty>
        ))}
      {tab === "interactions" &&
        (interactions.length ? (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>{t("日期")}</th>
                  <th>{t("对方")}</th>
                  <th>{t("方式")}</th>
                  <th>{t("情绪体验")}</th>
                  <th>{t("备注")}</th>
                </tr>
              </thead>
              <tbody>
                {interactions.map((i) => (
                  <tr key={i.id}>
                    <td className="muted nowrap">{prop(i, "date")}</td>
                    <td>{prop(i, "agent_name") || prop(i, "person_name")}</td>
                    <td>{t(channelLabel(prop(i, "channel")))}</td>
                    <td>{t(valence[prop(i, "valence")] || "平常")}</td>
                    <td>{prop(i, "notes")}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <Empty title={t("还没有互动记录")} />
        ))}
      {tab === "people" &&
        (people.length ? (
          <div className="document-list">
            {people.map((person) => {
              const links = byKind("relationship").filter((r) => prop(r, "person_id") === person.id);
              return (
                <button
                  key={person.id}
                  className="document-item"
                  onClick={() => onDialog({ type: "detail", entity: person })}
                >
                  <span className="world-icon self">
                    <UserRound size={21} />
                  </span>
                  <div>
                    <strong>{person.title}</strong>
                    <p>
                      {links.length
                        ? links.map((r) => t(typeLabel(prop(r, "type")))).join(t("、"))
                        : t("尚未建立关系")}
                      {prop(person, "notes") && ` · ${prop(person, "notes")}`}
                    </p>
                    <Scenes scenes={person.scenes} />
                  </div>
                  <CalendarClock size={16} />
                </button>
              );
            })}
          </div>
        ) : (
          <Empty title={t("还没有人物")} />
        ))}
      {tab === "agents" &&
        (agents.length ? (
          <div className="document-list">
            {agents.map((agent) => {
              const relation = agentRelation(agent);
              return (
                <button
                  key={agent.id}
                  className="document-item"
                  onClick={() => onDialog({ type: "detail", entity: agent })}
                >
                  <span className="world-icon digital">
                    <Bot size={21} />
                  </span>
                  <div>
                    <strong>{agent.title}</strong>
                    <p>
                      {label(prop(agent, "platform"))}
                      {relation
                        ? ` · ${t(typeLabel(prop(relation, "type")))}`
                        : ` · ${t("尚未建立关系")}`}
                      {prop(agent, "notes") && ` · ${prop(agent, "notes")}`}
                    </p>
                    <Scenes scenes={agent.scenes} />
                  </div>
                  {relation ? (
                    <StatusBadge status={relation.status} />
                  ) : (
                    <Bot size={16} />
                  )}
                </button>
              );
            })}
          </div>
        ) : (
          <Empty title={t("还没有接入 AI Agent")}>
            <button className="text-button" onClick={() => go("digital")}>
              {t("到数字空间签发接入令牌")}
            </button>
          </Empty>
        ))}
    </>
  );
}
