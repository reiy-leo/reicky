// 笔记列表面板：独立窗口，从菜单栏图标下方或便签旁边弹出，失焦自动收起（Rust 端处理）
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, colorOf, COLORS, paperOf, type Color, type NoteSummary } from "./api";
import { isDark, onThemeChange } from "./theme";
import { loadRender, onRenderChange, saveRender, type RenderOpts } from "./render";
import { lang, onLangChange, t } from "./i18n";
import { $, applyI18n, hideTip } from "./common";

const win = getCurrentWindow();
const listEl = $<HTMLUListElement>("list");
const sentinel = $("list-sentinel");
const q = $<HTMLInputElement>("q");
const chips = $("chips");
const statusEl = $("status");

let statusTimer: number | undefined;
function flash(msg: string) {
  statusEl.textContent = msg;
  clearTimeout(statusTimer);
  statusTimer = window.setTimeout(() => (statusEl.textContent = ""), 2200);
}

const PAGE = 30;
let offset = 0;
let hasMore = true;
let loading = false;
let gen = 0; // 每次重置 +1，丢掉过期的请求结果
let showArchived = false;
let color: Color | undefined;

// 今天：具体时间；更早：按日历算相差几年几月几天，如 3d ago / 1m 2d ago / 1y 3m 2d ago
function fmtTime(ms: number) {
  const d = new Date(ms);
  const now = new Date();
  if (d.toDateString() === now.toDateString()) {
    const loc = lang === "zh" ? "zh-CN" : "en-US";
    return d.toLocaleTimeString(loc, { hour: "2-digit", minute: "2-digit" });
  }
  let y = now.getFullYear() - d.getFullYear();
  let m = now.getMonth() - d.getMonth();
  let day = now.getDate() - d.getDate();
  if (day < 0) {
    m--;
    day += new Date(now.getFullYear(), now.getMonth(), 0).getDate(); // 上个月的天数
  }
  if (m < 0) {
    y--;
    m += 12;
  }
  if (y < 0) return d.toLocaleDateString(); // 时间在未来（时钟被调过）
  const parts = [y && `${y}y`, m && `${m}m`, day && `${day}d`].filter(Boolean);
  return `${parts.join(" ") || "1d"} ago`;
}

function renderChips() {
  chips.replaceChildren(
    ...COLORS.map(({ id, dot }) => {
      const b = document.createElement("button");
      b.className = "chip";
      b.style.background = paperOf(id, isDark).paper;
      b.style.borderColor = dot;
      b.classList.toggle("active", color === id);
      b.dataset.tip = color === id ? t.filterClear : t.filterBy(t.colors[id]);
      b.setAttribute("aria-label", b.dataset.tip);
      b.addEventListener("click", () => {
        color = color === id ? undefined : id;
        hideTip();
        renderChips();
        resetList();
      });
      return b;
    }),
  );
}

function item(it: NoteSummary) {
  const li = document.createElement("li");
  li.className = "item";
  li.style.setProperty("--c", colorOf(it.color).dot);
  const text = document.createElement("div");
  text.className = "item-text";
  text.textContent = it.preview || t.empty;
  const meta = document.createElement("div");
  meta.className = "item-meta";
  const bits = [it.pathCount ? t.pathCount(it.pathCount) : ""];
  if (it.main) bits.push(t.mainNote);
  const left = document.createElement("span");
  left.className = "meta-left";
  left.textContent = bits.filter(Boolean).join(" · ");
  const time = document.createElement("span");
  time.className = "meta-time";
  time.textContent = fmtTime(it.modified);
  time.dataset.tip = new Date(it.modified).toLocaleString(lang === "zh" ? "zh-CN" : "en-US");
  meta.append(left, time);
  if (it.open) {
    const dot = document.createElement("span");
    dot.className = "open-dot";
    dot.dataset.tip = t.isOpen;
    left.prepend(dot);
  }
  li.append(text, meta);
  if (it.archived) {
    const r = document.createElement("button");
    r.className = "lang restore";
    r.textContent = t.unarchive;
    r.addEventListener("click", async (e) => {
      e.stopPropagation();
      try {
        await api.unarchive(it.id);
        li.remove();
        flash(t.restored);
      } catch (err) {
        flash(t.saveFailed(err));
      }
    });
    li.append(r);
  } else {
    li.addEventListener("click", () => {
      const run = it.main ? api.showMain() : api.openWindow(it.id);
      run.catch((err) => flash(t.openFailed(err)));
    });
  }
  return li;
}

async function loadMore() {
  if (loading || !hasMore) return;
  loading = true;
  const my = gen;
  try {
    const page = await api.list(offset, PAGE, showArchived, q.value, color);
    if (my !== gen) return;
    offset += page.items.length;
    hasMore = page.hasMore;
    listEl.append(...page.items.map(item));
    if (!hasMore && listEl.childElementCount === 0) {
      const li = document.createElement("li");
      li.className = "empty";
      li.textContent = q.value.trim() || color ? t.noMatch : showArchived ? t.emptyArchived : t.emptyList;
      listEl.append(li);
    }
  } catch (err) {
    flash(t.loadFailed(err));
  } finally {
    loading = false;
  }
  if (my !== gen) return loadMore();
  // 一页没填满可视区时继续加载
  if (hasMore && sentinel.getBoundingClientRect().top < window.innerHeight) loadMore();
}

