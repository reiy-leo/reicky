// 各窗口共用：取元素、平台观感、悬停 tooltip、文案替换
import { t } from "./i18n";

export const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;

const ua = navigator.userAgent;
document.documentElement.dataset.platform = /Mac/i.test(ua) ? "mac" : /Win/i.test(ua) ? "windows" : "linux";

// ---------- 禁用 WebView 默认行为：右键菜单、开发者工具 / 刷新 / 查找 / 打印等快捷键 ----------
document.addEventListener("contextmenu", (e) => e.preventDefault(), { capture: true });
const isMac = document.documentElement.dataset.platform === "mac";
document.addEventListener(
  "keydown",
  (e) => {
    // mac 只看 ⌘（Ctrl+F/P 等是文本框里的 emacs 光标移动，不能拦）；其他平台看 Ctrl
    const mod = isMac ? e.metaKey : e.ctrlKey;
    const c = e.code;
    const blocked =
      ["F3", "F5", "F7", "F11", "F12"].includes(e.key) ||
      (mod && e.altKey && ["KeyI", "KeyJ", "KeyC", "KeyU"].includes(c)) || // 开发者工具 / 控制台 / 检查元素 / 源码
      (mod && e.shiftKey && ["KeyI", "KeyJ", "KeyC", "KeyR", "Delete"].includes(c)) ||
      (mod && !e.altKey && !e.shiftKey && ["KeyR", "KeyP", "KeyF", "KeyG", "KeyU", "KeyJ", "KeyH"].includes(c)) || // 刷新/打印/查找/源码/保存/下载/历史
      (mod && ["Equal", "Minus", "Digit0", "NumpadAdd", "NumpadSubtract"].includes(c)); // 缩放
    if (blocked) {
      e.preventDefault();
      e.stopImmediatePropagation();
    } else if (mod && !e.altKey && !e.shiftKey && c === "KeyS") {
      e.preventDefault(); // 挡掉 WebView 默认「保存网页」，贴纸窗口自己处理成「保存为文件」
    }
  },
  { capture: true },
);

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
  const w = tip.offsetWidth,
    h = tip.offsetHeight,
    pad = 4;
  const left = Math.min(Math.max(pad, r.left + r.width / 2 - w / 2), innerWidth - w - pad);
  const above = r.top - h - 6;
  tip.style.left = `${left}px`;
  tip.style.top = `${above >= pad ? above : r.bottom + 6}px`;
}
document.addEventListener("mouseover", (e) => {
  const el = (e.target as HTMLElement).closest<HTMLElement>("[data-tip]");
  if (el === tipFor) return;
  hideTip();
  if (!el?.dataset.tip) return;
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
