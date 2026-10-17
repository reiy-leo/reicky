import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { api, COLORS, type Color, type Note } from "./api";
import { lang, onLangChange, setLang, t, type Lang } from "./i18n";

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const editor = $<HTMLTextAreaElement>("editor");
const pathsEl = $<HTMLUListElement>("paths");
const statusEl = $("status");
const palette = $("palette");
const swatches = $("swatches");
const sheet = $("sheet");
const listEl = $<HTMLUListElement>("list");
const sentinel = $("list-sentinel");
const shapeIcon = $("shape-icon");

const win = getCurrentWindow();
const isMain = win.label === "main";
const noteParam = new URLSearchParams(location.search).get("note");
let note: Note | undefined;

// ---------- 平台观感 ----------
const ua = navigator.userAgent;
document.documentElement.dataset.platform = /Mac/i.test(ua) ? "mac" : /Win/i.test(ua) ? "windows" : "linux";
if (!isMain) document.body.classList.add("is-note-window");

// ---------- 状态提示 ----------
let statusTimer: number | undefined;
function flash(msg: string) {
  statusEl.textContent = msg;
  clearTimeout(statusTimer);
  statusTimer = window.setTimeout(() => (statusEl.textContent = ""), 2200);
}

// ---------- 文案 ----------
function applyTexts() {
  editor.placeholder = t.placeholder;
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    el.textContent = String(t[el.dataset.i18n as keyof typeof t] ?? "");
  });
  document.querySelectorAll<HTMLElement>("[data-i18n-title]").forEach((el) => {
    const v = String(t[el.dataset.i18nTitle as keyof typeof t] ?? "");
    el.title = v;
    el.setAttribute("aria-label", v);
  });
  document.querySelectorAll<HTMLElement>(".lang").forEach((b) => b.classList.toggle("active", b.dataset.lang === lang));
  renderSwatches();
  updateShapeTitle();
  if (note) renderPaths();
}

// ---------- 渲染笔记 ----------
function renderPaths() {
  if (!note) return;
  const n = note;
  pathsEl.replaceChildren(
    ...n.paths.map((p) => {
      const li = document.createElement("li");
      li.className = "path";
      const name = document.createElement("span");
      name.textContent = p.split(/[\\/]/).filter(Boolean).pop() ?? p;
      name.title = `${p}\n${t.pathTip}`;
      name.addEventListener("click", (e) => {
        const run = e.altKey ? revealItemInDir(p) : openPath(p);
        run.catch((err) => flash(t.openFailed(err)));
      });
      const rm = document.createElement("button");
      rm.className = "path-rm";
      rm.textContent = "×";
      rm.title = t.pathRemove;
      rm.setAttribute("aria-label", t.pathRemove);
      rm.addEventListener("click", () => api.removePath(n.id, p).then(setNote).catch((err) => flash(t.saveFailed(err))));
      li.append(name, rm);
      return li;
    }),
  );
}

function setNote(n: Note, force = false) {
  note = n;
  document.documentElement.dataset.color = n.color;
  if (force || document.activeElement !== editor) {
    if (editor.value !== n.text) editor.value = n.text;
  }
  renderPaths();
  if (!isMain) win.setTitle(n.text.split("\n")[0].slice(0, 40) || "reicky").catch(() => {});
}

function renderSwatches() {
  swatches.replaceChildren(
    ...COLORS.map(({ id }) => {
      const b = document.createElement("button");
      b.className = "swatch";
      b.dataset.swatch = id;
      b.title = t.colors[id];
      b.setAttribute("aria-label", t.colors[id]);
      b.addEventListener("click", async () => {
        if (!note) return;
        palette.hidden = true;
        setNote(await api.setColor(note.id, id as Color));
      });
      return b;
    }),
  );
}

