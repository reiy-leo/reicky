// 笔记列表面板：独立窗口，从菜单栏图标下方或便签旁边弹出，失焦自动收起（Rust 端处理）
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, colorOf, COLORS, paperOf, type Color, type NoteSummary } from "./api";
import { isDark, onThemeChange } from "./theme";
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

function fmtTime(ms: number) {
  const d = new Date(ms);
  const loc = lang === "zh" ? "zh-CN" : "en-US";
  return d.toDateString() === new Date().toDateString()
    ? d.toLocaleTimeString(loc, { hour: "2-digit", minute: "2-digit" })
    : d.toLocaleDateString(loc, { year: "numeric", month: "short", day: "numeric" });
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
  const bits = [fmtTime(it.modified), it.pathCount ? t.pathCount(it.pathCount) : ""];
  if (it.main) bits.push(t.mainNote);
  meta.textContent = bits.filter(Boolean).join(" · ");
  if (it.open) {
    const dot = document.createElement("span");
    dot.className = "open-dot";
    dot.dataset.tip = t.isOpen;
    meta.prepend(dot);
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

$("btn-new").addEventListener("click", () => api.newNote(color ?? "yellow").catch((err) => flash(t.saveFailed(err))));
$("btn-quit").addEventListener("click", () => api.quit());

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    if (q.value) {
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
  resetList();
  q.focus();
});

function applyTexts() {
  applyI18n();
  renderChips();
  resetList();
}
onLangChange(applyTexts);
applyTexts();

onThemeChange(renderChips);
