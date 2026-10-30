import { useCallback, useEffect, useRef, useState } from "react";
import {
  Activity,
  Boxes,
  Brain,
  ChevronRight,
  CircleCheck,
  CircleHelp,
  ClipboardCheck,
  Download,
  GitBranch,
  History,
  LayoutDashboard,
  LoaderCircle,
  LockKeyhole,
  LogOut,
  Menu,
  MessageSquare,
  Network,
  Plus,
  Search,
  ShieldCheck,
  Sparkles,
  UsersRound,
  X,
} from "lucide-react";
import {
  api,
  ApiError,
  prop,
  SCENES,
  sceneLabel,
  today,
  type Dialog,
  type Entity,
  type Vault,
  type Workspace,
} from "./api";
import type { Page, PageContext } from "./context";
import { LocaleSelect, t, useT } from "./i18n";
import { Editor } from "./Editor";
import { PrpDialog, isPrpDialog } from "./dialogs/PrpDialogs";
import { RelationDialog, isRelationDialog } from "./dialogs/RelationDialogs";
import { Auth } from "./pages/Auth";
import { AuditLog } from "./pages/AuditLog";
import { Dashboard } from "./pages/Dashboard";
import { Decisions } from "./pages/Decisions";
import { Digital } from "./pages/Digital";
import { Mental } from "./pages/Mental";
import { Ontology } from "./pages/Ontology";
import { Physical } from "./pages/Physical";
import { Relations } from "./pages/Relations";
import { SelfPage } from "./pages/SelfPage";

// Labels stay in Simplified Chinese here and are translated at render time.
const navigation = [
  { id: "dashboard", label: "工作台", icon: LayoutDashboard },
  { id: "self", label: "自我本体", icon: Activity },
  { id: "relations", label: "关系网络", icon: UsersRound },
  { id: "mental", label: "思维体系", icon: Brain },
  { id: "digital", label: "数字空间", icon: MessageSquare },
  { id: "physical", label: "物理资源", icon: Boxes },
  { id: "decisions", label: "决策中心", icon: ClipboardCheck },
  { id: "ontology", label: "本体关系", icon: Network },
  { id: "audit", label: "活动记录", icon: History },
] as const;
const eyebrows: Record<Page, string> = {
  dashboard: "PERSONAL RESOURCE · RELATION MANAGEMENT",
  self: "SELF ONTOLOGY",
  relations: "RELATIONS WITH OTHERS",
  mental: "MENTAL WORLD",
  digital: "DIGITAL WORLD",
  physical: "PHYSICAL WORLD · PRP",
  decisions: "DECISIONS",
  ontology: "ONTOLOGY",
  audit: "ACTIVITY",
};

