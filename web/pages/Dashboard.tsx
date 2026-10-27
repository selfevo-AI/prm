import {
  Activity,
  ArrowRight,
  ArrowUpRight,
  Boxes,
  Brain,
  Check,
  ClipboardCheck,
  Database,
  GitBranch,
  ListTodo,
  LockKeyhole,
  MessageSquare,
  Plus,
  Sparkles,
  UsersRound,
  Wallet,
} from "lucide-react";
import { api, daysSince, labels, money, num, prop, SCENES, sceneLabel, today } from "../api";
import { Empty, Values } from "../components";
import type { PageContext } from "../context";

export function Dashboard({ ctx }: { ctx: PageContext }) {
  const { byKind, filter, inScene, onDialog, go, busy, run, workspace } = ctx;
  const tasks = filter(byKind("task"));
  const allTasks = byKind("task").filter(inScene);
  const proposals = byKind("proposal").filter(
    (e) => e.status === "pending" && inScene(e),
  );
  const cash = byKind("cash_entry").filter(
    (e) => inScene(e) && prop(e, "date").startsWith(today().slice(0, 7)),
  );
  const balance = cash.reduce(
    (sum, e) =>
      sum +
      (prop(e, "direction") === "income" ? 1 : -1) * num(e, "amount_cents"),
    0,
  );
  const resources = byKind("resource").filter(inScene);
  const low = resources.filter((e) => num(e, "quantity") < num(e, "minimum"));
  const due = byKind("relationship").filter((r) => {
    const interval = num(r, "contact_interval_days");
    if (r.status !== "active" || interval <= 0 || !inScene(r)) return false;
    return daysSince(prop(r, "last_contact_at") || r.created_at) >= interval;
  });
  const queuedImports = byKind("agent_import").filter((i) =>
    ["queued", "processing"].includes(i.status),
  );
  const directives = byKind("directive").filter(
    (e) => e.status === "active" && inScene(e),
  );
  const demoAdded = workspace.audit.some((a) => a.action === "demo.created");
  const open = (kind: string, scene: string) =>
    byKind(kind).filter(
      (e) =>
        (e.scenes || []).includes(scene) &&
        !["done", "cancelled", "ended", "rejected", "approved"].includes(e.status),
    ).length;
  const setStatus = (task: (typeof tasks)[number], status: string) =>
    void run(
      () =>
        api(`/tasks/${task.id}/status`, {
          status,
          expected_version: task.version,
        }),
      "任务已更新",
    );
  return (
    <>
      <div className="overview-bar">
        <div>
          <span className="today-date">
            {new Date().toLocaleDateString("zh-CN", {
              month: "long",
              day: "numeric",
              weekday: "long",
            })}
          </span>
          <span className="overview-divider" />
          <span>让资源与关系服务于你的方向</span>
        </div>
        <button className="text-button" onClick={() => go("mental")}>
          回顾思维体系
          <ArrowRight size={15} />
        </button>
      </div>
      <section className="metrics" aria-label="工作区概况">
        <div className="metric">
          <div>
            <span>进行中的任务</span>
            <ListTodo size={18} />
          </div>
          <strong>
            {allTasks.filter((e) => ["todo", "doing"].includes(e.status)).length}
            <small>项</small>
          </strong>
          <span className="metric-note">
            已完成 {allTasks.filter((e) => e.status === "done").length} 项
          </span>
        </div>
        <div className="metric">
          <div>
            <span>等待你的判断</span>
            <ClipboardCheck size={18} />
          </div>
          <strong>
            {proposals.length}
            <small>条</small>
          </strong>
          <span className="metric-note coral">建议待确认</span>
        </div>
        <div className="metric">
          <div>
            <span>本月净现金流</span>
            <Wallet size={18} />
          </div>
          <strong className="money-metric">{money(balance)}</strong>
          <span className="metric-note">{cash.length} 笔收支 · CNY</span>
        </div>
        <div className="metric">
          <div>
            <span>资源需要关注</span>
            <Boxes size={18} />
          </div>
          <strong>
            {low.length}
            <small>项</small>
          </strong>
          <span className="metric-note">共管理 {resources.length} 项资源</span>
        </div>
      </section>
      <div className="dashboard-columns">
        <section className="task-section">
          <div className="section-heading">
            <h2>
              执行清单 <span className="count">{tasks.length}</span>
            </h2>
            <button
              className="text-button"
              onClick={() => onDialog({ type: "task" })}
            >
              <Plus size={15} />
              添加
            </button>
          </div>
          {tasks.length ? (
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>任务</th>
                    <th>评价维度</th>
                    <th>计划日期</th>
                    <th>状态</th>
                  </tr>
                </thead>
                <tbody>
                  {tasks.map((task) => (
                    <tr key={task.id}>
                      <td>
                        <div className="task-name">
                          <button
                            className={`check-button ${task.status === "done" ? "checked" : ""}`}
                            title={
                              task.status === "done"
                                ? "重新打开任务"
                                : "确认完成任务"
                            }
                            aria-label={`${task.status === "done" ? "重新打开" : "完成"}：${task.title}`}
                            disabled={busy}
                            onClick={() =>
                              setStatus(task, task.status === "done" ? "todo" : "done")
                            }
                          >
                            {task.status === "done" && <Check size={13} />}
                          </button>
                          <button
                            className="title-button"
                            onClick={() =>
                              onDialog({ type: "detail", entity: task })
                            }
                          >
                            {task.title}
                          </button>
                        </div>
                      </td>
                      <td>
                        <Values values={task.values} />
                      </td>
                      <td className="muted nowrap">
                        {prop(task, "due_date") || "未安排"}
                      </td>
                      <td>
                        <select
                          className="status-select"
                          aria-label={`${task.title}的状态`}
                          value={task.status}
                          disabled={busy}
                          onChange={(event) =>
                            setStatus(task, event.target.value)
                          }
                        >
                          {["todo", "doing", "done", "cancelled"].map(
                            (value) => (
                              <option key={value} value={value}>
                                {labels[value]}
                              </option>
                            ),
                          )}
                        </select>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <Empty title="还没有执行任务">
              <button
                className="text-button"
                onClick={() => onDialog({ type: "task" })}
              >
                创建第一项任务
                <Plus size={15} />
              </button>
            </Empty>
          )}
          <div className="section-heading lower">
            <h2>人生场景</h2>
            <span className="section-meta">点选后所有视图按场景筛选</span>
          </div>
          <div className="scene-overview">
            {SCENES.map((scene) => (
              <button
                key={scene}
                className={`scene-card ${scene} ${ctx.scene === scene ? "active" : ""}`}
                onClick={() => ctx.setScene(ctx.scene === scene ? "" : scene)}
              >
                <strong>{sceneLabel(scene)}</strong>
                <span>
                  {open("task", scene)} 项任务 · {open("relationship", scene)} 段关系
                </span>
              </button>
            ))}
          </div>
          <div className="section-heading lower">
            <h2>四个世界</h2>
            <span className="section-meta">资源与关系在其中交叉</span>
          </div>
          <div className="world-overview">
            <button onClick={() => go("self")}>
              <span className="world-icon self">
                <Activity size={23} />
              </span>
              <strong>自我世界</strong>
              <span>身体 · 心理 · 认同 · 关系</span>
              <div>
                {byKind("body_metric").length + byKind("psychological_metric").length} 条观察
                <ArrowUpRight size={16} />
              </div>
            </button>
            <button onClick={() => go("mental")}>
              <span className="world-icon mental">
                <Brain size={23} />
              </span>
              <strong>思维世界</strong>
              <span>原则 · 目标 · 反思</span>
              <div>
                <LockKeyhole size={13} />
                仅本人维护
                <ArrowUpRight size={16} />
              </div>
            </button>
            <button onClick={() => go("digital")}>
              <span className="world-icon digital">
                <MessageSquare size={23} />
              </span>
              <strong>数字世界</strong>
              <span>会话 · 知识 · Agent</span>
              <div>
                {byKind("message").filter((e) => prop(e, "role") === "owner").length}{" "}
                条消息
                <ArrowUpRight size={16} />
              </div>
            </button>
            <button onClick={() => go("physical")}>
              <span className="world-icon physical">
                <Boxes size={23} />
              </span>
              <strong>物理世界</strong>
              <span>物料 · 采购 · 收支</span>
              <div>
                {byKind("resource").length} 项资源
                <ArrowUpRight size={16} />
              </div>
            </button>
          </div>
        </section>
        <aside className="dashboard-aside">
          <section>
            <div className="section-heading">
              <h2>
                <GitBranch size={17} />
                执行方向
              </h2>
              <button
                className="icon-button"
                title="查看执行方向"
                aria-label="查看执行方向"
                onClick={() => go("digital")}
              >
                <ArrowUpRight size={16} />
              </button>
            </div>
            {directives.length ? (
              directives.slice(0, 4).map((d) => (
                <button
                  className="direction-item"
                  key={d.id}
                  onClick={() => onDialog({ type: "detail", entity: d })}
                >
                  <strong>{d.title}</strong>
                  <p>{prop(d, "body")}</p>
                  <Values values={d.values} />
                </button>
              ))
            ) : (
              <div className="aside-empty">
                <LockKeyhole size={22} />
                <p>尚未发布执行方向</p>
                <button className="text-button" onClick={() => go("mental")}>
                  打开思维体系
                  <ArrowRight size={14} />
                </button>
              </div>
            )}
          </section>
          <section className="attention-section">
            <div className="section-heading">
              <h2>
                <Activity size={17} />
                需要关注
              </h2>
            </div>
            {low.map((e) => (
              <div className="attention-item" key={e.id}>
                <span className="coral-dot" />
                <div>
                  <strong>{e.title}</strong>
                  <p>
                    库存 {num(e, "quantity")} {prop(e, "unit")} · 最低{" "}
                    {num(e, "minimum")}
                  </p>
                </div>
              </div>
            ))}
            {due.map((r) => (
              <div className="attention-item" key={r.id}>
                <span className="coral-dot" />
                <div>
                  <strong>{prop(r, "person_name")}</strong>
                  <p>超过 {num(r, "contact_interval_days")} 天未联系</p>
                </div>
              </div>
            ))}
            {!low.length && !due.length && (
              <p className="muted small-text">当前没有库存或联系提醒</p>
            )}
            {proposals.length > 0 && (
              <button
                className="attention-link"
                onClick={() => go("decisions")}
              >
                <Sparkles size={17} />
                {proposals.length} 条建议等待判断
                <ArrowRight size={15} />
              </button>
            )}
            {queuedImports.length > 0 && (
              <button className="attention-link" onClick={() => go("digital")}>
                <UsersRound size={17} />
                {queuedImports.length} 个 Agent 导入等待本地模型
                <ArrowRight size={15} />
              </button>
            )}
          </section>
          {!demoAdded && (
            <button
              className="demo-button"
              onClick={() =>
                void run(() => api("/demo", {}), "已添加业务演示数据")
              }
              disabled={busy}
            >
              <Database size={16} />
              <span>添加业务演示数据</span>
              <Plus size={14} />
            </button>
          )}
        </aside>
      </div>
    </>
  );
}
