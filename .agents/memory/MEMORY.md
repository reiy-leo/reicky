# MEMORY

## 决策
- 2026-10-17 选 Tauri 2：包小、三平台、Rust 后端便于做同步。
- 2026-10-17 每条贴纸单独一个 JSON 文件，便于后续按文件做 Drive / WebDAV 增量同步。
- 2026-10-17 形状切换按窗口记在 localStorage（`reicky.shape.<window label>`），尺寸：瘦高 240×400、宽扁 420×220、正方形 320×320。
- 2026-10-17 语言选择存 localStorage `reicky.lang`，窗口间靠 storage 事件同步。

## 坑
- macOS 10.15 的 WebView 是 Safari 13：前端构建 target 用 safari13。
- 沙箱里跑 tauri 的 cargo check / build 会 OOM 并重置沙箱；Rust 检查去 LightVela 服务器跑（见 .agents/status.md 第 5 节）。
- 2026-10-27 `.dav-form` 等元素的 display 规则盖掉了 `[hidden]`：全局加 `[hidden]{display:none!important}`。
- 2026-10-27 opener 只有 `allow-open-url` 没有 URL 范围时打开网址全被拒，要加 `opener:allow-default-urls`。

