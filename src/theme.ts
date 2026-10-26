import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

// 外观：跟随系统 / 浅色 / 深色。存 localStorage，所有窗口通过 storage 事件同步。
export type ThemeMode = "system" | "light" | "dark";
const KEY = "reicky.theme";
const media = window.matchMedia("(prefers-color-scheme: dark)");

function load(): ThemeMode {
  const v = localStorage.getItem(KEY);
  return v === "light" || v === "dark" ? v : "system";
}

export let themeMode: ThemeMode = load();
export let isDark = false;
const listeners: (() => void)[] = [];

function apply() {
  isDark = themeMode === "dark" || (themeMode === "system" && media.matches);
  document.documentElement.dataset.theme = isDark ? "dark" : "light";
  // 原生窗口外观（mac 圆角描边/阴影）跟着走；跟随系统时交还给系统
  getCurrentWindow()
    .setTheme(themeMode === "system" ? null : themeMode)
    .catch(() => {});
  listeners.forEach((cb) => {
    cb();
  });
}

export function setThemeMode(m: ThemeMode) {
  themeMode = m;
  localStorage.setItem(KEY, m);
  apply();
  emit("theme-changed", m).catch(() => {});
}

export function onThemeChange(cb: () => void) {
  listeners.push(cb);
}

media.addEventListener("change", () => themeMode === "system" && apply());
listen<ThemeMode>("theme-changed", (e) => {
  if (e.payload === themeMode) return;
  themeMode = e.payload;
  apply();
});
apply();
