# AGENTS.md — reicky

给在本仓库工作的 AI agent / 开发者看的总说明。产品细节见 [docs/PRODUCT.md](docs/PRODUCT.md)。

## 目标
跨平台桌面便签（macOS / Windows / Linux），类似 macOS Stickies。桌面指定位置放一个小编辑器，方便全局复制粘贴零碎文本、拖文件记路径。

## 边界（不做）
- 不做移动端（iOS / Android）
- 暂不做富文本、图片、Markdown 渲染（先只支持纯文本）
- 不复制拖入的文件，只存路径
- **不提供删除笔记**，只能归档 / 恢复

## 技术栈
- Tauri 2（Rust）+ TypeScript + Vite，原生 DOM，无框架
- 插件：window-state（记住窗口位置尺寸）、opener（打开路径 / 在文件管理器显示）
- 界面语言：9 种（`src/i18n.ts` 管切换，文案在 `src/locales/*.ts`，以 `en.ts` 的键为准；默认跟随系统，设置页「语言」下拉可改）。加文案时 9 个文件都要加，缺键 tsc 报错
- 必须能在 x86_64 macOS（2018 Intel MacBook Pro，macOS 15）跑；最低 macOS 10.15，前端构建 target safari13

## 目录
```
index.html            界面骨架（主窗口和笔记小窗口共用；小窗口 URL 带 ?note=<id>）
src/main.ts           交互：编辑自动保存、拖入路径、颜色、形状、归档、底部列表
src/api.ts            调 Rust 命令的类型化封装
src/i18n.ts           语言检测与切换
src/locales/*.ts      各语言文案（en.ts 定义 Dict 类型）
src/styles.css        便签颜色 × 平台观感（html[data-platform] / html[data-color]）
src-tauri/src/lib.rs  Tauri 命令、窗口
src-tauri/src/store.rs 本机存储：<app_data>/notes/<id>.json + state.json，原子写
.github/workflows/    三平台打包
.agents/              skills / rules / prompts / memory
```

## 命令
```bash
pnpm install
pnpm typecheck
pnpm build                     # 仅前端
pnpm app:dev                   # 运行桌面应用
pnpm app:build                 # 打包
cd src-tauri && cargo test     # 存储层单测
```

## 代码规范
- TypeScript strict；UI 文案一律走 `i18n.ts`，不要在 main.ts 里写死中文/英文
- 新增 Tauri 命令要同时：注册到 `generate_handler!`、加到 `src/api.ts`、需要的权限加进 `capabilities/default.json`
- 存储写入一律走 `Store::put`（临时文件 + rename）
- 平台差异用 CSS 变量 + `[data-platform]`，不要在 TS 里分支样式
- 出现 HTTP 状态码时附中文说明，例如 `408 请求超时`

## 禁止事项
- 不要加任何删除笔记的入口或命令
- 不要把真密钥、OAuth secret、WebDAV 密码提交进仓库
- 不要复制用户拖入的文件

## 工作约定
- 每轮对话完成后 commit 一次并 push 到 main
- 不要自己打 `v*` tag 发版本（会触发三平台打包）；等 Rei 明确说「推版本」再打
- 产品决策变化同步更新 docs/PRODUCT.md；经验与坑写进 `.agents/memory/MEMORY.md`