// ---------- 自动保存 ----------
let saveTimer: number | undefined;
function flushSave() {
  if (saveTimer === undefined || !note) return;
  clearTimeout(saveTimer);
  saveTimer = undefined;
  const id = note.id;
  api.saveText(id, editor.value).then((n) => {
    if (note?.id === id) note = { ...n, text: editor.value };
  }).catch((err) => flash(t.saveFailed(err)));
}
editor.addEventListener("input", () => {
  clearTimeout(saveTimer);
  saveTimer = window.setTimeout(flushSave, 400);
});
editor.addEventListener("blur", flushSave);
window.addEventListener("beforeunload", flushSave);

// ---------- 颜色 / 语言弹层 ----------
$("btn-color").addEventListener("click", (e) => {
  e.stopPropagation();
  palette.hidden = !palette.hidden;
});
document.addEventListener("click", (e) => {
  if (!palette.hidden && !palette.contains(e.target as Node)) palette.hidden = true;
});
document.querySelectorAll<HTMLButtonElement>(".lang").forEach((b) =>
  b.addEventListener("click", () => {
    setLang(b.dataset.lang as Lang);
    applyTexts();
    if (!sheet.hidden) resetList();
  }),
);
onLangChange(() => {
  applyTexts();
  if (!sheet.hidden) resetList();
});

// ---------- 变换形状：瘦高 → 宽扁 → 正方形 循环 ----------
type Shape = "tall" | "wide" | "square";
const SHAPES: Shape[] = ["tall", "wide", "square"];
const SIZE: Record<Shape, [number, number]> = { tall: [240, 420], wide: [440, 240], square: [320, 320] };
const ICON: Record<Shape, [number, number, number, number]> = {
  tall: [8, 4, 8, 16],
  wide: [3, 8, 18, 8],
  square: [5, 5, 14, 14],
};
const shapeKey = `reicky.shape.${win.label}`;
let shape: Shape = (SHAPES as string[]).includes(localStorage.getItem(shapeKey) ?? "")
  ? (localStorage.getItem(shapeKey) as Shape)
  : "square";

function updateShapeTitle() {
  const name = { tall: t.shapeTall, wide: t.shapeWide, square: t.shapeSquare }[shape];
  const b = $("btn-shape");
  b.title = `${t.shape}（${name}）`;
  b.setAttribute("aria-label", b.title);
  const [x, y, w, h] = ICON[shape];
  shapeIcon.setAttribute("x", String(x));
  shapeIcon.setAttribute("y", String(y));
  shapeIcon.setAttribute("width", String(w));
  shapeIcon.setAttribute("height", String(h));
}
$("btn-shape").addEventListener("click", async () => {
  shape = SHAPES[(SHAPES.indexOf(shape) + 1) % SHAPES.length];
  localStorage.setItem(shapeKey, shape);
  updateShapeTitle();
  const [w, h] = SIZE[shape];
  await win.setSize(new LogicalSize(w, h));
  flash({ tall: t.shapeTall, wide: t.shapeWide, square: t.shapeSquare }[shape]);
});

// ---------- 归档（唯一的“删除”） ----------
$("btn-archive").addEventListener("click", async () => {
  if (!note) return;
  if (!editor.value.trim() && note.paths.length === 0) return flash(t.emptyNoArchive);
  flushSave();
  try {
    const fresh = await api.archive(note.id);
    if (fresh) {
      setNote(fresh, true);
      flash(t.archivedNew);
    } else if (!isMain) {
      await win.close();
    } else {
      flash(t.archived);
    }
  } catch (err) {
    flash(t.saveFailed(err));
  }
});

// ---------- 笔记列表：底部弹出，modified 倒序，滚动加载 ----------
const PAGE = 30;
let offset = 0;
let hasMore = true;
let loading = false;
let showArchived = false;

function fmtTime(ms: number) {
  const d = new Date(ms);
  const loc = lang === "zh" ? "zh-CN" : "en-US";
  return d.toDateString() === new Date().toDateString()
    ? d.toLocaleTimeString(loc, { hour: "2-digit", minute: "2-digit" })
    : d.toLocaleDateString(loc, { year: "numeric", month: "short", day: "numeric" });
}

