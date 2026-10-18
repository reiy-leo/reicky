import { invoke } from "@tauri-apps/api/core";

// 便签颜色：paper 纸面、edge 纸边、dot 列表色条 / 色块描边。Rust 端 store.rs COLORS 要同步。
export const COLORS = [
  { id: "yellow", paper: "#fff7b1", edge: "#efe08a", dot: "#f2d64b" },
  { id: "cream", paper: "#fbf1dc", edge: "#ecdcb8", dot: "#dcc08a" },
  { id: "orange", paper: "#ffe2bf", edge: "#f5c891", dot: "#f5a54a" },
  { id: "coral", paper: "#ffd6cc", edge: "#f5b5a5", dot: "#ef7f66" },
  { id: "pink", paper: "#ffd9e8", edge: "#f5b6cf", dot: "#ef86b0" },
  { id: "purple", paper: "#e6dcff", edge: "#cbbaf5", dot: "#a48ae8" },
  { id: "blue", paper: "#d6ecff", edge: "#b3d6f5", dot: "#72b3ee" },
  { id: "cyan", paper: "#d2f4f7", edge: "#a9e0e6", dot: "#4fc3cf" },
  { id: "mint", paper: "#d3f5e6", edge: "#ade3cb", dot: "#5cc79a" },
  { id: "green", paper: "#d8f5c9", edge: "#b7e3a2", dot: "#7fcf5c" },
  { id: "white", paper: "#fdfdfb", edge: "#e4e4df", dot: "#d0d0c8" },
  { id: "gray", paper: "#ececec", edge: "#d2d2d2", dot: "#b3b3b3" },
] as const;

export type Color = (typeof COLORS)[number]["id"];
export const colorOf = (id: string) => COLORS.find((c) => c.id === id) ?? COLORS[0];

export interface Note {
  id: string;
  text: string;
  color: Color;
  paths: string[];
  archived: boolean;
  created: number;
  modified: number;
}

export interface NoteSummary {
  id: string;
  preview: string;
  color: Color;
  archived: boolean;
  pathCount: number;
  modified: number;
  /** 已在某个窗口开着 */
  open: boolean;
  /** 主便签那条 */
  main: boolean;
}

export interface Page {
  items: NoteSummary[];
  hasMore: boolean;
}

export const api = {
  newNote: (color: Color) => invoke<Note>("new_note_window", { color }),
  quit: () => invoke<void>("quit_app"),
  current: () => invoke<Note>("get_current_note"),
  get: (id: string) => invoke<Note>("get_note", { id }),
  saveText: (id: string, text: string) => invoke<Note>("save_text", { id, text }),
  setColor: (id: string, color: Color) => invoke<Note>("set_color", { id, color }),
  addPaths: (id: string, paths: string[]) => invoke<Note>("add_paths", { id, paths }),
  removePath: (id: string, path: string) => invoke<Note>("remove_path", { id, path }),
  archive: (id: string) => invoke<Note | null>("archive_note", { id }),
  unarchive: (id: string) => invoke<Note>("unarchive_note", { id }),
  list: (offset: number, limit: number, archived: boolean, query?: string, color?: Color) =>
    invoke<Page>("list_notes", { offset, limit, archived, query: query || null, color: color ?? null }),
  showList: () => invoke<void>("show_list"),
  showMain: () => invoke<void>("show_main_window"),
  openWindow: (id: string) => invoke<void>("open_note_window", { id }),
};
