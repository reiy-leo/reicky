# MEMORY

## 决策
- 2026-10-17 选 Tauri 2：包小、三平台、Rust 后端便于做同步。
- 2026-10-17 每条笔记单独一个 JSON 文件，便于后续按文件做 Drive / WebDAV 增量同步。
- 2026-10-17 形状切换按窗口记在 localStorage（`reicky.shape.<window label>`），尺寸：瘦高 240×400、宽扁 420×220、正方形 320×320。
- 2026-10-17 语言选择存 localStorage `reicky.lang`，窗口间靠 storage 事件同步。

## 坑
- macOS 10.15 的 WebView 是 Safari 13：前端构建 target 用 safari13。
