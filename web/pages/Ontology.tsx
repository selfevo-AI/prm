import { useState } from "react";
import { Activity, ArrowRight, Boxes, Brain, ChevronRight, GitBranch, LockKeyhole, MessageSquare, Network, Plus } from "lucide-react";
import { labels } from "../api";
import { Empty, Values } from "../components";
import type { PageContext } from "../context";

const worldIcons = { self: Activity, mental: Brain, digital: MessageSquare, physical: Boxes };
const worldNotes: Record<string, string> = {
  self: "本人记录与可见性控制",
  mental: "加密边界",
  digital: "可追溯建议与 Agent 导入",
  physical: "数量与现金分别记账",
};

export function Ontology({ ctx }: { ctx: PageContext }) {
  const [tab, setTab] = useState("schema");
  const [selected, setSelected] = useState("");
  const { workspace, search, onDialog, inScene } = ctx;
  const { entities, relations, ontology } = workspace;
  const selectedEntity = entities.find((e) => e.id === selected);
  const related = relations.filter(
    (r) => !selected || r.subject === selected || r.object === selected,
  );
  const classLabel = (id: string) =>
    labels[id] || ontology.classes.find((c) => c.id === id)?.label || id;
  return (
    <>
      <div className="tabs">
        <button
          className={tab === "schema" ? "active" : ""}
          onClick={() => setTab("schema")}
        >
          本体模型
        </button>
        <button
          className={tab === "instances" ? "active" : ""}
          onClick={() => setTab("instances")}
        >
          实体与关系<span>{relations.length}</span>
        </button>
      </div>
      {tab === "schema" ? (
        <>
          <div className="ontology-map four">
            {ontology.worlds.map((world) => {
              const WorldIcon = worldIcons[world.id as keyof typeof worldIcons] || Network;
              return (
                <section className={`ontology-world ${world.id}`} key={world.id}>
                  <header>
                    <WorldIcon size={25} />
                    <div>
                      <h2>{world.label}</h2>
                      <span>{worldNotes[world.id]}</span>
                    </div>
                  </header>
                  <div className="ontology-types">
                    {ontology.classes
                      .filter((c) => c.worlds.includes(world.id))
                      .map((c) => (
                        <div key={c.id}>
                          <span>{c.label}</span>
                          <span className={`category ${c.category}`}>
                            {ontology.categories.find((x) => x.id === c.category)?.label}
                          </span>
                          <code>{c.id}</code>
                          {c.storage === "vault" && <LockKeyhole size={12} />}
                        </div>
                      ))}
                  </div>
                  <footer>{worldNotes[world.id]}</footer>
                </section>
              );
            })}
          </div>
          <div className="axes-band">
            <div>
              <GitBranch size={19} />
              <strong>本体范畴（REA）</strong>
            </div>
            <span className="category-list">
              {ontology.categories.map((c) => (
                <span key={c.id} className={`category ${c.id}`} title={c.description}>
                  {c.label}
                </span>
              ))}
            </span>
            <span>资源—事件—主体，加上承诺、关系与观察</span>
          </div>
          <div className="axes-band">
            <div>
              <GitBranch size={19} />
              <strong>共同评价维度</strong>
            </div>
            <Values values={["need", "desire", "existence"]} />
            <span>每个实体可关联多个维度</span>
          </div>
          <div className="axes-band">
            <div>
              <GitBranch size={19} />
              <strong>人生场景</strong>
            </div>
            <span className="scenes">
              {ontology.scenes.map((s) => (
                <span key={s.id} className={`scene-chip ${s.id}`} title={s.description}>
                  {s.label}
                </span>
              ))}
            </span>
            <span>与世界、评价维度相互独立</span>
          </div>
          {ontology.layers.map((layer) => {
            const predicates = ontology.predicates.filter((p) => p.layer === layer.id);
            if (!predicates.length) return null;
            return (
              <div key={layer.id}>
                <div className="section-heading lower">
                  <h2>{layer.label}</h2>
                  <span className="section-meta">{layer.description}</span>
                </div>
                <div className="table-wrap">
                  <table>
                    <thead>
                      <tr>
                        <th>关系</th>
                        <th>起点类型</th>
                        <th>终点类型</th>
                        <th>建立方式</th>
                      </tr>
                    </thead>
                    <tbody>
                      {predicates.map((p) => (
                        <tr key={p.id}>
                          <td>
                            <strong>{p.label}</strong>{" "}
                            <code className="muted">{p.id}</code>
                          </td>
                          <td>{p.domain.map(classLabel).join("、")}</td>
                          <td>{p.range.map(classLabel).join("、")}</td>
                          <td>
                            {p.owner_settable
                              ? p.agent_proposable
                                ? "本人建立 · Agent 可建议"
                                : "本人建立"
                              : "随业务操作自动建立"}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            );
          })}
        </>
      ) : (
        <>
          <div className="section-heading">
            <h2>
              业务实体 <span className="count">{entities.length}</span>
            </h2>
            <button
              className="button secondary"
              onClick={() => onDialog({ type: "relation" })}
            >
              <Plus size={16} />
              建立关系
            </button>
          </div>
          <div className="entity-explorer">
            <div className="entity-list">
              {entities
                .filter(
                  (e) =>
                    inScene(e) &&
                    `${e.title} ${e.kind}`
                      .toLowerCase()
                      .includes(search.toLowerCase()),
                )
                .map((e) => (
                  <button
                    key={e.id}
                    className={selected === e.id ? "active" : ""}
                    onClick={() => setSelected(selected === e.id ? "" : e.id)}
                  >
                    <span className="entity-kind">{classLabel(e.kind)}</span>
                    <span>{e.title}</span>
                    <ChevronRight size={14} />
                  </button>
                ))}
            </div>
            <div className="relation-list">
              <div className="relation-heading">
                <Network size={18} />
                <strong>{selectedEntity?.title || "全部关系"}</strong>
              </div>
              {related.map((r) => (
                <div className="relation-row" key={r.id}>
                  <span>{entities.find((e) => e.id === r.subject)?.title}</span>
                  <span className="predicate">
                    {labels[r.predicate]}
                    <ArrowRight size={13} />
                  </span>
                  <span>{entities.find((e) => e.id === r.object)?.title}</span>
                </div>
              ))}
              {!related.length && <Empty title="还没有关联关系" />}
            </div>
          </div>
        </>
      )}
    </>
  );
}
