// 贴纸渲染：贴纸仍是纯文本，窗口失焦时按开关把链接 / 列表 / 待办渲染出来，点一下回到编辑。
// 开关是全局的：存 localStorage reicky.render，在列表面板底部切换，并广播 Tauri 事件让所有窗口同步。
import { emit, listen } from "@tauri-apps/api/event";
export interface RenderOpts {
  links: boolean;
  lists: boolean;
  todo: boolean;
}
const KEY = "reicky.render";
const DEFAULTS: RenderOpts = { links: true, lists: true, todo: true };

export function loadRender(): RenderOpts {
  try {
    return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(KEY) ?? "{}") };
  } catch {
    return { ...DEFAULTS };
  }
}
export function saveRender(o: RenderOpts) {
  localStorage.setItem(KEY, JSON.stringify(o));
  emit("render-changed", o).catch(() => {});
}
export function onRenderChange(cb: (o: RenderOpts) => void) {
  // 各窗口的 WebView 不一定互发 storage 事件，以 Tauri 事件为准
  listen<RenderOpts>("render-changed", (e) => cb({ ...DEFAULTS, ...e.payload }));
}

const TODO = /^(\s*)(?:[-*+]\s+)?\[( |x|X)\]\s?(.*)$/;
const UL = /^(\s*)[-*+]\s+(.*)$/;
const OL = /^(\s*)(\d{1,4})[.)]\s+(.*)$/;
const URL_RE = /\b(?:https?:\/\/|www\.)[^\s<>"'，。；、）】]+[^\s<>"'，。；、）】.,;:!?)\]]/gi;

/** 有没有可渲染的东西；没有就一直用编辑器，省得来回切 */
export function hasRenderable(text: string, o: RenderOpts) {
  return text
    .split("\n")
    .some(
      (l) => (o.todo && TODO.test(l)) || (o.lists && (UL.test(l) || OL.test(l))) || (o.links && new RegExp(URL_RE.source, "i").test(l)),
    );
}

function inline(text: string, o: RenderOpts): Node[] {
  if (!o.links || !text) return [document.createTextNode(text)];
  const out: Node[] = [];
  let last = 0;
  for (const m of text.matchAll(URL_RE)) {
    const i = m.index ?? 0;
    if (i > last) out.push(document.createTextNode(text.slice(last, i)));
    const a = document.createElement("a");
    a.textContent = m[0];
    a.dataset.href = /^www\./i.test(m[0]) ? `https://${m[0]}` : m[0];
    a.dataset.tip = a.dataset.href;
    out.push(a);
    last = i + m[0].length;
  }
  if (last < text.length) out.push(document.createTextNode(text.slice(last)));
  return out;
}

const indentOf = (s: string) => s.replace(/\t/g, "    ").length;

/** 一行一个 div（data-line 是行号），点哪行就把光标放到哪行末尾 */
export function renderText(el: HTMLElement, text: string, o: RenderOpts) {
  el.replaceChildren(
    ...text.split("\n").map((line, i) => {
      const row = document.createElement("div");
      row.className = "r-line";
      row.dataset.line = String(i);
      const todo = o.todo ? line.match(TODO) : null;
      const li = !todo && o.lists ? (line.match(UL) ?? line.match(OL)) : null;
      if (todo) {
        const m = todo;
        row.classList.add("r-todo");
        row.style.paddingLeft = `${indentOf(m[1]) * 0.5}em`;
        const box = document.createElement("span");
        box.className = "r-box";
        box.dataset.check = String(i);
        if (m[2] !== " ") row.classList.add("done");
        const body = document.createElement("span");
        body.append(...inline(m[3], o));
        row.append(box, body);
      } else if (li) {
        const m = li;
        const ordered = /^\s*\d/.test(line);
        row.classList.add("r-li");
        row.style.paddingLeft = `${indentOf(m[1]) * 0.5}em`;
        const mark = document.createElement("span");
        mark.className = "r-mark";
        mark.textContent = ordered ? `${m[2]}.` : "•";
        const body = document.createElement("span");
        body.append(...inline(ordered ? m[3] : m[2], o));
        row.append(mark, body);
      } else {
        row.append(...inline(line, o));
        if (!line) row.append(document.createElement("br"));
      }
      return row;
    }),
  );
}

/** 切换第 n 行的 [ ] / [x] */
export function toggleTodo(text: string, n: number) {
  const lines = text.split("\n");
  lines[n] = lines[n].replace(/\[( |x|X)\]/, (_, c) => (c === " " ? "[x]" : "[ ]"));
  return lines.join("\n");
}

/** 第 n 行末尾在全文里的偏移 */
export function lineEnd(text: string, n: number) {
  const lines = text.split("\n");
  return lines.slice(0, n + 1).join("\n").length;
}
