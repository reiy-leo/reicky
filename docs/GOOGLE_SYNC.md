# Google 同步配置

reicky 把贴纸同步到用户 Google Drive 的 **appDataFolder**（应用私有空间，网盘里看不见），每条贴纸一个 `reicky-<id>.json`（appProperties `app=reicky`；appDataFolder 按 GCP 项目共享，和同项目的 navill 共用，reicky 只读写带这个标记的文件），按修改时间取新（贴纸不删除，所以没有删除冲突）。登录走桌面端 OAuth（本机回环端口 + PKCE），refresh token 存在应用数据目录 `google-sync.json`。后台每 2 分钟同步一次，设置页可「立即同步」。

## 1. Google Cloud 里建桌面客户端（复用已有项目）

1. 打开 https://console.cloud.google.com/ ，选中要复用的项目。
2. **APIs & Services → Library**：搜 *Google Drive API*，启用。
3. **OAuth consent screen**（Google Auth Platform → Data access）：添加 scope `.../auth/drive.appdata`（以及 `openid`、`email`）。应用仍在 *Testing* 状态时，把自己的 Gmail 加到 Test users。
4. **Credentials → Create credentials → OAuth client ID**，类型选 **Desktop app**，名字填 `reicky`。
5. 记下 Client ID 和 Client secret。

> 桌面客户端不需要配置回调地址，任意 `http://127.0.0.1:<端口>` 都允许。

## 2. 交给构建

- **CI 发版**：仓库 Settings → Secrets and variables → Actions，新建
  `REICKY_GOOGLE_CLIENT_ID`、`REICKY_GOOGLE_CLIENT_SECRET`。
- **本地调试**：
  ```bash
  export REICKY_GOOGLE_CLIENT_ID=xxxx.apps.googleusercontent.com
  export REICKY_GOOGLE_CLIENT_SECRET=GOCSPX-xxxx
  pnpm app:dev
  ```

没配置时设置页的「用 Google 同步」是灰的，提示“这个版本没有配置 Google 客户端”。
