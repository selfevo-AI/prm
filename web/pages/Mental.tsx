import { useState } from "react";
import {
  ArrowUpRight,
  Brain,
  Check,
  Clock3,
  History,
  LockKeyhole,
  Pencil,
  Plus,
  ShieldCheck,
  UnlockKeyhole,
} from "lucide-react";
import { labels, time, type Dialog, type Vault } from "../api";
import { Empty, Values } from "../components";
import { Permissions } from "../Permissions";

export function Mental({
  vault,
  onDialog,
}: {
  vault: Vault | null;
  onDialog: (dialog: Dialog) => void;
}) {
  const [kind, setKind] = useState("principle");
  if (!vault)
    return (
      <section className="vault-locked">
        <div className="vault-seal">
          <LockKeyhole size={37} strokeWidth={1.4} />
        </div>
        <span className="eyebrow">PRIVATE MENTAL SPACE</span>
        <h2>你的判断，由你守护</h2>
        <p>思维库已锁定</p>
        <button
          className="button primary"
          onClick={() => onDialog({ type: "unlock" })}
        >
          <UnlockKeyhole size={17} />
          本人解锁
        </button>
        <div className="vault-boundaries">
          <span>
            <ShieldCheck size={16} />
            独立加密
          </span>
          <span>
            <Pencil size={16} />
            本人确认
          </span>
          <span>
            <LockKeyhole size={16} />5 分钟自动锁定
          </span>
        </div>
      </section>
    );
  const items = vault.items.filter((item) => item.kind === kind);
  return (
    <>
      <div className="vault-status">
        <ShieldCheck size={17} />
        <span>本人修改 · 按授权只读</span>
        <span className="muted">版本 {vault.version}</span>
        <span className="vault-status-right">
          <Clock3 size={14} />
          切换页面或离开窗口时锁定
        </span>
      </div>
      <Permissions vault={vault} />
      <div className="tabs">
        {["principle", "goal", "reflection"].map((value) => (
          <button
            className={kind === value ? "active" : ""}
            key={value}
            onClick={() => setKind(value)}
          >
            {labels[value]}
            <span>
              {vault.items.filter((item) => item.kind === value).length}
            </span>
          </button>
        ))}
      </div>
      {items.length ? (
        <div className="core-list">
          {items.map((item) => (
            <article key={item.id} className="core-entry">
              <div className="core-entry-heading">
                <span className="world-icon mental">
                  <Brain size={21} />
                </span>
                <div>
                  <h2>{item.title}</h2>
                  <span className="muted small-text">
                    版本 {item.version} · 最近确认 {time(item.confirmed_at)}
                  </span>
                </div>
                <Values values={item.values} />
              </div>
              <p className="preserve-text">{item.body}</p>
              <footer>
                <button
                  className="button secondary"
                  onClick={() => onDialog({ type: "review", core: item })}
                >
                  <Check size={16} />
                  今日确认
                </button>
                <button
                  className="text-button"
                  onClick={() => onDialog({ type: "core", core: item })}
                >
                  <Pencil size={15} />
                  编辑
                </button>
                <button
                  className="text-button"
                  onClick={() => onDialog({ type: "history", core: item })}
                >
                  <History size={15} />
                  版本
                </button>
                <button
                  className="text-button publish-button"
                  onClick={() => onDialog({ type: "publish", core: item })}
                >
                  <ArrowUpRight size={16} />
                  发布执行方向
                </button>
              </footer>
            </article>
          ))}
        </div>
      ) : (
        <Empty title={`尚未记录${labels[kind]}`}>
          <button
            className="text-button"
            onClick={() =>
              onDialog({
                type: "core",
                core: {
                  id: "",
                  kind,
                  title: "",
                  body: "",
                  values: [],
                  version: 0,
                  updated_at: "",
                  confirmed_at: "",
                },
              })
            }
          >
            <Plus size={15} />
            由我开始记录
          </button>
        </Empty>
      )}
    </>
  );
}
