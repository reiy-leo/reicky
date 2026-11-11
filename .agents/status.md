# 灵气贴纸（reicky）项目现状

> 新 session 先读这份，再读 `AGENTS.md`。本文写于 2026-11-02，记的是当时的状态；每次发版、改了功能或定了新规矩，都要回来改这份。

## 1. 项目是什么

跨平台桌面贴纸，像 macOS 自带的「便笺」（Stickies）：桌面上常驻一个小编辑器，随手贴文字，拖文件进来只记路径。给 Rei 自己用，开源（MIT）。

- 平台：macOS（mac 包是 universal，要能在 Rei 的 2018 Intel MacBook Pro / macOS 15 上跑）、Windows、Linux。不做手机。
- 名字：中文叫「灵气贴纸」，其他语言都叫 reicky。包名、productName、identifier（`dev.rei8t.reicky`）、仓库名、可执行文件都不改。
- 叫法：中文里一张张的东西只叫「贴纸」，不用「便签」「笔记」。「macOS 的便笺」指苹果自带应用，保留。
- 三条硬规矩：只存纯文本；拖入文件只记路径、不复制；贴纸**不能删除**，只能归档和恢复。
- 仓库：https://github.com/reiy-leo/reicky（public）
- 介绍页：https://reicky.rei8t.dev（独立仓库 reiy-leo/reicky-web，见第 8 节）

## 2. 版本和分支

| 项 | 现状 |
|---|---|
| 最新正式版 | v0.1.6（commit `1309a7c`，2026-11-12，CI 成功，7 个包） |
| Rei 的 Mac 上装的 | 0.1.6 |
| 分支 | 只有 `main`，没有别的分支和 worktree |
| main 最新 | 见 `git log`；本节下面列的是 v0.1.6 之后没发的 |
| 下个版本号 | 0.1.6（小改）或 0.2.0 |
| tag | v0.1.0 ~ v0.1.6，外加 9 个 `v0.1.0-alpha.*` 预发布 tag |

v0.1.6 之后还没发版的改动（发版时写进 CHANGELOG）：暂无。

发版流程：

1. 等 Rei 明说「发版 / 发布」，不要自己打 tag。
2. 升版本号：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`，再改 `Cargo.lock` 里 reicky 那条。版本号保持纯数字（Windows MSI 要求）；带 `-` 的 tag 会发成 prerelease。
3. 在 `CHANGELOG.md` 顶部加 `## [x.y.z] - 日期` 一节（中文，分新增 / 改进 / 修复）。CI 用 awk 取这一节当 Release 说明。
4. commit，打 `vx.y.z` tag，推送。CI（`.github/workflows/build.yml`）出 mac universal、Windows（exe、msi）、Linux（AppImage、deb、rpm）。
5. Rei 要装到 Mac 时：改 `/workspace/work/inst-reicky.sh` 里的 `V=`，经 tailcat 用 `bash -s` 在 Mac 上跑（下载 `reicky_universal.app.tar.gz`，替换 `/Applications/reicky.app`，去掉隔离标记，再打开）。

## 3. 已完成功能

