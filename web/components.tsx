import { useEffect, useRef, type ReactNode, type FormEvent } from "react";
import { Inbox, X, LoaderCircle, Check, LockKeyhole, Eye, EyeOff } from "lucide-react";
import { label, SCENES, sceneLabel, visible, type Entity } from "./api";
import { t } from "./i18n";

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
          {label(value)}
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
      <legend>{t("评价维度")}</legend>
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
          {label(value)}
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
      <legend>{t(legend)}</legend>
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
export function Dots({ value, max, label: name }: { value: number; max: number; label: string }) {
  return (
    <span className="dots" role="img" aria-label={`${t(name)} ${value}/${max}`}>
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
  const state = on ? t("本地模型可见") : t("仅本人");
  return (
    <button
      type="button"
      className={`visibility ${on ? "on" : ""}`}
      disabled={busy}
      aria-label={t("{title}：{state}，点击切换", { title: entity.title, state })}
      title={
        on
          ? t("本地模型可以读取，点击改为仅本人")
          : t("仅本人，点击允许本地模型读取")
      }
      onClick={() => onToggle(entity)}
    >
      {on ? <Eye size={14} /> : <EyeOff size={14} />}
      {state}
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
            aria-label={t("关闭")}
            title={t("关闭")}
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
            {t("取消")}
          </button>
          {onSubmit && (
            <button className="button primary" disabled={busy}>
              {busy ? (
                <LoaderCircle className="spin" size={16} />
              ) : (
                <Check size={16} />
              )}
              {t(submit)}
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
        {t("本人确认")}
      </div>
      <label className="field">
        {t("账户口令")}
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
          ? t("我确认发布以上内容，允许助理读取并提出建议")
          : t("我已阅读，确认这是我本人的判断")}
      </label>
    </div>
  );
}
export function StatusBadge({ status }: { status: string }) {
  return (
    <span className={`status ${status}`}>
      <i />
      {label(status)}
    </span>
  );
}
/** A controlled text input wrapped in the standard field label. */
export function TextField({
  label: name,
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
      {t(name)}
      <input
        required={required}
        type={type}
        maxLength={type === "number" ? undefined : maxLength}
        min={min}
        max={max}
        step={step}
        placeholder={placeholder === undefined ? undefined : t(placeholder)}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
    </label>
  );
}
export function SelectField({
  label: name,
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
      {t(name)}
      <select
        aria-label={t(name)}
        required={required}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      >
        {placeholder !== undefined && <option value="">{t(placeholder)}</option>}
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {t(option.label)}
          </option>
        ))}
      </select>
    </label>
  );
}
