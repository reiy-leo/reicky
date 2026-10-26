# WebDAV 同步

不想用 Google，可以把笔记同步到自己的 WebDAV 服务（坚果云、Nextcloud、群晖、自建 Apache/Nginx 等）。WebDAV 和 Google 二选一：连上一个，另一个自动关掉。

## 怎么开

1. 打开列表面板，点齿轮进设置。
2. 打开「用 WebDAV 同步」，填地址、用户名、密码，点「连接」。
3. reicky 先检查账号能不能用，再在这个地址下建 `reicky/` 目录。检查通过才保存，失败会显示原因（比如「401 未授权：用户名或密码不对」）。

地址没写 `http://` 就按 `https://` 处理。坚果云这类服务建议用「应用密码」，别填登录密码。

常见地址：

| 服务 | 地址 |
| --- | --- |
| 坚果云 | `https://dav.jianguoyun.com/dav/` |
| Nextcloud | `https://你的域名/remote.php/dav/files/用户名/` |
| 群晖 | `https://你的域名:5006/` |

## 存法

- 每条笔记一个文件：`reicky/reicky-<笔记 ID>-<修改时间>.json`。
- 文件名里带修改时间，所以列一次目录就知道远端每条笔记新不新，不用额外的索引文件；两台设备同时写也不会互相覆盖。
- 上传新版本后删掉旧文件。删失败也不要紧，下一轮按时间取最新的，再清掉旧的。
- 合并规则和 Google 一样：笔记不删除，本地新就上传，远端新就下载。后台每 2 分钟同步一次，设置里可以「立即同步」。

## 账号存在哪

地址、用户名、密码存在应用数据目录的 `webdav-sync.json`，只在本机，不同步、不上传。关掉「用 WebDAV 同步」就删掉这个文件。

macOS 上的路径：`~/Library/Application Support/dev.rei8t.reicky/webdav-sync.json`
