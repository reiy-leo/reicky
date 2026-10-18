import { emit, listen } from "@tauri-apps/api/event";
// 界面语言：中文 / English。默认跟随系统，手动选择存在 localStorage。
export type Lang = "zh" | "en";

const dict = {
  zh: {
    placeholder: "随手贴点什么……拖文件进来只记路径",
    color: "颜色",
    shape: "变换形状",
    archive: "归档",
    unarchive: "恢复",
    list: "笔记列表",
    close: "关闭",
    stow: "收起来（留在笔记列表里）",
    quit: "退出 reicky",
    newNote: "新建便签（⌘/Ctrl+N）",
    tabNotes: "笔记",
    tabArchived: "已归档",
    language: "语言",
    renderLinks: "链接",
    renderLists: "列表",
    renderTodo: "待办",
    settings: "设置",
    ago: (y: number, m: number, d: number) => [y && `${y}年`, m && `${m}个月`, d && `${d}天`].filter(Boolean).join("") + "前",
    back: "返回",
    renderSection: "便签失焦时渲染",
    trayIcon: "菜单栏图标",
    general: "通用",
    autostart: "随系统启动",
    autostartFailed: (e: unknown) => `开机启动设置失败：${e}`,
    trayStyles: { default: "默认", "layout-freeform": "自由布局", "layout-grid-circles": "圆点网格", waypoints: "节点", bubbles: "气泡" } as Record<string, string>,
    trayFailed: (e: unknown) => `换图标失败：${e}`,
    readonlyArchived: "已归档，只读",
    themeSystem: "跟随系统",
    themeLight: "浅色",
    themeDark: "深色",
    shapeTall: "瘦高",
    shapeWide: "宽扁",
    shapeSquare: "正方形",
    pathTip: "单击打开，⌥/Alt+单击在文件管理器中显示",
    pathRemove: "移除这条路径引用（不会动文件）",
    pathsAdded: (n: number) => `已记下 ${n} 个路径`,
    pathCount: (n: number) => `${n} 个路径`,
    saveFailed: (e: unknown) => `保存失败：${e}`,
    loadFailed: (e: unknown) => `读取失败：${e}`,
    openFailed: (e: unknown) => `打不开：${e}`,
    archived: "已归档",
    archivedNew: "已归档，开了一张新的",
    restored: "已恢复",
    emptyNoArchive: "空白便签不用归档",
    empty: "（空）",
    emptyList: "还没有笔记",
    noMatch: "没有匹配的笔记",
    search: "搜索文字或路径",
    filterBy: (c: string) => `只看${c}色`,
    filterClear: "取消颜色筛选",
    mainNote: "主便签",
    isOpen: "正开着",
    emptyArchived: "还没有归档的笔记",
    pinOn: "已置顶，点击取消",
    pinOff: "未置顶，点击置顶",
    pinnedNow: "已置顶",
    unpinnedNow: "已取消置顶",
    copy: "拷贝（有选中拷选中，否则整张）",
    paste: "粘贴到光标处",
    copiedAll: "已拷贝整张便签",
    copiedSel: "已拷贝选中内容",
    pasted: "已粘贴",
    nothingToCopy: "没有可拷贝的内容",
    clipEmpty: "剪贴板里没有文本",
    clipFailed: (e: unknown) => `剪贴板出错：${e}`,
    colors: { yellow: "黄", cream: "奶油", orange: "橙", coral: "珊瑚", pink: "粉", purple: "紫", blue: "蓝", cyan: "青", mint: "薄荷", green: "绿", white: "白", gray: "灰" },
  },
  en: {
    placeholder: "Jot anything… dropped files are kept as paths only",
    color: "Color",
    shape: "Change shape",
    archive: "Archive",
    unarchive: "Restore",
    list: "Notes",
    close: "Close",
    stow: "Tuck away (stays in Notes)",
    quit: "Quit reicky",
    newNote: "New note (⌘/Ctrl+N)",
    tabNotes: "Notes",
    tabArchived: "Archived",
    language: "Language",
    renderLinks: "Links",
    renderLists: "Lists",
    renderTodo: "To-dos",
    settings: "Settings",
    ago: (y: number, m: number, d: number) => [y && `${y}y`, m && `${m}m`, d && `${d}d`].filter(Boolean).join(" ") + " ago",
    back: "Back",
    renderSection: "Render when a note loses focus",
    trayIcon: "Menu bar icon",
    general: "General",
    autostart: "Launch at login",
    autostartFailed: (e: unknown) => `Couldn't change launch at login: ${e}`,
    trayStyles: { default: "Default", "layout-freeform": "Freeform", "layout-grid-circles": "Grid circles", waypoints: "Waypoints", bubbles: "Bubbles" } as Record<string, string>,
    trayFailed: (e: unknown) => `Couldn't change icon: ${e}`,
    readonlyArchived: "Archived · read-only",
    themeSystem: "Auto",
    themeLight: "Light",
    themeDark: "Dark",
    shapeTall: "Tall",
    shapeWide: "Wide",
    shapeSquare: "Square",
    pathTip: "Click to open, ⌥/Alt+click to reveal in file manager",
    pathRemove: "Remove this path reference (the file is untouched)",
    pathsAdded: (n: number) => `Saved ${n} path${n > 1 ? "s" : ""}`,
    pathCount: (n: number) => `${n} path${n > 1 ? "s" : ""}`,
    saveFailed: (e: unknown) => `Save failed: ${e}`,
    loadFailed: (e: unknown) => `Load failed: ${e}`,
    openFailed: (e: unknown) => `Can't open: ${e}`,
    archived: "Archived",
    archivedNew: "Archived, started a fresh note",
    restored: "Restored",
    emptyNoArchive: "Nothing to archive in a blank note",
    empty: "(empty)",
    emptyList: "No notes yet",
    noMatch: "No matching notes",
    search: "Search text or paths",
    filterBy: (c: string) => `Only ${c.toLowerCase()}`,
    filterClear: "Clear color filter",
    mainNote: "Main note",
    isOpen: "Open now",
    emptyArchived: "No archived notes yet",
    pinOn: "Always on top — click to unpin",
    pinOff: "Not on top — click to pin",
    pinnedNow: "Pinned on top",
    unpinnedNow: "Unpinned",
    copy: "Copy (selection, or the whole note)",
    paste: "Paste at cursor",
    copiedAll: "Copied the whole note",
    copiedSel: "Copied selection",
    pasted: "Pasted",
    nothingToCopy: "Nothing to copy",
    clipEmpty: "No text on the clipboard",
    clipFailed: (e: unknown) => `Clipboard error: ${e}`,
    colors: { yellow: "Yellow", cream: "Cream", orange: "Orange", coral: "Coral", pink: "Pink", purple: "Purple", blue: "Blue", cyan: "Cyan", mint: "Mint", green: "Green", white: "White", gray: "Gray" },
  },
} as const;

const KEY = "reicky.lang";

function detect(): Lang {
  const saved = localStorage.getItem(KEY);
  if (saved === "zh" || saved === "en") return saved;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}

export let lang: Lang = detect();
export let t = dict[lang];

export function setLang(l: Lang) {
  lang = l;
  t = dict[l];
  localStorage.setItem(KEY, l);
  document.documentElement.lang = l === "zh" ? "zh-CN" : "en";
}

document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";

// 其他窗口切换语言时同步（用 Tauri 事件广播，WebView 之间不一定互发 storage 事件）
export function broadcastLang() {
  emit("lang-changed", lang).catch(() => {});
}
export function onLangChange(cb: () => void) {
  listen<Lang>("lang-changed", (e) => {
    if (e.payload === lang || (e.payload !== "zh" && e.payload !== "en")) return;
    setLang(e.payload);
    cb();
  });
}
