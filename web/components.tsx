import { useEffect, useRef, type ReactNode, type FormEvent } from "react";
import { Inbox, X, LoaderCircle, Check, LockKeyhole, Eye, EyeOff } from "lucide-react";
import { labels, SCENES, sceneLabel, visible, type Entity } from "./api";

export function Empty({
  title,
  children,
}: {
  title: string;
  children?: ReactNode;
}) {
  return (
    <div className="empty">
      <Inbox size={30} strokeWidth={1.3} />
      <strong>{title}</strong>
      {children}
    </div>
  );
}
export function Values({ values }: { values: string[] }) {
  return (
    <span className="values">
      {values.map((value) => (
        <span key={value} className={`value ${value}`}>
          {labels[value]}
        </span>
      ))}
    </span>
  );
}
export function Scenes({ scenes }: { scenes: string[] }) {
  if (!scenes?.length) return null;
  return (
    <span className="scenes">
      {scenes.map((scene) => (
        <span key={scene} className={`scene-chip ${scene}`}>
          {sceneLabel(scene)}
        </span>
      ))}
    </span>
  );
}
export function ValueInput({
  values,
  onChange,
}: {
  values: string[];
  onChange: (values: string[]) => void;
}) {
  return (
    <fieldset className="value-input">
      <legend>评价维度</legend>
      {["need", "desire", "existence"].map((value) => (
        <label key={value}>
          <input
            type="checkbox"
            checked={values.includes(value)}
            onChange={(event) =>
              onChange(
                event.target.checked
                  ? [...values, value]
                  : values.filter((v) => v !== value),
              )
            }
          />
          {labels[value]}
        </label>
      ))}
    </fieldset>
  );
}
export function SceneInput({
  scenes,
  onChange,
  legend = "人生场景",
}: {
  scenes: string[];
  onChange: (scenes: string[]) => void;
  legend?: string;
}) {
  return (
    <fieldset className="value-input scene-input">
      <legend>{legend}</legend>
      {SCENES.map((scene) => (
        <label key={scene}>
          <input
            type="checkbox"
            checked={scenes.includes(scene)}
            onChange={(event) =>
              onChange(
                event.target.checked
                  ? [...scenes, scene]
                  : scenes.filter((s) => s !== scene),
              )
            }
          />
          {sceneLabel(scene)}
        </label>
      ))}
    </fieldset>
  );
}
/** Filled dots for ordinal ratings such as IOS closeness or centrality. */
export function Dots({ value, max, label }: { value: number; max: number; label: string }) {
  return (
    <span className="dots" role="img" aria-label={`${label} ${value}/${max}`}>
      {Array.from({ length: max }, (_, i) => (
        <i key={i} className={i < value ? "on" : ""} />
      ))}
    </span>
  );
}
export function VisibilityToggle({
  entity,
  busy,
  onToggle,
}: {
  entity: Entity;
  busy: boolean;
  onToggle: (entity: Entity) => void;
}) {
  const on = visible(entity);
  return (
    <button
      type="button"
      className={`visibility ${on ? "on" : ""}`}
      disabled={busy}
      aria-label={`${entity.title}：${on ? "本地模型可见" : "仅本人"}，点击切换`}
      title={on ? "本地模型可以读取，点击改为仅本人" : "仅本人，点击允许本地模型读取"}
      onClick={() => onToggle(entity)}
    >
      {on ? <Eye size={14} /> : <EyeOff size={14} />}
      {on ? "本地模型可见" : "仅本人"}
    </button>
  );
}
export function Modal({
  title,
  onClose,
  children,
  onSubmit,
  busy,
  submit = "保存",
  error,
  wide,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  onSubmit?: (event: FormEvent<HTMLFormElement>) => void;
  busy?: boolean;
  submit?: string;
  error?: string;
  wide?: boolean;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  return (
    <dialog
      ref={ref}
      className={`modal ${wide ? "wide" : ""}`}
      onCancel={(event) => {
        event.preventDefault();
        if (!busy) onClose();
      }}
    >
      <form onSubmit={onSubmit || ((event) => event.preventDefault())}>
        <header>
          <h2>{title}</h2>
          <button
            type="button"
            className="icon-button"
            aria-label="关闭"
            title="关闭"
            onClick={onClose}
            disabled={busy}
          >
            <X size={19} />
          </button>
        </header>
        <div className="modal-body">
          {children}
          {error && (
            <p role="alert" className="form-error">
              {error}
            </p>
          )}
        </div>
        <footer>
          <button
            type="button"
            className="button secondary"
            onClick={onClose}
            disabled={busy}
          >
            取消
          </button>
          {onSubmit && (
            <button className="button primary" disabled={busy}>
              {busy ? (
                <LoaderCircle className="spin" size={16} />
              ) : (
                <Check size={16} />
              )}
              {submit}
            </button>
          )}
        </footer>
      </form>
    </dialog>
  );
}
export function PasswordConfirm({
  password,
  setPassword,
  confirmed,
  setConfirmed,
  publish = false,
}: {
  password: string;
  setPassword: (v: string) => void;
  confirmed: boolean;
  setConfirmed: (v: boolean) => void;
  publish?: boolean;
}) {
  return (
    <div className="confirm-section">
      <div className="section-caption">
        <LockKeyhole size={15} />
        本人确认
      </div>
      <label className="field">
        账户口令
        <input
          required
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
        />
      </label>
      <label className="check-label">
        <input
          type="checkbox"
          required
          checked={confirmed}
          onChange={(event) => setConfirmed(event.target.checked)}
        />
        {publish
          ? "我确认发布以上内容，允许助理读取并提出建议"
          : "我已阅读，确认这是我本人的判断"}
      </label>
    </div>
  );
}
export function StatusBadge({ status }: { status: string }) {
  return (
    <span className={`status ${status}`}>
      <i />
      {labels[status] || status}
    </span>
  );
}
/** A controlled text input wrapped in the standard field label. */
export function TextField({
  label,
  value,
  onChange,
  type = "text",
  maxLength = 120,
  required = true,
  placeholder,
  min,
  max,
  step,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: string;
  maxLength?: number;
  required?: boolean;
  placeholder?: string;
  min?: number;
  max?: number;
  step?: number | "any";
}) {
  return (
    <label className="field">
      {label}
      <input
        required={required}
        type={type}
        maxLength={type === "number" ? undefined : maxLength}
        min={min}
        max={max}
        step={step}
        placeholder={placeholder}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
    </label>
  );
}
export function SelectField({
  label,
  value,
  onChange,
  options,
  required = false,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
  required?: boolean;
  placeholder?: string;
}) {
  return (
    <label className="field">
      {label}
      <select
        aria-label={label}
        required={required}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      >
        {placeholder !== undefined && <option value="">{placeholder}</option>}
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}
