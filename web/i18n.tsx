import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { EN } from "./en";

export type Locale = "zh" | "en";
const STORAGE_KEY = "prm.locale";
const meta: Record<Locale, { lang: string; title: string }> = {
  zh: { lang: "zh-CN", title: "PRM · 个人资源管理" },
  en: { lang: "en", title: "PRM · Personal Resource Manager" },
};
function stored(): Locale {
  try {
    return localStorage.getItem(STORAGE_KEY) === "en" ? "en" : "zh";
  } catch {
    return "zh";
  }
}
// The active locale lives outside React so plain functions (formatting,
// validation errors) can read it; components re-render through the provider
// below whenever it changes.
let current: Locale = stored();
export const locale = () => current;

/**
 * Translate a Simplified Chinese source string into the active locale.
 * `en` overrides the dictionary for source strings that mean different things
 * in different contexts (for example the "收货" relation verb vs. its button).
 */
export function translate(
  text: string,
  vars?: Record<string, string | number>,
  en?: string,
): string {
  const template = current === "zh" ? text : en ?? EN[text] ?? text;
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (match, key) =>
    key in vars ? String(vars[key]) : match,
  );
}
export const t = translate;

const LocaleContext = createContext<{
  locale: Locale;
  setLocale: (next: Locale) => void;
}>({ locale: current, setLocale: () => {} });

/**
 * Wrap the whole app once. Components re-render on a language switch simply by
 * re-rendering from the tree root (there is no memoization in the UI).
 */
export function LocaleProvider({ children }: { children: ReactNode }) {
  const [active, setActive] = useState<Locale>(current);
  useEffect(() => {
    document.documentElement.lang = meta[active].lang;
    document.title = meta[active].title;
  }, [active]);
  const setLocale = (next: Locale) => {
    current = next;
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Read-only storage only loses the preference, never content.
    }
    setActive(next);
  };
  return (
    <LocaleContext.Provider value={{ locale: active, setLocale }}>
      {children}
    </LocaleContext.Provider>
  );
}
/** Subscribes the calling component to locale changes and returns `t`. */
export function useT() {
  useContext(LocaleContext);
  return translate;
}
export function useLocale() {
  return useContext(LocaleContext);
}
export function LocaleSelect() {
  const { locale: active, setLocale } = useLocale();
  return (
    <select
      className="locale-select"
      aria-label={t("界面语言")}
      value={active}
      onChange={(event) => setLocale(event.target.value as Locale)}
    >
      <option value="zh">中文</option>
      <option value="en">English</option>
    </select>
  );
}