function resetList() {
  gen++;
  listEl.replaceChildren();
  offset = 0;
  hasMore = true;
  loading = false;
  document.querySelectorAll<HTMLElement>(".tab").forEach((b) =>
    b.classList.toggle("active", (b.dataset.archived === "true") === showArchived),
  );
  loadMore();
}

new IntersectionObserver((es) => {
  if (es.some((e) => e.isIntersecting)) loadMore();
}, { root: listEl.parentElement }).observe(sentinel);

document.querySelectorAll<HTMLElement>(".tab").forEach((b) =>
  b.addEventListener("click", () => {
    showArchived = b.dataset.archived === "true";
    resetList();
  }),
);

let qTimer: number | undefined;
q.addEventListener("input", () => {
  clearTimeout(qTimer);
  qTimer = window.setTimeout(resetList, 150);
});

$("btn-quit").addEventListener("click", () => api.quit());

// ---------- 设置页 ----------
const paneList = $("pane-list");
const paneSettings = $("pane-settings");
const btnSettings = $("btn-settings");
function showSettings(on: boolean) {
  paneList.hidden = on;
  paneSettings.hidden = !on;
  btnSettings.classList.toggle("active", on);
  hideTip();
  if (!on) q.focus();
}
btnSettings.addEventListener("click", () => showSettings(paneSettings.hidden));
$("btn-back").addEventListener("click", () => showSettings(false));

// 菜单栏图标样式（lucide 图标；default = 原图标）
const TRAY_ICONS: Record<string, string> = {
  default: "",
  "layout-freeform": '<rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="4" rx="1"/><rect width="7" height="7" x="4" y="14" rx="1"/>',
  "layout-grid-circles": '<circle cx="17.5" cy="17.5" r="3.5"/><circle cx="17.5" cy="6.5" r="3.5"/><circle cx="6.5" cy="17.5" r="3.5"/><circle cx="6.5" cy="6.5" r="3.5"/>',
  waypoints: '<path d="m10.586 5.414-5.172 5.172"/><path d="m18.586 13.414-5.172 5.172"/><path d="M6 12h12"/><circle cx="12" cy="20" r="2"/><circle cx="12" cy="4" r="2"/><circle cx="20" cy="12" r="2"/><circle cx="4" cy="12" r="2"/>',
  bubbles: '<path d="M7.001 15.085A1.5 1.5 0 0 1 9 16.5"/><circle cx="18.5" cy="8.5" r="3.5"/><circle cx="7.5" cy="16.5" r="5.5"/><circle cx="7.5" cy="4.5" r="2.5"/>',
};
const trayBox = $("tray-styles");
let trayStyle = "default";
function renderTrayStyles() {
  trayBox.replaceChildren(
    ...Object.entries(TRAY_ICONS).map(([id, svg]) => {
      const b = document.createElement("button");
      b.className = "tray-opt";
      b.classList.toggle("active", id === trayStyle);
      b.setAttribute("role", "radio");
      b.setAttribute("aria-checked", String(id === trayStyle));
      b.dataset.tip = t.trayStyles[id];
      b.setAttribute("aria-label", t.trayStyles[id]);
      b.innerHTML = svg
        ? `<svg viewBox="0 0 24 24">${svg}</svg>`
        : '<img src="/tray-template.png" alt="" draggable="false" />';
      b.addEventListener("click", async () => {
        if (id === trayStyle) return;
        try {
          await api.setTrayIcon(id);
          trayStyle = id;
          renderTrayStyles();
        } catch (err) {
          flash(t.trayFailed(err));
        }
      });
      return b;
    }),
  );
}
api.getTrayIcon().then((s) => {
  trayStyle = s;
  renderTrayStyles();
}).catch(() => {});

// ---------- 渲染开关（作用于所有便签窗口） ----------
const toggles = document.querySelectorAll<HTMLInputElement>("[data-render]");
function syncToggles(o: RenderOpts) {
  toggles.forEach((x) => (x.checked = o[x.dataset.render as keyof RenderOpts]));
}
toggles.forEach((x) =>
  x.addEventListener("change", () => {
    const o = loadRender();
    o[x.dataset.render as keyof RenderOpts] = x.checked;
    saveRender(o);
  }),
);
syncToggles(loadRender());
onRenderChange(syncToggles);

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    if (!paneSettings.hidden) showSettings(false);
    else if (q.value) {
      q.value = "";
      resetList();
    } else win.hide();
  }
  if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "n") {
    e.preventDefault();
    api.newNote(color ?? "yellow").catch((err) => flash(t.saveFailed(err)));
  }
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
    e.preventDefault();
    q.focus();
    q.select();
  }
});

// 每次弹出都刷新（开着/关着的状态、别处的修改）
listen("list-shown", () => {
  showSettings(false);
  resetList();
  q.focus();
});

function applyTexts() {
  applyI18n();
  renderTrayStyles();
  renderChips();
  resetList();
}
onLangChange(applyTexts);
applyTexts();

onThemeChange(renderChips);
