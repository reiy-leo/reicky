# 灵气贴纸 reicky

中文名「灵气贴纸」，其他语言都叫 reicky。介绍页：https://reicky.rei8t.dev

跨平台桌面便签（macOS / Windows / Linux），像 macOS Stickies：桌面上放一个小编辑器，随手贴文本、拖文件进来记路径。

- 纯文本便签，6 种颜色
- 拖入文件/文件夹只记路径引用，不复制文件
- 笔记只能归档，不能删除
- 底部按钮：颜色/语言、变换形状（瘦高 / 宽扁 / 正方形）、归档、笔记列表
- 界面 9 种语言（中文、English、日本語、Deutsch、Français、Español、Português、Русский、Bahasa Indonesia），默认跟随系统，设置里可以改
- 存储：本机 JSON；可选同步到 Google Drive appDataFolder 或你自己的 WebDAV（二选一，见 docs/WEBDAV_SYNC.md）

## 让 AI 操作便签（MCP）

reicky 自带一个 MCP 服务，用 `--mcp` 参数启动，不开窗口，走标准输入输出。AI 能列出、搜索、读、新建、改写、追加、改颜色、归档和恢复便签。没有删除，跟应用里一样只能归档。应用开着的时候，AI 改完 1~2 秒内窗口就会刷新。

macOS 上在 MCP 客户端里这样配：

```json
{
  "mcpServers": {
    "reicky": {
      "command": "/Applications/reicky.app/Contents/MacOS/reicky",
      "args": ["--mcp"]
    }
  }
}
```

Windows 和 Linux 把 `command` 换成 reicky 可执行文件的路径就行。

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
