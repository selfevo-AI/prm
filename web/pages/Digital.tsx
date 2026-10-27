import { useEffect, useRef, useState, type FormEvent } from "react";
import {
  BookOpen,
  ChevronRight,
  GitBranch,
  LoaderCircle,
  MessageSquare,
  Plus,
  Send,
  Sparkles,
} from "lucide-react";
import { api, prop, time, type Entity } from "../api";
import { Empty, Scenes, Values } from "../components";
import type { PageContext } from "../context";
import { AgentHub } from "./AgentHub";

export function Digital({ ctx }: { ctx: PageContext }) {
  const { byKind, filter, onDialog } = ctx;
  const [tab, setTab] = useState("messages");
  return (
    <>
      <div className="tabs">
        {[
          ["messages", "会话"],
          ["note", "知识条目"],
          ["directive", "执行方向"],
          ["agents", "Agent 接入"],
        ].map(([value, label]) => (
          <button
            className={tab === value ? "active" : ""}
            key={value}
            onClick={() => setTab(value)}
          >
            {label}
          </button>
        ))}
        <span className="tab-end">
          <span className="mini-dot" />
          离线助理
        </span>
      </div>
      {tab === "messages" ? (
        <Messenger ctx={ctx} />
      ) : tab === "agents" ? (
        <AgentHub ctx={ctx} />
      ) : (
        <>
          <div className="section-heading">
            <h2>{tab === "note" ? "知识条目" : "已发布的执行方向"}</h2>
            {tab === "note" && (
              <button
                className="button secondary"
                onClick={() => onDialog({ type: "note" })}
              >
                <Plus size={16} />
                新建条目
              </button>
            )}
          </div>
          <div className="document-list">
            {filter(byKind(tab)).map((entity) => (
              <button
                key={entity.id}
                className="document-item"
                onClick={() => onDialog({ type: "detail", entity })}
              >
                <span
                  className={`world-icon ${tab === "note" ? "digital" : "mental"}`}
                >
                  {tab === "note" ? (
                    <BookOpen size={21} />
                  ) : (
                    <GitBranch size={21} />
                  )}
                </span>
                <div>
                  <strong>{entity.title}</strong>
                  <p>{prop(entity, "body")}</p>
                  <Values values={entity.values} />
                  <Scenes scenes={entity.scenes} />
                </div>
                <time>{time(entity.created_at)}</time>
                <ChevronRight size={16} />
              </button>
            ))}
          </div>
          {!byKind(tab).length && (
            <Empty
              title={
                tab === "note" ? "还没有知识条目" : "尚未从思维库发布执行方向"
              }
            />
          )}
        </>
      )}
    </>
  );
}

function Messenger({ ctx }: { ctx: PageContext }) {
  const { byKind, onDialog, refresh, search } = ctx;
  const channels = byKind("channel");
  const [selected, setSelected] = useState("");
  const channel =
    channels.find((c) => c.id === selected) ||
    channels.find((c) => c.title === "收件箱") ||
    channels[0];
  const [body, setBody] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const end = useRef<HTMLDivElement>(null);
  const messages: Entity[] = byKind("message")
    .filter((e) => prop(e, "channel_id") === channel?.id)
    .sort((a, b) => a.created_at.localeCompare(b.created_at));
  useEffect(() => {
    end.current?.scrollIntoView({ block: "nearest" });
  }, [messages.length, selected]);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!channel) return;
    setBusy(true);
    setError("");
    try {
      await api("/messages", { channel_id: channel.id, body });
      await refresh();
      setBody("");
    } catch (reason) {
      setError((reason as Error).message);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="messenger">
      <aside className="channels">
        <div className="channel-heading">
          <span>我的会话</span>
          <button
            className="icon-button"
            title="新建会话"
            aria-label="新建会话"
            onClick={() => onDialog({ type: "channel" })}
          >
            <Plus size={17} />
          </button>
        </div>
        {channels.map((c) => (
          <button
            className={`channel ${c.id === channel?.id ? "active" : ""}`}
            key={c.id}
            onClick={() => setSelected(c.id)}
          >
            <MessageSquare size={18} />
            <span>{c.title}</span>
          </button>
        ))}
      </aside>
      <section className="conversation">
        <header>
          <div>
            <MessageSquare size={18} />
            <h2>{channel?.title || "会话"}</h2>
          </div>
          <span className="section-meta">{messages.length} 条消息</span>
        </header>
        <div className="message-list">
          {messages.length ? (
            // Conversations stay whole: only the search box (not the scene filter) applies.
            messages
              .filter((m) => prop(m, "body").toLowerCase().includes(search.toLowerCase()))
              .map((message) => (
              <div
                className={`message ${prop(message, "role") === "owner" ? "from-owner" : ""}`}
                key={message.id}
              >
                <span
                  className={`message-avatar ${prop(message, "role") === "owner" ? "" : "assistant-avatar"}`}
                >
                  {prop(message, "role") === "owner" ? "我" : <Sparkles size={17} />}
                </span>
                <div>
                  <div className="message-meta">
                    {prop(message, "role") === "owner" ? "我" : "离线助理 · 规则分析"}
                    <time>{time(message.created_at)}</time>
                  </div>
                  <div className="message-body">{prop(message, "body")}</div>
                </div>
              </div>
            ))
          ) : (
            <Empty title="收集一件待办，开始今天的协同" />
          )}
          <div ref={end} />
        </div>
        <form className="composer" onSubmit={submit}>
          <textarea
            aria-label="消息内容"
            required
            maxLength={8000}
            placeholder="记录一条消息或待办…"
            value={body}
            onChange={(event) => setBody(event.target.value)}
          />
          <div>
            {error && (
              <span className="form-error" role="alert">
                {error}
              </span>
            )}
            <button className="button primary" disabled={busy || !body.trim() || !channel}>
              {busy ? <LoaderCircle className="spin" size={16} /> : <Send size={16} />}
              发送
            </button>
          </div>
        </form>
      </section>
    </div>
  );
}