| 功能 | 说明 | 细节在哪 |
|---|---|---|
| 菜单栏应用 | mac 不进 Dock（Accessory）；左键菜单栏图标弹贴纸列表；Linux 有右键菜单兜底 | `lib.rs` 托盘部分 |
| 贴纸窗口 | 纯文本，输入即存；多窗口；始终置顶；mac 无红绿灯、不进台前调度、跟随所有桌面空间；没有关闭按钮，用「收起来」 | `src/main.ts`、`tauri.macos.conf.json` |
| 颜色 | 12 种，新建时随机；深浅色两套纸色 | `src/api.ts` 与 `store.rs` 的 `COLORS`（两边要一致）、`src/theme.ts` |
| 形状 | 瘦高 240×400 / 宽扁 420×220 / 正方形 320×320 循环 | `src/main.ts` |
| 拖入路径 | 只记路径，显示成小标签；单击打开，⌥/Alt+ 单击在文件管理器里显示 | `src/main.ts` |
| 渲染开关 | 失焦时把链接、列表、待办渲染出来，点回去是原文 | `src/render.ts` |
| 底栏 | 新建、拷贝、粘贴、颜色、形状、图钉置顶、全屏大字、归档、列表、收起来；全图标按钮 + 自绘 tooltip | `src/main.ts`、`src/common.ts` |
| 快捷键 | ⌘/Ctrl+N 新建；⌘/Ctrl+S 弹系统保存框，默认存到「下载/首行.txt」；⌘/Ctrl+L 全屏大字。禁掉右键菜单和刷新、devtools、查找等默认快捷键 | `src/common.ts`、`src/main.ts` |
| 列表面板 | 独立窗口 `list`，失焦自动收起；「贴纸 / 已归档」两页；搜索、按颜色筛选；贴纸页滚动加载；归档页先显示 20 条，点「加载更多」再加 20 条 | `src/list.ts`、`list.html` |
| 归档 | 只归档不删除；已归档的双击只读打开；行尾「恢复」，恢复后切回贴纸页、滚到那条并高亮 | `src/list.ts` |
| 设置页 | 深浅色、菜单栏图标样式、渲染开关、随系统启动、语言、同步方式 | `src/list.ts` 的 `#pane-settings` |
| 9 种语言 | zh en ja de fr es pt ru id，默认跟系统，设置里能改；`en.ts` 定类型，缺键 tsc 报错。Rust 端报错只有中文 | `src/i18n.ts`、`src/locales/` |
| Google Drive 同步 | 回环 + PKCE 登录，`drive.appdata`，每 2 分钟一次；文件 `reicky-<id>.json` + `appProperties app=reicky`，和 navill 共用 GCP 项目但互不干扰；授权不够时显示「重新登录」，登录没勾云端硬盘权限会直接提示 | `src-tauri/src/sync.rs`、`docs/GOOGLE_SYNC.md` |
| WebDAV 同步 | 和 Google 二选一（「同步方式」下拉：不同步 / Google Drive / WebDAV）；文件 `<地址>/reicky/reicky-<id>-<修改时间>.json` | `src-tauri/src/dav.rs`、`docs/WEBDAV_SYNC.md` |
| ydiyr 卡片 | 设置页同步区下面，点开跳 your-data-is-your-responsibility.rei8t.dev | `src/list.ts` |
| MCP 服务 | `reicky --mcp`，stdio；8 个工具：list / get / create / update / append / set_color / archive / restore，没有删除，归档的只读；外部改了文件，窗口 1.5 秒内刷新 | `src-tauri/src/mcp.rs`、README |
| 全局快捷键 | 按下就新建一张贴纸，默认 `Control+Alt+Minus`（⌃⌥-），存在数据目录的 `hotkey` 文件。设置页第一项点按钮录新组合（必须带 Ctrl / Alt / ⌘，Esc 取消，录的时候先停用旧组合），旁边按钮恢复默认；注册不上会提示被别的应用占用。Linux Wayland 下系统不让注册全局快捷键 | `src-tauri/src/hotkey.rs`、`src/list.ts` |
| 调试日志 | `dlog!` 写到数据目录的 `debug.log`（托盘点击、列表显隐、同步起止、DNS 耗时）；网络 DNS 5 秒、连接 8 秒超时 | `src-tauri/src/dbg.rs` |

完整开发历史按时间记在 `notes/history.md`（每次有改动或发版追加一行）。版本说明看 `CHANGELOG.md`。

## 4. 代码地图

