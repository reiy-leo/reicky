import { emit, listen } from "@tauri-apps/api/event";
import type { Dict } from "./locales/en";
import en from "./locales/en";
import zh from "./locales/zh";
import ja from "./locales/ja";
import de from "./locales/de";
import fr from "./locales/fr";
import es from "./locales/es";
import pt from "./locales/pt";
import ru from "./locales/ru";
import id from "./locales/id";
// 界面语言：9 种，默认跟随系统；设置里手动选的存在 localStorage（"system" 表示跟随系统）。
// 文案在 src/locales/，以 en.ts 的键为准（类型 Dict），缺键编译不过。

export const LANGS = ["zh", "en", "ja", "de", "fr", "es", "pt", "ru", "id"] as const;
export type Lang = (typeof LANGS)[number];
export type LangPref = Lang | "system";

/** 下拉框里显示的语言名：用各自的语言写 */
export const LANG_NAMES: Record<Lang, string> = {
  zh: "中文",
  en: "English",
  ja: "日本語",
  de: "Deutsch",
  fr: "Français",
  es: "Español",
  pt: "Português",
  ru: "Русский",
  id: "Bahasa Indonesia",
};

const dict: Record<Lang, Dict> = { zh, en, ja, de, fr, es, pt, ru, id };

/** 日期格式用的地区代码 */
const LOCALES: Record<Lang, string> = { zh: "zh-CN", en: "en-US", ja: "ja-JP", de: "de-DE", fr: "fr-FR", es: "es-ES", pt: "pt-BR", ru: "ru-RU", id: "id-ID" };

const KEY = "reicky.lang";

const isLang = (v: unknown): v is Lang => (LANGS as readonly unknown[]).includes(v);
const isPref = (v: unknown): v is LangPref => v === "system" || isLang(v);

/** 按系统语言挑：依次看 navigator.languages，取第一个支持的；都不支持就用英文 */
function systemLang(): Lang {
  for (const l of navigator.languages?.length ? navigator.languages : [navigator.language]) {
    const base = l.toLowerCase().split(/[-_]/)[0];
    if (base === "in") return "id"; // 印尼语的旧代码
    if (isLang(base)) return base;
  }
  return "en";
}

function readPref(): LangPref {
  const saved = localStorage.getItem(KEY);
  return isPref(saved) ? saved : "system";
}

export let langPref: LangPref = readPref();
export let lang: Lang = langPref === "system" ? systemLang() : langPref;
export let t: Dict = dict[lang];
export const locale = () => LOCALES[lang];

export function setLang(p: LangPref) {
  langPref = p;
  lang = p === "system" ? systemLang() : p;
  t = dict[lang];
  if (p === "system") localStorage.removeItem(KEY);
  else localStorage.setItem(KEY, p);
  document.documentElement.lang = LOCALES[lang];
}

document.documentElement.lang = LOCALES[lang];

// 其他窗口切换语言时同步（用 Tauri 事件广播，WebView 之间不一定互发 storage 事件）
export function broadcastLang() {
  emit("lang-changed", langPref).catch(() => {});
}
export function onLangChange(cb: () => void) {
  listen<LangPref>("lang-changed", (e) => {
    if (!isPref(e.payload) || e.payload === langPref) return;
    setLang(e.payload);
    cb();
  });
}
