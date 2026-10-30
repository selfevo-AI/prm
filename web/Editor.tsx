import { useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { ArrowUpRight, Boxes, ListTodo, LockKeyhole, Network, ShieldCheck } from "lucide-react";
import {
  api,
  cents,
  label,
  money,
  num,
  prop,
  time,
  today,
  type Dialog,
  type Entity,
  type Vault,
  type Workspace,
} from "./api";
import {
  Modal,
  PasswordConfirm,
  SceneInput,
  Scenes,
  StatusBadge,
  ValueInput,
  Values,
} from "./components";
import { t } from "./i18n";

const detailFields: Record<string, [string, string][]> = {
  resource: [
    ["location", "位置"],
    ["sku", "SKU / 物料编码"],
    ["category", "分类"],
    ["supplier", "供应商"],
    ["reorder_qty", "建议补货量"],
    ["lead_time_days", "交货周期（天）"],
    ["warranty_until", "保修截止"],
    ["serial_no", "序列号"],
  ],
  person: [["notes", "备注"]],
  ai_agent: [
    ["platform", "接入平台"],
    ["protocol", "接入方式"],
    ["notes", "备注"],
  ],
  supplier: [
    ["contact", "联系方式"],
    ["tax_id", "税号"],
    ["notes", "备注"],
  ],
  location: [
    ["code", "编码"],
    ["notes", "备注"],
  ],
  relationship: [
    ["platform", "接入平台"],
    ["closeness", "亲密度（IOS）"],
    ["satisfaction", "满意度（RAS）"],
    ["contact_interval_days", "联系节奏（天）"],
    ["last_contact_at", "上次联系"],
    ["notes", "备注"],
  ],
  agent_import: [
    ["agent_name", "来源 Agent"],
    ["format", "格式"],
    ["item_count", "项目数"],
    ["conversation", "会话引用"],
  ],
};

export function Editor({
  dialog,
  workspace,
  vault,
  onClose,
  onSaved,
  onUnlocked,
}: {
  dialog: NonNullable<Dialog>;
  workspace: Workspace;
  vault: Vault | null;
  onClose: () => void;
  onSaved: (vault: Vault | null, message: string) => Promise<void>;
  onUnlocked: (vault: Vault) => void;
}) {
  const { type, entity, core } = dialog;
  const action = entity?.kind === "proposal" ? prop(entity, "action") || "create_task" : "";
  const [title, setTitle] = useState(
    type === "publish" ? "" : core?.title || entity?.title || "",
  );
  const [body, setBody] = useState(type === "publish" ? "" : core?.body || "");
  const [kind, setKind] = useState(core?.kind || "principle");
  const [values, setValues] = useState<string[]>(core?.values || []);
  const [scenes, setScenes] = useState<string[]>([]);
  const [password, setPassword] = useState("");
  const [confirmed, setConfirmed] = useState(false);
  const [unit, setUnit] = useState("个");
  const [location, setLocation] = useState("");
  const [locationId, setLocationId] = useState("");
  const [minimum, setMinimum] = useState("0");
  const [reorderQty, setReorderQty] = useState("0");
  const [leadTime, setLeadTime] = useState("0");
  const [itemType, setItemType] = useState("consumable");
  const [warranty, setWarranty] = useState("");
  const [serial, setSerial] = useState("");
  const [sku, setSku] = useState("");
  const [resourceCategory, setResourceCategory] = useState("");
  const [supplierId, setSupplierId] = useState("");
  const [unitCost, setUnitCost] = useState("");
  const [quantity, setQuantity] = useState("1");
  const [direction, setDirection] = useState("expense");
  const [amount, setAmount] = useState("");
  const [category, setCategory] = useState("日常");
  const [date, setDate] = useState(today());
  const [directive, setDirective] = useState("");
  const [subject, setSubject] = useState("");
  const [predicate, setPredicate] = useState("references");
  const [object, setObject] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  const of = (kind: string) => workspace.entities.filter((e) => e.kind === kind);
  const titles: Record<string, string> = {
    task: "新建任务",
    resource: "登记资源",
    cash: "记一笔收支",
    note: "新建知识条目",
    channel: "新建会话",
    core: core?.id ? "编辑思维条目" : "新建思维条目",
    unlock: "本人解锁",
    review: "确认我的判断",
    publish: "发布执行方向",
    history: "版本记录",
    approve: "审阅并采纳建议",
    movement: dialog.delta === 1 ? "资源入库" : "资源出库",
    relation: "建立本体关系",
    detail: entity?.title || "详情",
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError("");
    setBusy(true);
    try {
      let updated: Vault | null = null;
      const confirmation = {
        password,
        confirmed,
        expected_version: vault?.version || 0,
      };
      if (type === "unlock") {
        const unlocked = await api<Vault>("/vault/unlock", { password });
        if (alive.current) onUnlocked(unlocked);
        else await api("/vault/lock", {});
        return;
      }
      if (type === "core")
        updated = await api<Vault>("/vault/entries", {
          ...confirmation,
          id: core?.id || null,
          kind,
          title,
          body,
          values,
        });
      else if (type === "review")
        updated = await api<Vault>("/vault/review", {
          ...confirmation,
          id: core?.id,
        });
      else if (type === "publish")
        await api("/vault/publish", {
          ...confirmation,
          id: core?.id,
          title,
          brief: body,
          scenes,
        });
      else if (type === "movement")
        await api(`/resources/${entity!.id}/movement`, {
          delta: Number(quantity) * dialog.delta!,
          reason: body,
          expected_version: entity!.version,
        });
      else if (type === "cash")
        await api("/cash", {
          title,
          direction,
          amount_cents: cents(amount),
          category,
          date,
          values,
          scenes,
        });
      else if (type === "detail" && entity?.kind === "directive")
        await api(`/directives/${entity.id}/revoke`, {
          expected_version: entity.version,
        });
      else if (type === "approve")
        await api(`/proposals/${entity!.id}/decide`, {
          decision: "approve",
          expected_version: entity!.version,
          title: action === "create_relation" ? "" : title,
          directive_id: action === "create_task" ? directive : "",
        });
      else if (type === "relation")
        await api("/relations", { subject, predicate, object });
      else
        await api("/entities", {
          kind: type,
          title,
          values,
          scenes,
          properties:
            type === "resource"
              ? {
                  unit,
                  location,
                  location_id: locationId,
                  minimum: Number(minimum),
                  reorder_qty: Number(reorderQty),
                  lead_time_days: Number(leadTime),
                  item_type: itemType,
                  warranty_until: itemType === "asset" ? warranty : "",
                  serial_no: itemType === "asset" ? serial : "",
                  sku,
                  category: resourceCategory,
                  supplier_id: supplierId,
                  unit_cost_cents: unitCost.trim() ? cents(unitCost) : 0,
                }
              : type === "task"
                ? { body, due_date: date, directive_id: directive }
                : type === "note"
                  ? { body }
                  : {},
        });
      setPassword("");
      if (alive.current)
        await onSaved(
          updated,
          type === "publish"
            ? t("执行方向已发布")
            : type === "approve"
              ? action === "create_task"
                ? t("已采纳，任务已创建")
                : action === "create_entity"
                  ? t("已采纳，已写入本体")
                  : t("已采纳，关系已建立")
              : t("已保存"),
        );
    } catch (reason) {
      if (alive.current) setError((reason as Error).message);
    } finally {
      if (alive.current) setBusy(false);
    }
  };
  const field = (
    name: string,
    value: string,
    setter: (v: string) => void,
    inputType = "text",
    maxLength = 120,
    required = true,
  ) => (
    <label className="field">
      {t(name)}
      <input
        required={required}
        type={inputType}
        maxLength={maxLength}
        min={inputType === "number" ? 0 : undefined}
        max={inputType === "number" ? 1_000_000_000 : undefined}
        step={inputType === "number" ? 1 : undefined}
        value={value}
        onChange={(event) => setter(event.target.value)}
      />
    </label>
  );
  const textArea = (name: string, maxLength = 4000) => (
    <label className="field">
      {t(name)}
      <textarea
        required
        rows={5}
        maxLength={maxLength}
        value={body}
        onChange={(event) => setBody(event.target.value)}
      />
    </label>
  );
  const select = (
    name: string,
    value: string,
    setter: (v: string) => void,
    options: { value: string; label: string }[],
    empty?: string,
  ) => (
    <label className="field">
      {t(name)}
      <select aria-label={t(name)} value={value} onChange={(event) => setter(event.target.value)}>
        {empty !== undefined && <option value="">{t(empty)}</option>}
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {t(o.label)}
          </option>
        ))}
      </select>
    </label>
  );
  const directionSelect = select(
    "关联执行方向",
    directive,
    setDirective,
    of("directive")
      .filter((e) => e.status === "active")
      .map((e) => ({ value: e.id, label: e.title })),
    "暂不关联",
  );
  const owned = workspace.ontology.predicates.filter((p) => p.owner_settable);
  const current = owned.find((p) => p.id === predicate);
  const subjectKind = workspace.entities.find((e) => e.id === subject)?.kind;
  const objectKinds = (current?.signatures || [])
    .filter((s) => !subjectKind || s.domain.includes(subjectKind))
    .flatMap((s) => s.range);
  const canRevoke =
    type === "detail" &&
    entity?.kind === "directive" &&
    entity.status === "active";
  const readonly = ["detail", "history"].includes(type) && !canRevoke;
  const spec = (entity?.properties.entity || {}) as { kind?: string; properties?: Record<string, unknown> };
  const relationSpec = (entity?.properties.relation || {}) as Record<string, string>;
  return (
    <Modal
      title={t(titles[type])}
      onClose={onClose}
      onSubmit={readonly ? undefined : submit}
      busy={busy}
      error={error}
      wide={type === "history" || type === "resource"}
      submit={
        canRevoke
          ? t("撤回授权")
          : type === "unlock"
            ? t("解锁")
            : type === "approve"
              ? action === "create_task"
                ? t("确认创建任务")
                : action === "create_entity"
                  ? t("确认纳入")
                  : t("确认建立关系")
              : type === "publish"
                ? t("确认发布")
                : ["core", "review"].includes(type)
                  ? t("本人确认并保存")
                  : t("保存")
      }
    >
      {type === "unlock" && (
        <>
          <div className="privacy-note">
            <LockKeyhole size={19} />
            <span>
              {t("仅本人可修改；有效只读授权在解锁期间可读取指定内容。解锁有效期 5 分钟。")}
            </span>
          </div>
          <label className="field">
            {t("账户口令")}
            <input
              required
              autoFocus
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </label>
        </>
      )}
      {(["task", "resource", "cash", "note", "channel", "core", "publish"].includes(type) ||
        (type === "approve" && action !== "create_relation")) &&
        field(
          type === "resource"
            ? "资源名称"
            : type === "channel"
              ? "会话名称"
              : "标题",
          title,
          setTitle,
        )}
      {type === "core" &&
        select(
          "条目类型",
          kind,
          setKind,
          ["principle", "goal", "reflection"].map((k) => ({ value: k, label: label(k) })),
        )}
      {["task", "note", "core"].includes(type) &&
        textArea(
          type === "core" ? "我的判断与记录" : "内容",
          type === "task" ? 4000 : 12000,
        )}
      {type === "publish" && (
        <>
          <div className="privacy-note">
            <ArrowUpRight size={18} />
            <span>
              {t("发布后的标题、内容和评价维度将进入业务数据，作为核心原则提供给本地模型，并允许助理读取。")}
            </span>
          </div>
          {textArea("允许助理读取的执行要求")}
          <SceneInput scenes={scenes} onChange={setScenes} legend="适用场景（不选表示全部场景）" />
        </>
      )}
      {type === "task" && (
        <div className="form-grid">
          {field("计划日期", date, setDate, "date")}
          {directionSelect}
        </div>
      )}
      {type === "resource" && (
        <>
          <div className="form-grid">
            {select("物料类型", itemType, setItemType, [
              { value: "consumable", label: "消耗品" },
              { value: "asset", label: "资产" },
            ])}
            {field("单位", unit, setUnit, "text", 20)}
          </div>
          <div className="form-grid">
            {select(
              "库位（可选）",
              locationId,
              (value) => {
                setLocationId(value);
                const place = of("location").find((l) => l.id === value);
                if (place) setLocation(place.title);
              },
              of("location").map((l) => ({ value: l.id, label: l.title })),
              "不指定",
            )}
            {field("存放位置", location, setLocation)}
          </div>
          <div className="form-grid">
            {field("最低库存", minimum, setMinimum, "number")}
            {field("建议补货量", reorderQty, setReorderQty, "number")}
          </div>
          <div className="form-grid">
            {field("SKU / 物料编码", sku, setSku, "text", 80, false)}
            {field("分类", resourceCategory, setResourceCategory, "text", 80, false)}
          </div>
          <div className="form-grid">
            {select(
              "供应商（可选）",
              supplierId,
              setSupplierId,
              of("supplier").map((s) => ({ value: s.id, label: s.title })),
              "不指定",
            )}
            {field("交货周期（天）", leadTime, setLeadTime, "number")}
          </div>
          <div className="form-grid">
            {field("单位成本（元）", unitCost, setUnitCost, "text", 20, false)}
            {itemType === "asset" && field("保修截止", warranty, setWarranty, "date", 10, false)}
          </div>
          {itemType === "asset" && field("序列号", serial, setSerial, "text", 80, false)}
          <p className="muted small-text">
            {t("期初数量为 0，登记后可办理入库或采购收货。")}
          </p>
        </>
      )}
      {type === "movement" && (
        <>
          <div className="movement-summary">
            <Boxes size={22} />
            <div>
              <strong>{entity!.title}</strong>
              <p>
                {t("当前库存 {quantity} {unit}", {
                  quantity: num(entity!, "quantity"),
                  unit: t(prop(entity!, "unit")),
                })}
              </p>
            </div>
          </div>
          {field(
            t("数量（{unit}）", { unit: t(prop(entity!, "unit")) }),
            quantity,
            setQuantity,
            "number",
          )}
          {textArea("变动原因", 300)}
        </>
      )}
      {type === "cash" && (
        <>
          <div className="segmented">
            <button
              type="button"
              className={direction === "expense" ? "active" : ""}
              onClick={() => setDirection("expense")}
            >
              {t("支出")}
            </button>
            <button
              type="button"
              className={direction === "income" ? "active" : ""}
              onClick={() => setDirection("income")}
            >
              {t("收入")}
            </button>
          </div>
          <div className="form-grid">
            <label className="field">
              {t("金额（元）")}
              <input
                required
                inputMode="decimal"
                placeholder="0.00"
                value={amount}
                onChange={(event) => setAmount(event.target.value)}
              />
            </label>
            {field("日期", date, setDate, "date")}
          </div>
          {field("分类", category, setCategory, "text", 60)}
        </>
      )}
      {["task", "resource", "cash", "note", "core"].includes(type) && (
        <ValueInput values={values} onChange={setValues} />
      )}
      {["task", "resource", "cash", "note"].includes(type) && (
        <SceneInput scenes={scenes} onChange={setScenes} legend="人生场景（不选则按类型默认）" />
      )}
      {type === "review" && (
        <div className="review-text">
          <h3>{core!.title}</h3>
          <p className="preserve-text">{core!.body}</p>
          <Values values={core!.values} />
        </div>
      )}
      {["core", "review", "publish"].includes(type) && (
        <PasswordConfirm
          password={password}
          setPassword={setPassword}
          confirmed={confirmed}
          setConfirmed={setConfirmed}
          publish={type === "publish"}
        />
      )}
      {type === "approve" && (
        <>
          <div className="review-text">
            {action === "create_entity" && (
              <p>
                <span className="tag">
                  {t("纳入{kind}", { kind: spec.kind ? label(spec.kind) : t("实体") })}
                </span>{" "}
                {Object.entries(spec.properties || {})
                  .filter(([, v]) => String(v ?? "") !== "")
                  .map(([k, v]) => `${k}：${String(v)}`)
                  .join("；")}
              </p>
            )}
            {action === "create_relation" && (
              <p className="relation-summary">
                <Network size={15} /> {relationSpec.subject_title}{" "}
                <strong>{label(relationSpec.predicate)}</strong>{" "}
                {relationSpec.object_title}
              </p>
            )}
            <p className="preserve-text">{prop(entity!, "rationale")}</p>
            <p className="muted small-text">
              {t("依据：")}
              {workspace.entities.find((e) => e.id === prop(entity!, "source_id"))?.title}
            </p>
          </div>
          {action === "create_task" && directionSelect}
          <div className="privacy-note">
            <ListTodo size={18} />
            <span>
              {action === "create_task"
                ? t("确认后将创建一项待开始任务。")
                : action === "create_entity"
                  ? t("确认后将按本体约束重新校验并写入；之后可在对应页面继续维护。")
                  : t("关系两端须已存在；引用同一导入中的其他候选时，请先采纳那一条。")}
            </span>
          </div>
        </>
      )}
      {type === "relation" && (
        <>
          {select(
            "关系",
            predicate,
            (value) => {
              setPredicate(value);
              setSubject("");
              setObject("");
            },
            owned.map((p) => ({ value: p.id, label: p.label })),
          )}
          {entitySelect(
            "起点实体",
            subject,
            (value) => {
              setSubject(value);
              setObject("");
            },
            workspace.entities.filter((e) => current?.domain.includes(e.kind)),
          )}
          {entitySelect(
            "终点实体",
            object,
            setObject,
            workspace.entities.filter(
              (e) => e.id !== subject && objectKinds.includes(e.kind),
            ),
          )}
        </>
      )}
      {type === "history" && (
        <div className="version-list">
          {[
            core!,
            ...(vault?.history.filter((e) => e.id === core!.id).reverse() ||
              []),
          ].map((e) => (
            <div key={e.version}>
              <h3>
                v{e.version} · {e.title}
              </h3>
              <p className="muted small-text">
                {t("确认于 {at}", { at: time(e.confirmed_at) })}
              </p>
              <p className="preserve-text">{e.body}</p>
            </div>
          ))}
        </div>
      )}
      {type === "detail" && <Detail entity={entity!} canRevoke={canRevoke} />}
    </Modal>
  );
}