```
index.html / list.html   贴纸窗口 / 列表面板的骨架
src/main.ts              贴纸窗口：编辑、自动保存、拖入、底栏、全屏大字、保存文件
src/list.ts              列表面板 + 设置页（同步、语言、图标样式…）
src/common.ts            tooltip、i18n 挂载、禁默认快捷键
src/api.ts               invoke 封装、COLORS 表
src/i18n.ts              语言检测与切换（localStorage reicky.lang）
src/locales/*.ts         9 种语言文案，en.ts 定 Dict 类型
src/render.ts            失焦渲染（链接、列表、待办）
src/theme.ts             深浅色与纸色
src/styles.css           样式，平台差异用 html[data-platform]
src-tauri/src/lib.rs     Tauri 命令（24 个）、窗口、托盘、--mcp 分发、watch_notes 轮询
src-tauri/src/store.rs   本机存储：<数据目录>/notes/<id>.json + state.json，原子写（临时文件 + rename）
src-tauri/src/sync.rs    Google Drive 同步（google-sync.json 存账号）
src-tauri/src/dav.rs     WebDAV 同步（webdav-sync.json 存账号）
src-tauri/src/mcp.rs     MCP stdio 服务
src-tauri/src/dbg.rs     debug.log
src-tauri/src/hotkey.rs  全局快捷键（新建贴纸）
src-tauri/capabilities/default.json  权限
.github/workflows/build.yml          三平台打包，tag v* 触发
```

前端存在 localStorage 的键：`reicky.lang`、`reicky.theme`、`reicky.render`、`reicky.shape.<窗口>`、`reicky.pin.<窗口>`。

mac 数据目录：`~/Library/Application Support/dev.rei8t.reicky/`。

加 Tauri 命令时三处一起改：`generate_handler!` 注册、`src/api.ts` 封装、需要的权限加进 `capabilities/default.json`。

## 5. 开发和验证规矩

- 改完代码 commit 并 push `main`（不触发 CI）。作者 `Rei Leo <reiy.leoz@gmail.com>`，提交说明用中文。
- 提交前跑 `pnpm check`（Biome 格式 + lint，再跑 tsc；`pnpm fix` 自动修），Rust 跑 `cargo fmt`。
- **沙箱里别跑 tauri 的 cargo check / build**：会 OOM，还会把沙箱重置掉。Rust 检查在 LightVela 服务器上跑：`~/reicky` 是只读 https clone，先 `git pull`，再 `cd src-tauri && . ~/.cargo/env && cargo fmt --check && cargo clippy -j2 --all-targets`，增量几秒。
- 沙箱重置后：`sudo -n apt-get install -y openssh-client`（推送要 ssh），`source /workspace/tools/env.sh`。沙箱里 rustfmt 要设 `RUSTUP_HOME=/workspace/tools/rustup CARGO_HOME=/workspace/tools/cargo`。
- 包管理只用 pnpm（pnpm 12，`pnpm-workspace.yaml` 里 `allowBuilds: esbuild: true`）。
- 图标用 Lucide：Rei 点名哪个就用哪个，从 unpkg lucide-static 取路径内联 SVG，线宽 1.8。
- 改界面文案时 9 个语言文件都要改；中文按 zh-writing 清单写，再跑 AutoCorrect。
- 出现 HTTP 状态码时附中文说明，如「403 没有权限」。
- 路径标签保持简洁，不加目录行、滚动或丢失检测（Rei 定）。「收起来」的 tooltip 只写「收起来」。
- 应用功能变了，介绍页 9 个语言文件的文案跟着改。
- 以前出过两个 run 同时写 repo 互相覆盖，动手前先 `git status`。

## 6. 没做完的事

### 等 Rei 动手

- **拖文件闪退**（2026-10-18，alpha.5 时报的）：一直在等崩溃日志。之后 Rei 再没提过，可能已经不出现了。请 Rei 在 0.1.6 上拖一次文件试试；不闪退就关掉这条。
- **断网后点菜单栏不弹列表**（2026-10-24）：加了 debug.log 和超时后 Rei 复现不了。复发时要 mac 上的 `~/Library/Application Support/dev.rei8t.reicky/debug.log`。

