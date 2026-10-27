import { useEffect, useState } from "react";
import { CalendarClock, MessageCircleHeart, SlidersHorizontal, UserRound } from "lucide-react";
import { api, daysSince, num, prop, type Entity, type SelfCatalog } from "../api";
import { Dots, Empty, Scenes, StatusBadge, VisibilityToggle } from "../components";
import type { PageContext } from "../context";

const valence: Record<string, string> = {
  "-2": "很消极",
  "-1": "偏消极",
  "0": "平常",
  "1": "愉快",
  "2": "很愉快",
};

export function Relations({ ctx }: { ctx: PageContext }) {
  const [tab, setTab] = useState("relationships");
  const [catalog, setCatalog] = useState<SelfCatalog | null>(null);
  const { byKind, filter, onDialog, run, busy, fail } = ctx;
  useEffect(() => {
    void api<SelfCatalog>("/self/catalog").then(setCatalog).catch(fail);
  }, [fail]);
  const relationships = filter(byKind("relationship"));
  const interactions = filter(byKind("interaction")).sort((a, b) =>
    prop(b, "date").localeCompare(prop(a, "date")),
  );
  const people = filter(byKind("person"));
  const typeLabel = (id: string) =>
    catalog?.relationship_types.find((t) => t.id === id)?.label || id;
  const channelLabel = (id: string) =>
    catalog?.interaction_channels.find((c) => c.id === id)?.label || id;
  const toggle = (entity: Entity) =>
    void run(
      () =>
        api("/self/visibility", {
          id: entity.id,
          agent_visible: entity.properties.agent_visible !== true,
          expected_version: entity.version,
        }),
      "可见性已更新",
    );
  return (
    <>
      <div className="intro-panel">
        <div>
          <span className="eyebrow">R = RELATION</span>
          <h2>与他人的关系，也是自我本体的一部分</h2>
          <p>
            每段关系记录类型、亲密度（IOS 他人融入自我量表，Aron 等，1992）、满意度（RAS 关系评估量表，Hendrick，1988）和你希望的联系节奏；
            互动按事件只追加记录。超过节奏时只会生成待你判断的建议。
          </p>
        </div>
        <div className="metric-summary">
          <strong>{byKind("relationship").filter((r) => r.status === "active").length}</strong>
          <span>段进行中的关系</span>
        </div>
      </div>
      <div className="tabs">
        {(
          [
            ["relationships", "关系", relationships.length],
            ["interactions", "互动记录", interactions.length],
            ["people", "人物", people.length],
          ] as const
        ).map(([value, label, count]) => (
          <button
            key={value}
            className={tab === value ? "active" : ""}
            onClick={() => setTab(value)}
          >
            {label}
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
              return (
                <article className="relationship-card" key={r.id}>
                  <header>
                    <span className="world-icon self">
                      <UserRound size={20} />
                    </span>
                    <div>
                      <h2>{prop(r, "person_name") || r.title}</h2>
                      <span className="tag">{typeLabel(prop(r, "type"))}</span>
                    </div>
                    <StatusBadge status={r.status} />
                  </header>
                  <dl>
                    <dt>亲密度</dt>
                    <dd>
                      <Dots value={num(r, "closeness")} max={7} label="亲密度" />
                    </dd>
                    <dt>满意度</dt>
                    <dd>
                      {typeof satisfaction === "number" ? `${satisfaction} / 5` : "未评估"}
                    </dd>
                    <dt>联系节奏</dt>
                    <dd>
                      {interval > 0 ? `每 ${interval} 天` : "不提醒"}
                      {last ? ` · 上次 ${last}` : " · 尚无互动"}
                      {due && <span className="due-badge">已 {since} 天</span>}
                    </dd>
                  </dl>
                  {prop(r, "notes") && <p className="muted small-text">{prop(r, "notes")}</p>}
                  <footer>
                    <Scenes scenes={r.scenes} />
                    <VisibilityToggle entity={r} busy={busy} onToggle={toggle} />
                    <button
                      className="button secondary"
                      aria-label={`调整与${prop(r, "person_name")}的关系`}
                      onClick={() => onDialog({ type: "relationship_update", entity: r })}
                    >
                      <SlidersHorizontal size={14} />
                      调整
                    </button>
                    <button
                      className="button primary"
                      aria-label={`记录与${prop(r, "person_name")}的互动`}
                      onClick={() => onDialog({ type: "interaction", entity: r })}
                    >
                      <MessageCircleHeart size={14} />
                      记录互动
                    </button>
                  </footer>
                </article>
              );
            })}
          </div>
        ) : (
          <Empty title="还没有记录人际关系">
            <button className="text-button" onClick={() => onDialog({ type: "relationship" })}>
              新建第一段关系
            </button>
          </Empty>
        ))}
      {tab === "interactions" &&
        (interactions.length ? (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>日期</th>
                  <th>人物</th>
                  <th>方式</th>
                  <th>情绪体验</th>
                  <th>备注</th>
                </tr>
              </thead>
              <tbody>
                {interactions.map((i) => (
                  <tr key={i.id}>
                    <td className="muted nowrap">{prop(i, "date")}</td>
                    <td>{prop(i, "person_name")}</td>
                    <td>{channelLabel(prop(i, "channel"))}</td>
                    <td>{valence[prop(i, "valence")] || "平常"}</td>
                    <td>{prop(i, "notes")}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <Empty title="还没有互动记录" />
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
                        ? links.map((r) => typeLabel(prop(r, "type"))).join("、")
                        : "尚未建立关系"}
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
          <Empty title="还没有人物" />
        ))}
    </>
  );
}
