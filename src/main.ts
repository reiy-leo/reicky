import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";
import { api, colorOf, COLORS, type Note } from "./api";
import { lang, onLangChange, setLang, t, type Lang } from "./i18n";
import { $, applyI18n, hideTip } from "./common";
const editor = $<HTMLTextAreaElement>("editor");
const pathsEl = $<HTMLUListElement>("paths");
const statusEl = $("status");
const palette = $("palette");
const swatches = $("swatches");
const shapeIcon = $("shape-icon");

const win = getCurrentWindow();
const isMain = win.label === "main";
const noteParam = new URLSearchParams(location.search).get("note");
let note: Note | undefined;

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
  applyI18n();
  document.querySelectorAll<HTMLElement>(".lang").forEach((b) => b.classList.toggle("active", b.dataset.lang === lang));
  renderSwatches();
  updateShapeTitle();
  renderPin();
  if (note) renderPaths();
}

// ---------- 渲染笔记 ----------
const ICON_DIR = '<svg viewBox="0 0 24 24"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>';
const ICON_FILE = '<svg viewBox="0 0 24 24"><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5" /></svg>';

function renderPaths() {
  if (!note) return;
  const n = note;
  pathsEl.replaceChildren(
    ...n.paths.map((p) => {
      const li = document.createElement("li");
      li.className = "path";
      const base = p.split(/[\\/]/).filter(Boolean).pop() ?? p;
      // 没有扩展名的当作文件夹（只是图标提示，不读磁盘）
      const isDir = !/\.[^.]+$/.test(base) || p.endsWith("/") || p.endsWith("\\");
      const name = document.createElement("span");
      name.className = "path-name";
      name.innerHTML = isDir ? ICON_DIR : ICON_FILE;
      name.append(base);
      name.dataset.tip = `${p}\n${t.pathTip}`;
      name.addEventListener("click", (e) => {
        const run = e.altKey ? revealItemInDir(p) : openPath(p);
        run.catch((err) => flash(t.openFailed(err)));
      });
      const rm = document.createElement("button");
      rm.className = "path-rm";
      rm.textContent = "×";
      rm.dataset.tip = t.pathRemove;
      rm.setAttribute("aria-label", t.pathRemove);
      rm.addEventListener("click", () => api.removePath(n.id, p).then(setNote).catch((err) => flash(t.saveFailed(err))));
      li.append(name, rm);
      return li;
    }),
  );
}

function setNote(n: Note, force = false) {
  note = n;
  const c = colorOf(n.color);
  const root = document.documentElement.style;
  root.setProperty("--paper", c.paper);
  root.setProperty("--paper-edge", c.edge);
  swatches.querySelectorAll<HTMLElement>(".swatch").forEach((b) => b.classList.toggle("active", b.dataset.swatch === c.id));
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
      b.style.background = colorOf(id).paper;
      if (note) b.classList.toggle("active", note.color === id);
      b.dataset.tip = t.colors[id];
      b.setAttribute("aria-label", t.colors[id]);
      b.addEventListener("click", async () => {
        if (!note) return;
        palette.hidden = true;
        setNote(await api.setColor(note.id, id));
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
  }),
);
onLangChange(applyTexts);

// ---------- 置顶开关：默认置顶，按窗口 label 记住 ----------
const pinKey = `reicky.pin.${win.label}`;
let pinned = localStorage.getItem(pinKey) !== "false";
function renderPin() {
  const b = $("btn-pin");
  b.setAttribute("aria-pressed", String(pinned));
  b.dataset.tip = pinned ? t.pinOn : t.pinOff;
  b.setAttribute("aria-label", b.dataset.tip);
}
async function applyPin(flashIt = false) {
  renderPin();
  try {
    await win.setAlwaysOnTop(pinned);
    if (flashIt) flash(pinned ? t.pinnedNow : t.unpinnedNow);
  } catch (err) {
    flash(t.saveFailed(err));
  }
}
$("btn-pin").addEventListener("click", () => {
  pinned = !pinned;
  localStorage.setItem(pinKey, String(pinned));
  hideTip();
  applyPin(true);
});
applyPin();

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
  b.dataset.tip = `${t.shape}（${name}）`;
  b.setAttribute("aria-label", b.dataset.tip);
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

// ---------- 笔记列表：独立面板窗口，贴着这个便签弹出 ----------
$("btn-list").addEventListener("click", () => {
  flushSave();
  palette.hidden = true;
  api.showList().catch((err) => flash(t.openFailed(err)));
});
// 收起来：便签窗口关掉、笔记留在列表里；主窗口收起（从菜单栏图标再唤出，Rust 端拦截了 close）
$("btn-stow").addEventListener("click", () => {
  flushSave();
  hideTip();
  win.close();
});
function newNote() {
  flushSave();
  api.newNote(note?.color ?? "yellow").catch((err) => flash(t.saveFailed(err)));
}
$("btn-new").addEventListener("click", newNote);

// ---------- 剪贴板：拷贝（有选区拷选区，否则整张）/ 粘贴到光标处 ----------
$("btn-copy").addEventListener("click", async () => {
  const { selectionStart: a, selectionEnd: b, value } = editor;
  const text = a !== b ? value.slice(a, b) : value;
  if (!text) return flash(t.nothingToCopy);
  try {
    await writeText(text);
    flash(a !== b ? t.copiedSel : t.copiedAll);
  } catch (err) {
    flash(t.clipFailed(err));
  }
});
// mousedown 不抢焦点，保住编辑器里的光标位置
for (const id of ["btn-copy", "btn-paste"]) $(id).addEventListener("mousedown", (e) => e.preventDefault());
$("btn-paste").addEventListener("click", async () => {
  try {
    const text = await readText();
    if (!text) return flash(t.clipEmpty);
    editor.focus();
    editor.setRangeText(text, editor.selectionStart, editor.selectionEnd, "end");
    editor.dispatchEvent(new Event("input"));
    flash(t.pasted);
  } catch (err) {
    flash(t.clipFailed(err));
  }
});
document.addEventListener("keydown", (e) => {
  if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "n") {
    e.preventDefault();
    newNote();
    return;
  }
  if (e.key === "Escape") palette.hidden = true;
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
