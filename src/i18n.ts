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
    quit: "退出 reicky",
    tabNotes: "笔记",
    tabArchived: "已归档",
    language: "语言",
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
    emptyList: "还没有别的笔记",
    emptyArchived: "还没有归档的笔记",
    colors: { yellow: "黄", blue: "蓝", green: "绿", pink: "粉", purple: "紫", gray: "灰" },
  },
  en: {
    placeholder: "Jot anything… dropped files are kept as paths only",
    color: "Color",
    shape: "Change shape",
    archive: "Archive",
    unarchive: "Restore",
    list: "Notes",
    close: "Close",
    quit: "Quit reicky",
    tabNotes: "Notes",
    tabArchived: "Archived",
    language: "Language",
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
    emptyList: "No other notes yet",
    emptyArchived: "No archived notes yet",
    colors: { yellow: "Yellow", blue: "Blue", green: "Green", pink: "Pink", purple: "Purple", gray: "Gray" },
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

// 其他窗口切换语言时同步（同源 localStorage 会触发 storage 事件）
export function onLangChange(cb: () => void) {
  window.addEventListener("storage", (e) => {
    if (e.key === KEY && (e.newValue === "zh" || e.newValue === "en")) {
      setLang(e.newValue);
      cb();
    }
  });
}