### 等 Rei 拍板

- **全局快捷键真机验证**：0.1.6 已装到 Mac，还没在真机上按过 ⌃⌥-、录键、恢复默认。
- **删不删 alpha tag**：2026-10-17 问过 alpha.3 / alpha.4 两个误推的 tag 要不要删，没回。现在一共 9 个 alpha tag，都在 Release 里挂着预发布包。
- **Rust 端报错只有中文**：前端 9 种语言，Rust 返回的错误（同步失败、找不到贴纸等）和菜单栏「贴纸列表 / Notes」还是写死的。要不要也做 i18n？

### 我们能做的

- Rust 改动 `a3682cf`、`6d677c9` 只改了字符串和注释，没在 LightVela 上跑过 fmt / clippy，发版前跑一次。

## 7. 写了但没真机验证的

- 全局快捷键（dbeff3a）：⌃⌥- 能不能新建贴纸，新窗口能不能拿到焦点；设置页录新组合、恢复默认。
- Google 同步在两台设备同时改同一张贴纸时的冲突处理。
- 随系统启动（autostart）。
- 全屏大字在 Windows / Linux 上的效果（mac 上是铺满当前屏，其他平台用 setFullscreen）。
- ⌘/Ctrl+S 保存框在 Windows / Linux 上的效果。
- mac 台前调度、全屏应用上方的置顶表现。
- Windows、Linux 包整体都没人装过，只确认 CI 能出包。
- Google「重新登录」和登录时的权限检查（0.1.5 新加，Rei 还没反馈）。
- 已验证过的：WebDAV 连接、「你的数据你负责」跳转（Rei 10-27 在 Mac 上试过）；MCP 协议在服务器上实测通过。

## 8. 仓库外的东西

| 东西 | 在哪 |
|---|---|
| 本地仓库 | `/workspace/projects/33677e90-52c6-499f-af12-2195bb3949e5/repo`，remote `git@github.com:reiy-leo/reicky.git` |
| 推送密钥 | 项目目录 `.keys/reicky_deploy`（deploy key v2，已配 `core.sshCommand`）。vault 里也存了，但读不回来，本地这份不能删 |
| 介绍页 | 项目目录 `reicky-web/`，仓库 reiy-leo/reicky-web，Worker `reicky-web`，域名 reicky.rei8t.dev。React + Vite + TS、Radix Themes、Tailwind（无 preflight）、lucide-react，9 种语言，下载按钮在浏览器里直接读 GitHub 最新 Release。推送密钥 `/workspace/.keys/reicky-web_deploy`。部署分开跑（一起跑会 OOM）：`npx tsc --noEmit && npx vite build && node scripts/build-inline-assets.mjs && CLOUDFLARE_API_TOKEN=placeholder npx wrangler deploy`。资源内联进 Worker，因为 API Token 下 Static Assets 上传会 401（未授权） |
| Google OAuth | Desktop client，编译时读 `REICKY_GOOGLE_CLIENT_ID` / `REICKY_GOOGLE_CLIENT_SECRET`；CI 用仓库 Actions secrets，本地 dev 读终端环境变量 |
| CI | GitHub Actions，仓库 public，查运行和 Release 不用凭据：`https://api.github.com/repos/reiy-leo/reicky/actions/runs` |
| Rei 的 Mac | tailcat 连接，见 Hark 的 mac-remote 说明；Mac 上没有 reicky 源码 clone，只装了 app |
| LightVela 服务器 | `~/reicky` 只读 clone，装好了 rustup、clippy、rustfmt、webkit 依赖，用来跑 Rust 检查 |
| 装到 Mac 的脚本 | `/workspace/work/inst-reicky.sh` |
| 其他站点 | me.rei8t.dev 项目列表里有 reicky（中文显示「reicky 灵气贴纸」，链到介绍页） |
