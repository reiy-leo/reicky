# reicky

跨平台桌面便签（macOS / Windows / Linux），像 macOS Stickies：桌面上放一个小编辑器，随手贴文本、拖文件进来记路径。

- 纯文本便签，6 种颜色
- 拖入文件/文件夹只记路径引用，不复制文件
- 笔记只能归档，不能删除
- 底部按钮：颜色/语言、变换形状（瘦高 / 宽扁 / 正方形）、归档、笔记列表
- 界面 9 种语言（中文、English、日本語、Deutsch、Français、Español、Português、Русский、Bahasa Indonesia），默认跟随系统，设置里可以改
- 存储：本机 JSON；可选同步到 Google Drive appDataFolder 或你自己的 WebDAV（二选一，见 docs/WEBDAV_SYNC.md）

## 开发

需要 Node 20+、pnpm、Rust stable，以及 [Tauri 2 的系统依赖](https://v2.tauri.app/start/prerequisites/)。

```bash
pnpm install
pnpm app:dev      # 开发运行
pnpm app:build    # 打包当前平台
```

Intel Mac（x86_64）：`pnpm tauri build --target x86_64-apple-darwin`。

## 打包

GitHub Actions `.github/workflows/build.yml`：推 `v*` tag 或手动触发，产出 macOS universal（Intel + Apple Silicon）、Windows、Linux 安装包到草稿 Release。

## 许可

MIT
