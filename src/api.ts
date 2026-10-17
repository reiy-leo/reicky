import { invoke } from "@tauri-apps/api/core";

export type Color = "yellow" | "blue" | "green" | "pink" | "purple" | "gray";

export const COLORS: { id: Color; name: string }[] = [
  { id: "yellow", name: "黄" },
  { id: "blue", name: "蓝" },
  { id: "green", name: "绿" },
  { id: "pink", name: "粉" },
  { id: "purple", name: "紫" },
  { id: "gray", name: "灰" },
];

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
}

export interface Page {
  items: NoteSummary[];
  hasMore: boolean;
}

export const api = {
  quit: () => invoke<void>("quit_app"),
  current: () => invoke<Note>("get_current_note"),
  get: (id: string) => invoke<Note>("get_note", { id }),
  saveText: (id: string, text: string) => invoke<Note>("save_text", { id, text }),
  setColor: (id: string, color: Color) => invoke<Note>("set_color", { id, color }),
  addPaths: (id: string, paths: string[]) => invoke<Note>("add_paths", { id, paths }),
  removePath: (id: string, path: string) => invoke<Note>("remove_path", { id, path }),
  archive: (id: string) => invoke<Note | null>("archive_note", { id }),
  unarchive: (id: string) => invoke<Note>("unarchive_note", { id }),
  list: (offset: number, limit: number, archived: boolean) =>
    invoke<Page>("list_notes", { offset, limit, archived }),
  openWindow: (id: string) => invoke<void>("open_note_window", { id }),
};