async function loadMore() {
  if (loading || !hasMore) return;
  loading = true;
  try {
    const page = await api.list(offset, PAGE, showArchived);
    offset += page.items.length;
    hasMore = page.hasMore;
    for (const it of page.items) {
      if (!showArchived && it.id === note?.id) continue; // 主窗口正在编辑的那条不重复列出
      const li = document.createElement("li");
      li.className = "item";
      li.dataset.color = it.color;
      const text = document.createElement("div");
      text.className = "item-text";
      text.textContent = it.preview || t.empty;
      const meta = document.createElement("div");
      meta.className = "item-meta";
      meta.textContent = [fmtTime(it.modified), it.pathCount ? t.pathCount(it.pathCount) : ""].filter(Boolean).join(" · ");
      li.append(text, meta);
      if (it.archived) {
        const r = document.createElement("button");
        r.className = "lang";
        r.style.marginTop = "4px";
        r.textContent = t.unarchive;
        r.addEventListener("click", async (e) => {
          e.stopPropagation();
          await api.unarchive(it.id);
          li.remove();
          flash(t.restored);
        });
        li.append(r);
      }
      li.addEventListener("click", () => api.openWindow(it.id).catch((err) => flash(t.openFailed(err))));
      listEl.append(li);
    }
    if (!hasMore && listEl.childElementCount === 0) {
      const li = document.createElement("li");
      li.className = "empty";
      li.textContent = showArchived ? t.emptyArchived : t.emptyList;
      listEl.append(li);
    }
  } catch (err) {
    flash(t.loadFailed(err));
  } finally {
    loading = false;
  }
  // 一页没填满可视区时继续加载
  if (hasMore && sentinel.getBoundingClientRect().top < window.innerHeight) loadMore();
}

function resetList() {
  listEl.replaceChildren();
  offset = 0;
  hasMore = true;
  document.querySelectorAll<HTMLElement>(".tab").forEach((b) =>
    b.classList.toggle("active", (b.dataset.archived === "true") === showArchived),
  );
  loadMore();
}

new IntersectionObserver((es) => {
  if (!sheet.hidden && es.some((e) => e.isIntersecting)) loadMore();
}, { root: listEl.parentElement }).observe(sentinel);

function openSheet() {
  flushSave();
  palette.hidden = true;
  sheet.hidden = false;
  requestAnimationFrame(() => sheet.classList.add("open"));
  resetList();
}
function closeSheet() {
  sheet.classList.remove("open");
  sheet.hidden = true;
}
$("btn-list").addEventListener("click", () => (sheet.hidden ? openSheet() : closeSheet()));
$("btn-close-sheet").addEventListener("click", closeSheet);
$("btn-quit").addEventListener("click", () => {
  flushSave();
  api.quit();
});
// 菜单栏图标左键 → 打开列表
if (isMain) listen("open-list", () => openSheet());
document.querySelectorAll<HTMLElement>(".tab").forEach((b) =>
  b.addEventListener("click", () => {
    showArchived = b.dataset.archived === "true";
    resetList();
  }),
);
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    palette.hidden = true;
    closeSheet();
  }
});

// ---------- 拖入文件 / 文件夹：只记路径 ----------
getCurrentWebview().onDragDropEvent(async (ev) => {
  const p = ev.payload;
  if (p.type === "enter" || p.type === "over") document.body.classList.add("dragging");
  else if (p.type === "leave") document.body.classList.remove("dragging");
  else if (p.type === "drop") {
    document.body.classList.remove("dragging");
    if (!note || p.paths.length === 0) return;
    try {
      setNote(await api.addPaths(note.id, p.paths));
      flash(t.pathsAdded(p.paths.length));
    } catch (err) {
      flash(t.saveFailed(err));
    }
  }
});

// ---------- 多窗口同步 ----------
listen<Note>("note-changed", (e) => {
  if (e.payload.id === note?.id) setNote(e.payload);
});

// ---------- 启动 ----------
(async () => {
  applyTexts();
  try {
    setNote(noteParam ? await api.get(noteParam) : await api.current(), true);
  } catch (err) {
    flash(t.loadFailed(err));
  }
})();