function Detail({ entity, canRevoke }: { entity: Entity; canRevoke: boolean }) {
  const fields = (detailFields[entity.kind] || []).filter(
    ([key]) => String(entity.properties[key] ?? "") !== "",
  );
  const value = (key: string): ReactNode => {
    const raw = String(entity.properties[key]);
    return key === "platform" || key === "protocol" ? label(raw) : raw;
  };
  return (
    <>
      <div className="detail-meta">
        <span>{label(entity.kind)}</span>
        <StatusBadge status={entity.status} />
        <span>v{entity.version}</span>
      </div>
      {canRevoke && (
        <div className="privacy-note">
          <ShieldCheck size={17} />
          <span>
            {t("撤回后，后续助理上下文与本地模型的核心原则将不再包含这一方向。已生成的任务和已读取的内容无法自动收回。")}
          </span>
        </div>
      )}
      <Values values={entity.values} />
      <Scenes scenes={entity.scenes} />
      <p className="preserve-text">
        {prop(entity, "body") || prop(entity, "rationale")}
      </p>
      {entity.kind === "resource" && (
        <dl>
          <dt>{t("库存")}</dt>
          <dd>
            {num(entity, "quantity")} {t(prop(entity, "unit"))}
          </dd>
          <dt>{t("最低库存")}</dt>
          <dd>{num(entity, "minimum")}</dd>
          {num(entity, "unit_cost_cents") > 0 && (
            <>
              <dt>{t("单位成本")}</dt>
              <dd>{money(num(entity, "unit_cost_cents"))}</dd>
            </>
          )}
        </dl>
      )}
      {fields.length > 0 && (
        <dl>
          {fields.map(([key, name]) => (
            <div key={key} className="detail-row">
              <dt>{t(name)}</dt>
              <dd>{value(key)}</dd>
            </div>
          ))}
        </dl>
      )}
      <p className="muted small-text">
        {t("建立于 {at}", { at: time(entity.created_at) })}
      </p>
      <code className="entity-id">{entity.id}</code>
    </>
  );
}

function entitySelect(
  name: string,
  value: string,
  setter: (value: string) => void,
  entities: Entity[],
) {
  return (
    <label className="field">
      {t(name)}
      <select
        required
        aria-label={t(name)}
        value={value}
        onChange={(event) => setter(event.target.value)}
      >
        <option value="">{t("选择实体")}</option>
        {entities.map((e) => (
          <option key={e.id} value={e.id}>
            {label(e.kind)} · {e.title}
          </option>
        ))}
      </select>
    </label>
  );
}
