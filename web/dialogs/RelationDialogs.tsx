import { useEffect, useState, type FormEvent } from "react";
import { api, prop, today, type Dialog, type SelfCatalog, type Workspace } from "../api";
import { Modal, SceneInput, SelectField, TextField } from "../components";
import { t } from "../i18n";

const types = ["relationship", "relationship_update", "interaction"];
export const isRelationDialog = (type: string) => types.includes(type);
const closeness: [string, string][] = [
  ["1", "1 · 完全分离"],
  ["2", "2"],
  ["3", "3"],
  ["4", "4 · 有一定交集"],
  ["5", "5"],
  ["6", "6"],
  ["7", "7 · 高度重叠"],
];
const valence: [string, string][] = [
  ["2", "很愉快"],
  ["1", "愉快"],
  ["0", "平常"],
  ["-1", "偏消极"],
  ["-2", "很消极"],
];
const partyName = (entity: { properties: Record<string, unknown> }) =>
  String(entity.properties.agent_name ?? "") ||
  String(entity.properties.person_name ?? "");

export function RelationDialog({
  dialog,
  workspace,
  onClose,
  onSaved,
}: {
  dialog: NonNullable<Dialog>;
  workspace: Workspace;
  onClose: () => void;
  onSaved: (message: string) => Promise<void>;
}) {
  const { type, entity } = dialog;
  const [catalog, setCatalog] = useState<SelfCatalog | null>(null);
  useEffect(() => {
    void api<SelfCatalog>("/self/catalog").then(setCatalog).catch(() => {});
  }, []);
  const people = workspace.entities.filter((e) => e.kind === "person");
  const [person, setPerson] = useState("");
  const [name, setName] = useState("");
  const [kind, setKind] = useState("friend");
  const [close, setClose] = useState(entity ? prop(entity, "closeness") || "4" : "4");
  const [satisfaction, setSatisfaction] = useState(
    entity && typeof entity.properties.satisfaction === "number" ? String(entity.properties.satisfaction) : "",
  );
  const [cadence, setCadence] = useState(entity ? prop(entity, "contact_interval_days") || "0" : "30");
  const [status, setStatus] = useState(entity?.status || "active");
  const [notes, setNotes] = useState(type === "relationship_update" && entity ? prop(entity, "notes") : "");
  const [scenes, setScenes] = useState<string[]>(type === "relationship_update" && entity ? entity.scenes : []);
  const [agentVisible, setAgentVisible] = useState(false);
  const [date, setDate] = useState(today());
  const [channel, setChannel] = useState("meet");
  const [feeling, setFeeling] = useState("0");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const parsedSatisfaction = () => {
    if (!satisfaction.trim()) return null;
    const value = Number(satisfaction);
    if (!Number.isFinite(value) || value < 1 || value > 5)
      throw new Error(t("满意度应为 1–5"));
    return value;
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError("");
    setBusy(true);
    try {
      let message = t("已保存");
      if (type === "relationship") {
        await api("/relationships", {
          ...(person ? { person_id: person } : { person_name: name }),
          type: kind,
          closeness: Number(close),
          satisfaction: parsedSatisfaction(),
          contact_interval_days: Number(cadence),
          notes,
          scenes,
          agent_visible: agentVisible,
        });
        message = t("关系已建立");
      } else if (type === "relationship_update") {
        await api(`/relationships/${entity!.id}`, {
          closeness: Number(close),
          satisfaction: parsedSatisfaction(),
          contact_interval_days: Number(cadence),
          status,
          notes,
          scenes,
          expected_version: entity!.version,
        });
        message = t("关系已更新");
      } else {
        await api(`/relationships/${entity!.id}/interactions`, {
          date,
          channel,
          valence: Number(feeling),
          notes,
          expected_version: entity!.version,
        });
        message = t("互动已记录");
      }
      await onSaved(message);
    } catch (reason) {
      setError((reason as Error).message);
    } finally {
      setBusy(false);
    }
  };
  const measures = (
    <>
      <div className="form-grid">
        <SelectField
          label="亲密度（IOS 1–7）"
          value={close}
          onChange={setClose}
          options={closeness.map(([value, optionLabel]) => ({ value, label: t(optionLabel) }))}
        />
        <TextField
          label="满意度（RAS 1–5，可选）"
          type="number"
          min={1}
          max={5}
          step="any"
          value={satisfaction}
          onChange={setSatisfaction}
          required={false}
        />
      </div>
      <TextField
        label="联系节奏（天，0 为不提醒）"
        type="number"
        min={0}
        max={3650}
        step={1}
        value={cadence}
        onChange={setCadence}
      />
    </>
  );
  return (
    <Modal
      title={
        type === "relationship"
          ? t("新建关系")
          : type === "relationship_update"
            ? t("调整关系 · {name}", { name: partyName(entity!) })
            : t("记录互动 · {name}", { name: partyName(entity!) })
      }
      onClose={onClose}
      onSubmit={submit}
      busy={busy}
      error={error}
    >
      {type === "relationship" && (
        <>
          <div className="form-grid">
            <SelectField
              label="已有人物"
              value={person}
              onChange={setPerson}
              placeholder="新建人物"
              options={people.map((p) => ({ value: p.id, label: p.title }))}
            />
            {!person && <TextField label="人物姓名" value={name} onChange={setName} />}
          </div>
          <SelectField
            label="关系类型"
            value={kind}
            onChange={setKind}
            options={(catalog?.relationship_types || [{ id: "friend", label: "朋友", scene: "social" }])
              .filter((entry) => !entry.id.startsWith("ai_"))
              .map((entry) => ({ value: entry.id, label: entry.label }))}
          />
          {measures}
          <TextField label="备注" value={notes} onChange={setNotes} maxLength={2000} required={false} />
          <SceneInput scenes={scenes} onChange={setScenes} legend="人生场景（不选则按关系类型默认）" />
          <label className="check-row">
            <input type="checkbox" checked={agentVisible} onChange={(e) => setAgentVisible(e.target.checked)} />
            {t("允许本地模型读取这段关系")}
          </label>
        </>
      )}
      {type === "relationship_update" && (
        <>
          {measures}
          <SelectField
            label="状态"
            value={status}
            onChange={setStatus}
            options={[
              { value: "active", label: "进行中" },
              { value: "paused", label: "暂停" },
              { value: "ended", label: "已结束" },
            ]}
          />
          <TextField label="备注" value={notes} onChange={setNotes} maxLength={2000} required={false} />
          <SceneInput scenes={scenes} onChange={setScenes} />
        </>
      )}
      {type === "interaction" && (
        <>
          <div className="form-grid">
            <TextField label="日期" type="date" value={date} onChange={setDate} />
            <SelectField
              label="方式"
              value={channel}
              onChange={setChannel}
              options={(catalog?.interaction_channels || [{ id: "meet", label: "见面" }]).map((entry) => ({
                value: entry.id,
                label: entry.label,
              }))}
            />
          </div>
          <SelectField
            label="情绪体验"
            value={feeling}
            onChange={setFeeling}
            options={valence.map(([value, optionLabel]) => ({ value, label: t(optionLabel) }))}
          />
          <TextField label="备注" value={notes} onChange={setNotes} maxLength={2000} required={false} />
          <p className="muted small-text">
            {t("互动按事件追加记录，并更新这段关系的最近联系日期。")}
          </p>
        </>
      )}
    </Modal>
  );
}
