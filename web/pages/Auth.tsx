import { useState, type FormEvent } from "react";
import {
  Activity,
  Boxes,
  Brain,
  GitBranch,
  LoaderCircle,
  LockKeyhole,
  MessageSquare,
  ShieldCheck,
  UnlockKeyhole,
} from "lucide-react";
import { api } from "../api";

export function Auth({
  initialized,
  onSuccess,
}: {
  initialized: boolean;
  onSuccess: () => void;
}) {
  const [password, setPassword] = useState("");
  const [repeat, setRepeat] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError("");
    if (!initialized && password !== repeat) {
      setError("两次口令不一致");
      return;
    }
    setBusy(true);
    try {
      await api(initialized ? "/auth/login" : "/auth/setup", { password });
      setPassword("");
      setRepeat("");
      onSuccess();
    } catch (reason) {
      setError((reason as Error).message);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="auth-screen">
      <div className="auth-top">
        <div className="brand">
          <div className="brand-symbol">
            <GitBranch size={23} />
          </div>
          <span>prm.</span>
        </div>
        <span className="local-badge">
          <ShieldCheck size={15} />
          本机工作区
        </span>
      </div>
      <div className="auth-content">
        <div className="auth-worlds">
          <Activity />
          <span />
          <Brain />
          <span />
          <MessageSquare />
          <span />
          <Boxes />
        </div>
        <p className="eyebrow">PERSONAL RESOURCE · RELATION MANAGEMENT</p>
        <h1>{initialized ? "回到你的资源空间" : "建立你的资源空间"}</h1>
        <p className="auth-subtitle">自我 · 思维 · 数字 · 物理</p>
        <form onSubmit={submit}>
          <label className="field">
            {initialized ? "账户口令" : "设置账户口令"}
            <input
              type="password"
              autoComplete={initialized ? "current-password" : "new-password"}
              autoFocus
              required
              minLength={initialized ? 1 : 12}
              maxLength={128}
              placeholder={initialized ? "输入口令" : "至少 12 个字符"}
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </label>
          {!initialized && (
            <>
              <label className="field">
                再次输入口令
                <input
                  type="password"
                  autoComplete="new-password"
                  required
                  value={repeat}
                  onChange={(event) => setRepeat(event.target.value)}
                />
              </label>
              <div className="privacy-note">
                <LockKeyhole size={17} />
                <span>
                  此口令也用于加密思维库。请妥善保管，遗失后无法恢复其中内容。
                </span>
              </div>
            </>
          )}
          {error && (
            <p className="form-error" role="alert">
              {error}
            </p>
          )}
          <button className="button primary auth-submit" disabled={busy}>
            {busy ? (
              <LoaderCircle className="spin" size={18} />
            ) : (
              <UnlockKeyhole size={18} />
            )}
            {initialized ? "进入工作区" : "创建本地账户"}
          </button>
        </form>
        <div className="auth-bottom">
          <span className="mini-dot" />
          本地存储 <span>·</span> 思维库加密 <span>·</span> 离线运行
        </div>
      </div>
      <div className="auth-footer">
        PRM / 个人资源与关系本体 <span>0.4</span>
      </div>
    </div>
  );
}
