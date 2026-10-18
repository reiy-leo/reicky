// 各窗口共用：取元素、平台观感、悬停 tooltip、文案替换
import { t } from "./i18n";

export const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;

const ua = navigator.userAgent;
document.documentElement.dataset.platform = /Mac/i.test(ua) ? "mac" : /Win/i.test(ua) ? "windows" : "linux";

// ---------- 悬停提示：自绘 tooltip（原生 title 在置顶/未激活窗口里常常不出），夹在窗口内 ----------
const tip = document.createElement("div");
tip.className = "tip";
tip.hidden = true;
document.body.append(tip);
let tipTimer: number | undefined;
let tipFor: HTMLElement | null = null;
export function hideTip() {
  clearTimeout(tipTimer);
  tipFor = null;
  tip.hidden = true;
}
function showTip(el: HTMLElement) {
  tip.textContent = el.dataset.tip ?? "";
  tip.hidden = false;
  const r = el.getBoundingClientRect();
  const w = tip.offsetWidth, h = tip.offsetHeight, pad = 4;
  const left = Math.min(Math.max(pad, r.left + r.width / 2 - w / 2), innerWidth - w - pad);
  const above = r.top - h - 6;
  tip.style.left = `${left}px`;
  tip.style.top = `${above >= pad ? above : r.bottom + 6}px`;
}
document.addEventListener("mouseover", (e) => {
  const el = (e.target as HTMLElement).closest<HTMLElement>("[data-tip]");
  if (el === tipFor) return;
  hideTip();
  if (!el || !el.dataset.tip) return;
  tipFor = el;
  tipTimer = window.setTimeout(() => showTip(el), 350);
});
document.addEventListener("mousedown", hideTip, true);
document.addEventListener("mouseleave", hideTip);
window.addEventListener("blur", hideTip);

/** 把 data-i18n（文字）/ data-i18n-title（tooltip + 无障碍名）/ data-i18n-placeholder 换成当前语言 */
export function applyI18n() {
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    el.textContent = String(t[el.dataset.i18n as keyof typeof t] ?? "");
  });
  document.querySelectorAll<HTMLElement>("[data-i18n-title]").forEach((el) => {
    const v = String(t[el.dataset.i18nTitle as keyof typeof t] ?? "");
    el.dataset.tip = v;
    el.setAttribute("aria-label", v);
  });
  document.querySelectorAll<HTMLInputElement>("[data-i18n-placeholder]").forEach((el) => {
    el.placeholder = String(t[el.dataset.i18nPlaceholder as keyof typeof t] ?? "");
  });
}
