import { useEffect, useState, type FormEvent } from "react";
import { api, type CoreItem, type Vault } from "./api";
import { devicePublicKey, signatureHeaders } from "./security";
interface Grant { id: string; label: string; expires_at: number; revoked: boolean }
export function Permissions({ vault }: { vault: Vault }) {
  const [grants, setGrants] = useState<Grant[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  const [publicKey, setPublicKey] = useState("");
  const [label, setLabel] = useState("");
  const [password, setPassword] = useState("");
  const [minutes, setMinutes] = useState(60);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const refresh = () => api<{ grants: Grant[] }>("/vault/grants").then(v => setGrants(v.grants));
  useEffect(() => { void refresh().catch(e => setError(e.message)); }, []);
  const create = async (event: FormEvent) => {
    event.preventDefault(); setBusy(true); setError("");
    try {
      await api("/vault/grants", { password, public_key: publicKey.trim(), label, item_ids: selected, expires_in_seconds: minutes * 60 });
      setPassword(""); setSelected([]); await refresh();
    } catch (e) { setError((e as Error).message); }
    finally { setBusy(false); }
  };
  const revoke = async (id: string) => {
    setBusy(true); setError("");
    try { await api("/vault/grants/revoke", { id }); await refresh(); }
    catch (e) { setError((e as Error).message); } finally { setBusy(false); }
  };
  return <details className="permissions-panel"><summary>核心精神 · 只读授权设置</summary>
    <p>只有你可以修改。接收者使用自己的设备公钥，只能读取你勾选的条目。锁定思维库、退出登录或授权到期后，读取立即暂停。授权不包含历史版本。</p>
    <p>接收者先打开 <a href="/shared" target="_blank" rel="noreferrer">只读访问页面</a>，将设备公钥交给你。</p>
    <form onSubmit={create}>
      <label className="field">接收者名称<input required maxLength={120} value={label} onChange={e => setLabel(e.target.value)} /></label>
      <label className="field">接收者设备公钥<input required value={publicKey} onChange={e => setPublicKey(e.target.value)} placeholder="Ed25519 公钥" /></label>
      <label className="field">有效期（分钟）<input type="number" min={1} max={1440} required value={minutes} onChange={e => setMinutes(Number(e.target.value))} /></label>
      <fieldset><legend>允许读取的条目</legend>{vault.items.map(item => <label className="grant-item" key={item.id}><input type="checkbox" checked={selected.includes(item.id)} onChange={e => setSelected(e.target.checked ? [...selected, item.id] : selected.filter(id => id !== item.id))} />{item.title}</label>)}</fieldset>
      <label className="field">本人确认口令<input type="password" required autoComplete="current-password" value={password} onChange={e => setPassword(e.target.value)} /></label>
      <button className="button primary" disabled={busy || !selected.length}>确认只读授权</button>
    </form>
    {error && <p role="alert">{error}</p>}
    {grants.map(grant => <article key={grant.id} className="grant-row"><strong>{grant.label}</strong><span>{grant.revoked ? "已撤销" : grant.expires_at * 1000 <= Date.now() ? "已到期" : "只读"}</span><span>到期：{new Date(grant.expires_at * 1000).toLocaleString()}</span><code>{grant.id}</code>{!grant.revoked && <button className="button secondary" disabled={busy} onClick={() => void revoke(grant.id)}>撤销授权</button>}</article>)}
    <p className="muted">将授权编号交给接收者。撤销会阻止后续读取，接收者已经保存的内容无法收回。</p>
  </details>;
}
export function SharedVault() {
  const [publicKey, setPublicKey] = useState("");
  const [grant, setGrant] = useState("");
  const [items, setItems] = useState<CoreItem[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => { void devicePublicKey().then(setPublicKey).catch(e => setError(e.message)); }, []);
  useEffect(() => {
    const clear = () => setItems([]);
    window.addEventListener("blur", clear);
    const timer = window.setInterval(clear, 60000);
    return () => { window.removeEventListener("blur", clear); window.clearInterval(timer); };
  }, []);
  const read = async (event: FormEvent) => {
    event.preventDefault(); setError(""); setItems([]); setBusy(true);
    try {
      const path = "/api/shared/vault";
      const response = await fetch(path, { headers: { ...await signatureHeaders(path, "GET", ""), "X-PRM-Grant": grant.trim() }, credentials: "omit", cache: "no-store" });
      const result = await response.json();
      if (!response.ok) throw new Error(response.status === 423 ? "本人尚未解锁思维库，请稍后重试" : "授权无效、已到期或公钥不匹配");
      setItems(result.items);
    } catch (e) { setError((e as Error).message); } finally { setBusy(false); }
  };
  return <main className="shared-vault"><h1>核心精神 · 只读访问</h1><p>将本设备公钥交给所有者，获得授权编号后读取。私钥由浏览器保管。</p><label className="field">本设备公钥<textarea readOnly value={publicKey} /></label><form onSubmit={read}><label className="field">授权编号<input required value={grant} onChange={e => setGrant(e.target.value)} /></label><button className="button primary" disabled={busy}>读取授权内容</button></form>{error && <p role="alert">{error}</p>}{items.map(item => <article className="core-entry" key={item.id}><h2>{item.title}</h2><p className="preserve-text">{item.body}</p></article>)}</main>;
}