export default function App() {
  useT(); // Subscribe the whole tree to language switches.
  const [auth, setAuth] = useState<{
    initialized: boolean;
    authenticated: boolean;
  } | null>(null);
  const [workspace, setWorkspace] = useState<Workspace | null>(null);
  const [page, setPage] = useState<Page>("dashboard");
  const [dialog, setDialog] = useState<Dialog>(null);
  const [vault, setVault] = useState<Vault | null>(null);
  const [search, setSearch] = useState("");
  const [scene, setScene] = useState("");
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [sidebar, setSidebar] = useState(false);
  const mounted = useRef(true);
  const privacyGeneration = useRef(0);
  const notify = (text: string) => {
    setNotice(text);
  };
  const fail = useCallback((reason: unknown) => {
    if (reason instanceof ApiError && reason.status === 401) {
      setAuth((a) => a && { ...a, authenticated: false });
      setWorkspace(null);
      setVault(null);
      setDialog(null);
    }
    setError(reason instanceof Error ? reason.message : t("操作未完成"));
  }, []);
  const refresh = useCallback(async () => {
    const data = await api<Workspace>("/workspace");
    if (mounted.current) setWorkspace(data);
  }, []);
  const lockVault = useCallback(() => {
    privacyGeneration.current += 1;
    setVault(null);
    setDialog(null);
    void api("/vault/lock", {}).catch(() => {});
  }, []);
  useEffect(() => {
    mounted.current = true;
    api<{ initialized: boolean; authenticated: boolean }>("/auth/status")
      .then(setAuth)
      .catch(fail);
    return () => {
      mounted.current = false;
    };
  }, [fail]);
  useEffect(() => {
    if (auth?.authenticated) void refresh().catch(fail);
  }, [auth?.authenticated, refresh, fail]);
  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(""), 4000);
    return () => clearTimeout(timer);
  }, [notice]);
  const isUnlocked = vault !== null;
  useEffect(() => {
    if (!isUnlocked) return;
    const timer = setTimeout(lockVault, 300_000);
    const hidden = () => {
      if (document.visibilityState === "hidden") lockVault();
    };
    document.addEventListener("visibilitychange", hidden);
    window.addEventListener("pagehide", lockVault);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", hidden);
      window.removeEventListener("pagehide", lockVault);
    };
  }, [isUnlocked, lockVault]);
  const run = async (work: () => Promise<unknown>, message: string) => {
    setBusy(true);
    setError("");
    try {
      await work();
      await refresh();
      notify(message);
      return true;
    } catch (reason) {
      fail(reason);
      return false;
    } finally {
      setBusy(false);
    }
  };
  const go = (next: Page) => {
    if (page === "mental") lockVault();
    setPage(next);
    setSearch("");
    setSidebar(false);
    setError("");
    // Agent imports and executor results arrive in the background.
    void refresh().catch(fail);
  };
  const logout = async () => {
    await api("/auth/logout", {});
    privacyGeneration.current += 1;
    setVault(null);
    setWorkspace(null);
    setDialog(null);
    setAuth({ initialized: true, authenticated: false });
  };
  if (!auth)
    return (
      <div className="loading">
        <LoaderCircle className="spin" />
        <span>{error || t("正在连接本机工作区")}</span>
        {error && (
          <button
            className="button secondary"
            onClick={() => location.reload()}
          >
            {t("重试")}
          </button>
        )}
      </div>
    );
  if (!auth.authenticated)
    return (
      <Auth
        initialized={auth.initialized}
        onSuccess={() => {
          setError("");
          setAuth({ initialized: true, authenticated: true });
        }}
      />
    );
  if (!workspace)
    return (
      <div className="loading">
        <LoaderCircle className="spin" />
        <span>{error || t("正在读取工作区")}</span>
        {error && (
          <button
            className="button secondary"
            onClick={() => void refresh().catch(fail)}
          >
            {t("重试")}
          </button>
        )}
      </div>
    );
  const entities = workspace.entities;
  const byKind = (kind: string) =>
    entities.filter((entity) => entity.kind === kind);
  const inScene = (entity: Entity) =>
    !scene || (entity.scenes || []).includes(scene);
  const filtered = (list: Entity[]) =>
    list.filter(
      (entity) =>
        inScene(entity) &&
        `${entity.title} ${prop(entity, "body")}`
          .toLowerCase()
          .includes(search.toLowerCase()),
    );
  const proposals = byKind("proposal").filter(
    (entity) => entity.status === "pending",
  );
  const current = navigation.find((item) => item.id === page)!;
  const ctx: PageContext = {
    workspace,
    entities,
    byKind,
    filter: filtered,
    inScene,
    scene,
    setScene,
    search,
    onDialog: setDialog,
    run,
    busy,
    go,
    fail,
    notify,
    refresh,
  };
  const exportData = async () => {
    const data = await api("/export");
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(data, null, 2)], { type: "application/json" }),
    );
    const link = document.createElement("a");
    link.href = url;
    link.download = `prm-operations-${today()}.json`;
    link.click();
    URL.revokeObjectURL(url);
  };
  const closeDialog = () => setDialog(null);
  const saved = async (message: string) => {
    await refresh();
    setDialog(null);
    notify(message);
  };
  return (
    <div className="app-shell">
      {sidebar && (
        <button
          className="sidebar-shade"
          aria-label={t("关闭导航")}
          onClick={() => setSidebar(false)}
        />
      )}
      <aside className={`sidebar ${sidebar ? "open" : ""}`}>
        <div className="brand">
          <div className="brand-symbol">
            <GitBranch size={23} />
          </div>
          <span>
            prm<span className="brand-dot">.</span>
          </span>
          <span className="local-edition">PERSONAL</span>
        </div>
        <div className="workspace-label">
          <span className="avatar">{t("我")}</span>
          <div>
            <strong>{t("我的资源与关系")}</strong>
            <small>{t("个人本体工作区")}</small>
          </div>
          <ShieldCheck size={17} />
        </div>
        <div className="nav-caption">{t("工作空间")}</div>
        <nav aria-label={t("主导航")}>
          {navigation.map((item) => (
            <button
              key={item.id}
              className={page === item.id ? "active" : ""}
              onClick={() => go(item.id)}
            >
              <item.icon size={18} />
              <span>{t(item.label)}</span>
              {item.id === "decisions" && proposals.length > 0 && (
                <span className="nav-count">{proposals.length}</span>
              )}
              {item.id === "mental" && (
                <LockKeyhole className="nav-lock" size={13} />
              )}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="local-state">
            <i />
            <span>{t("本机运行")}</span>
            <span className="mono">v0.5</span>
          </div>
          <button
            className="sign-out"
            onClick={() => void logout().catch(fail)}
          >
            <LogOut size={16} />
            {t("退出账户")}
          </button>
        </div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          <div className="breadcrumb">
            <button
              className="icon-button mobile-menu"
              aria-label={t("打开导航")}
              onClick={() => setSidebar(true)}
            >
              <Menu size={20} />
            </button>
            <span>{t("个人资源与关系")}</span>
            <ChevronRight size={14} />
            <strong>{t(current.label)}</strong>
          </div>
          <div className="topbar-right">
            <LocaleSelect />
            <select
              className="scene-select"
              aria-label={t("人生场景")}
              value={scene}
              onChange={(event) => setScene(event.target.value)}
            >
              <option value="">{t("全部场景")}</option>
              {SCENES.map((id) => (
                <option key={id} value={id}>
                  {sceneLabel(id)}
                </option>
              ))}
            </select>
            <span className="local-badge">
              <ShieldCheck size={14} />
              {t("本地数据")}
            </span>
            <span className="avatar small">{t("我")}</span>
          </div>
        </header>
        <main>
          <div className="page-heading">
            <div>
              <div className="eyebrow">{eyebrows[page]}</div>
              <h1>{t(current.label)}</h1>
            </div>
            <div className="page-actions">
              {page !== "mental" && page !== "audit" && (
                <label className="search">
                  <Search size={16} />
                  <input
                    aria-label={t("搜索当前视图")}
                    placeholder={t("搜索当前视图")}
                    value={search}
                    onChange={(event) => setSearch(event.target.value)}
                  />
                </label>
              )}
              {page === "dashboard" && (
                <button
                  className="button primary"
                  onClick={() => setDialog({ type: "task" })}
                >
                  <Plus size={17} />
                  {t("新建任务")}
                </button>
              )}
              {page === "relations" && (
                <button
                  className="button primary"
                  onClick={() => setDialog({ type: "relationship" })}
                >
                  <Plus size={17} />
                  {t("新建关系")}
                </button>
              )}
              {page === "mental" && vault && (
                <>
                  <button
                    className="icon-button outlined"
                    aria-label={t("锁定思维库")}
                    title={t("锁定思维库")}
                    onClick={lockVault}
                  >
                    <LockKeyhole size={18} />
                  </button>
                  <button
                    className="button primary"
                    onClick={() => setDialog({ type: "core" })}
                  >
                    <Plus size={17} />
                    {t("新建条目")}
                  </button>
                </>
              )}
              {page === "decisions" && (
                <button
                  className="button primary"
                  disabled={busy}
                  onClick={() =>
                    void run(
                      () => api("/analyze", {}),
                      t("分析完成，建议已更新"),
                    )
                  }
                >
                  <Sparkles size={17} />
                  {t("分析资源")}
                </button>
              )}
              {page === "audit" && (
                <button
                  className="button secondary"
                  onClick={() => void exportData().catch(fail)}
                >
                  <Download size={16} />
                  {t("导出业务数据")}
                </button>
              )}
            </div>
          </div>
          {scene && page !== "mental" && page !== "audit" && (
            <div className="scene-banner">
              <span>
                {t("当前场景：{scene}，列表只显示标注了这一场景的记录", {
                  scene: sceneLabel(scene),
                })}
              </span>
              <button className="text-button" onClick={() => setScene("")}>
                {t("显示全部场景")}
              </button>
            </div>
          )}
          {error && (
            <div className="error-banner" role="alert">
              <CircleHelp size={17} />
              <span>{error}</span>
              <button
                className="icon-button"
                aria-label={t("关闭错误")}
                onClick={() => setError("")}
              >
                <X size={16} />
              </button>
            </div>
          )}
          {page === "dashboard" && <Dashboard ctx={ctx} />}
          {page === "self" && <SelfPage ctx={ctx} />}
          {page === "relations" && <Relations ctx={ctx} />}
          {page === "mental" && <Mental vault={vault} onDialog={setDialog} />}
          {page === "digital" && <Digital ctx={ctx} />}
          {page === "physical" && <Physical ctx={ctx} />}
          {page === "decisions" && <Decisions ctx={ctx} />}
          {page === "ontology" && <Ontology ctx={ctx} />}
          {page === "audit" && <AuditLog workspace={workspace} />}
        </main>
        <footer className="page-footer">
          <span>{t("由自己的判断出发")}</span>
          <span>
            <span className="mini-dot" />
            {t("离线规则引擎")} <span className="footer-divider">/</span>{" "}
            {t("数据保存在本机")}
          </span>
        </footer>
      </div>
      {notice && (
        <div className="toast" role="status">
          <CircleCheck size={18} />
          {notice}
        </div>
      )}
      {dialog && isPrpDialog(dialog.type) && (
        <PrpDialog
          key={`${dialog.type}-${dialog.entity?.id || ""}`}
          dialog={dialog}
          workspace={workspace}
          onClose={closeDialog}
          onSaved={saved}
        />
      )}
      {dialog && isRelationDialog(dialog.type) && (
        <RelationDialog
          key={`${dialog.type}-${dialog.entity?.id || ""}`}
          dialog={dialog}
          workspace={workspace}
          onClose={closeDialog}
          onSaved={saved}
        />
      )}
      {dialog && !isPrpDialog(dialog.type) && !isRelationDialog(dialog.type) && (
        <Editor
          key={`${dialog.type}-${dialog.entity?.id || dialog.core?.id || ""}`}
          dialog={dialog}
          workspace={workspace}
          vault={vault}
          onClose={closeDialog}
          onSaved={async (newVault, message) => {
            const generation = privacyGeneration.current;
            await refresh();
            if (generation === privacyGeneration.current) {
              if (newVault) setVault(newVault);
              setDialog(null);
              notify(message);
            }
          }}
          onUnlocked={(newVault) => {
            if (document.visibilityState === "visible") {
              setVault(newVault);
              setDialog(null);
            } else lockVault();
          }}
        />
      )}
    </div>
  );
}
