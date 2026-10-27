//! 同步：支持 Google Drive 和 WebDAV（见 dav.rs），两者二选一。合并逻辑两边共用（sync_once）。
//!
//! Google Drive：笔记存到用户 Drive 的 appDataFolder（应用私有、用户网盘里看不见），
//! 每条笔记一个 `reicky-<id>.json`，appProperties.app=reicky 标记归属（appDataFolder 按 GCP 项目共享，可能和 navill 同一个），appProperties.modified 记修改时间。笔记不删除，所以合并就是
//! 按 modified 取新：本地新就上传，远端新就下载。
//!
//! 登录走桌面端 OAuth：本机回环端口 + PKCE。客户端 ID/密钥编译时从环境变量
//! `REICKY_GOOGLE_CLIENT_ID` / `REICKY_GOOGLE_CLIENT_SECRET` 读入（运行时同名环境变量优先，方便本地调试）。
//! 桌面客户端的“密钥”按 Google 的说法本就不算机密，但仍不提交进仓库。

use crate::store::{now_ms, Note};
use crate::AppState;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata openid email";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const DRIVE: &str = "https://www.googleapis.com/drive/v3/files";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3/files";
/// 自动同步间隔
const EVERY: Duration = Duration::from_secs(120);

pub(crate) type Res<T> = Result<T, String>;

fn client() -> Option<(String, String)> {
    let id = std::env::var("REICKY_GOOGLE_CLIENT_ID")
        .ok()
        .or(option_env!("REICKY_GOOGLE_CLIENT_ID").map(String::from))?;
    let secret = std::env::var("REICKY_GOOGLE_CLIENT_SECRET")
        .ok()
        .or(option_env!("REICKY_GOOGLE_CLIENT_SECRET").map(String::from))
        .unwrap_or_default();
    (!id.trim().is_empty()).then(|| (id.trim().to_string(), secret.trim().to_string()))
}

/// 存在 app_data/google-sync.json 的登录信息
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Saved {
    refresh_token: Option<String>,
    email: Option<String>,
    last_sync: Option<u64>,
}

#[derive(Default)]
pub struct Sync {
    /// 同一时间只跑一次同步
    run: Mutex<()>,
    access: Mutex<Option<(String, Instant)>>,
    status: Mutex<Status>,
}

#[derive(Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub configured: bool,
    pub signed_in: bool,
    pub email: Option<String>,
    /// 用 WebDAV 同步时的账号（`用户名@主机`）
    pub dav: Option<String>,
    pub last_sync: Option<u64>,
    pub busy: bool,
    pub error: Option<String>,
}

fn saved_path(dir: &Path) -> PathBuf {
    dir.join("google-sync.json")
}
fn load(dir: &Path) -> Saved {
    std::fs::read(saved_path(dir))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
fn save(dir: &Path, s: &Saved) -> Res<()> {
    let b = serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(saved_path(dir), b).map_err(|e| e.to_string())
}

/// HTTP 状态码带中文说明
fn code_text(code: u16) -> String {
    let zh = match code {
        400 => "请求有误",
        401 => "未授权",
        403 => "没有权限",
        404 => "找不到",
        405 => "方法不允许",
        409 => "冲突",
        423 => "资源被锁定",
        408 => "请求超时",
        429 => "请求太频繁",
        500 => "服务器内部错误",
        502 => "网关错误",
        503 => "服务不可用",
        504 => "网关超时",
        507 => "存储空间不足",
        _ => "请求失败",
    };
    format!("{code} {zh}")
}

pub(crate) fn http_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, r) => {
            let body = r.into_string().unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| {
                    v["error_description"]
                        .as_str()
                        .or(v["error"]["message"].as_str())
                        .or(v["error"].as_str())
                        .map(String::from)
                })
                .unwrap_or_default();
            if msg.is_empty() {
                code_text(code)
            } else {
                format!("{}：{msg}", code_text(code))
            }
        }
        ureq::Error::Transport(t) => format!("网络错误：{t}"),
    }
}

/// 断网/弱网时系统 DNS（getaddrinfo）可能卡很久，ureq 的 timeout 管不到这一步：
/// 放到单独线程里解析，5 秒拿不到就当网络错误
fn resolve(addr: &str) -> std::io::Result<Vec<std::net::SocketAddr>> {
    use std::net::ToSocketAddrs;
    let (tx, rx) = std::sync::mpsc::channel();
    let a = addr.to_string();
    let t0 = Instant::now();
    std::thread::Builder::new()
        .name("reicky-dns".into())
        .spawn(move || {
            let _ = tx.send(a.to_socket_addrs().map(|i| i.collect::<Vec<_>>()));
        })?;
    let r = rx.recv_timeout(Duration::from_secs(5)).unwrap_or_else(|_| {
        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "DNS 解析超时",
        ))
    });
    if r.is_err() || t0.elapsed() > Duration::from_secs(1) {
        crate::dlog!(
            "sync: DNS {addr} 用时 {:?} 结果 {:?}",
            t0.elapsed(),
            r.as_ref().err()
        );
    }
    r
}

pub(crate) fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .resolver(resolve)
        .timeout_connect(Duration::from_secs(8))
        .timeout(Duration::from_secs(30))
        .build()
}

#[derive(Deserialize)]
struct Token {
    access_token: String,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
}

impl Sync {
    pub fn status(&self, dir: &Path) -> Status {
        let s = load(dir);
        let d = crate::dav::load(dir);
        let st = self.status.lock().unwrap();
        let signed_in = s.refresh_token.is_some();
        Status {
            configured: client().is_some(),
            signed_in,
            email: s.email,
            dav: d.as_ref().map(crate::dav::label),
            last_sync: if signed_in {
                s.last_sync
            } else {
                d.and_then(|d| d.last_sync)
            },
            busy: st.busy,
            error: st.error.clone(),
        }
    }

    fn set(&self, app: &AppHandle, busy: bool, error: Option<String>) {
        {
            let mut st = self.status.lock().unwrap();
            st.busy = busy;
            st.error = error;
        }
        let dir = &app.state::<AppState>().data_dir;
        let _ = app.emit("sync-status", self.status(dir));
    }

    /// 用 refresh token 换 access token（有效期内复用）
    fn access_token(&self, dir: &Path) -> Res<String> {
        if let Some((t, until)) = self.access.lock().unwrap().clone() {
            if Instant::now() < until {
                return Ok(t);
            }
        }
        let (id, secret) = client().ok_or("此版本没有配置 Google 客户端")?;
        let rt = load(dir).refresh_token.ok_or("还没登录 Google")?;
        let tok: Token = agent()
            .post(TOKEN_URL)
            .send_form(&[
                ("client_id", &id),
                ("client_secret", &secret),
                ("refresh_token", &rt),
                ("grant_type", "refresh_token"),
            ])
            .map_err(http_err)?
            .into_json()
            .map_err(|e| e.to_string())?;
        let until = Instant::now()
            + Duration::from_secs(tok.expires_in.unwrap_or(3600).saturating_sub(120));
        *self.access.lock().unwrap() = Some((tok.access_token.clone(), until));
        Ok(tok.access_token)
    }
}

// ---------- 登录 ----------

fn query_param(target: &str, key: &str) -> Option<String> {
    let u = url::Url::parse(&format!("http://localhost{target}")).ok()?;
    u.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

/// 打开浏览器登录，等回调（最多 5 分钟），存下 refresh token 和邮箱
pub fn sign_in(app: &AppHandle) -> Res<()> {
    use tauri_plugin_opener::OpenerExt;
    let (id, secret) = client().ok_or("此版本没有配置 Google 客户端")?;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}");
    let verifier = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = uuid::Uuid::new_v4().simple().to_string();
    let mut auth = url::Url::parse(AUTH_URL).unwrap();
    auth.query_pairs_mut()
        .append_pair("client_id", &id)
        .append_pair("redirect_uri", &redirect)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPE)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    app.opener()
        .open_url(auth.as_str(), None::<&str>)
        .map_err(|e| e.to_string())?;

    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(300);
    let code = loop {
        if Instant::now() > deadline {
            return Err("登录超时".into());
        }
        let (mut stream, _) = match listener.accept() {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };
        let _ = stream.set_nonblocking(false);
        let mut line = String::new();
        let _ = BufReader::new(&stream).read_line(&mut line);
        let target = line.split_whitespace().nth(1).unwrap_or("/").to_string();
        let got = query_param(&target, "code")
            .filter(|_| query_param(&target, "state").as_deref() == Some(state.as_str()));
        let err = query_param(&target, "error");
        if got.is_none() && err.is_none() {
            // 浏览器顺带请求的 /favicon.ico 之类
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            continue;
        }
        let ok = got.is_some();
        let html = format!(
            "<!doctype html><meta charset=utf-8><title>reicky</title><body style=\"font:15px -apple-system,system-ui,sans-serif;text-align:center;padding-top:80px\"><h2>{}</h2><p>{}</p>",
            if ok { "已登录 reicky ✓" } else { "登录没有完成" },
            if ok { "可以关闭这个页面，回到 reicky。" } else { "可以关闭这个页面，回到 reicky 重试。" }
        );
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
            html.len()
        );
        match got {
            Some(c) => break c,
            None => return Err(format!("Google 拒绝了登录：{}", err.unwrap_or_default())),
        }
    };

    let tok: Token = agent()
        .post(TOKEN_URL)
        .send_form(&[
            ("client_id", id.as_str()),
            ("client_secret", secret.as_str()),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .map_err(http_err)?
        .into_json()
        .map_err(|e| e.to_string())?;
    let rt = tok.refresh_token.ok_or("Google 没有返回 refresh token")?;
    let email = agent()
        .get("https://openidconnect.googleapis.com/v1/userinfo")
        .set("Authorization", &format!("Bearer {}", tok.access_token))
        .call()
        .ok()
        .and_then(|r| r.into_json::<serde_json::Value>().ok())
        .and_then(|v| v["email"].as_str().map(String::from));

    let st = app.state::<AppState>();
    let until =
        Instant::now() + Duration::from_secs(tok.expires_in.unwrap_or(3600).saturating_sub(120));
    *st.sync.access.lock().unwrap() = Some((tok.access_token, until));
    save(
        &st.data_dir,
        &Saved {
            refresh_token: Some(rt),
            email,
            last_sync: None,
        },
    )?;
    // 两种同步二选一：登录 Google 就关掉 WebDAV
    crate::dav::remove(&st.data_dir);
    Ok(())
}

pub fn sign_out(app: &AppHandle) -> Res<()> {
    let st = app.state::<AppState>();
    let _guard = st.sync.run.lock().unwrap();
    // 顺手吊销授权；失败也照样退出
    if let Some(rt) = load(&st.data_dir).refresh_token {
        let _ = agent()
            .post("https://oauth2.googleapis.com/revoke")
            .send_form(&[("token", &rt)]);
    }
    *st.sync.access.lock().unwrap() = None;
    let _ = std::fs::remove_file(saved_path(&st.data_dir));
    st.sync.set(app, false, None);
    Ok(())
}

// ---------- 同步 ----------

/// 远端的一条笔记
pub(crate) struct Entry {
    pub id: String,
    pub modified: u64,
    /// Google：文件 ID；WebDAV：文件名
    pub key: String,
    /// WebDAV 残留的旧版本文件名
    pub stale: Vec<String>,
}

/// 远端存储：Google Drive 或 WebDAV
pub(crate) trait Remote {
    fn list(&mut self) -> Res<Vec<Entry>>;
    /// 上传笔记；prev 是远端已有的那份
    fn put(&mut self, note: &Note, prev: Option<&Entry>) -> Res<()>;
    fn get(&mut self, e: &Entry) -> Res<Note>;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteFile {
    id: String,
    name: String,
    #[serde(default)]
    app_properties: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileList {
    #[serde(default)]
    files: Vec<RemoteFile>,
    next_page_token: Option<String>,
}

struct Drive {
    token: String,
}

impl Remote for Drive {
    fn list(&mut self) -> Res<Vec<Entry>> {
        let mut out = vec![];
        let mut page: Option<String> = None;
        loop {
            let mut req = agent()
                .get(DRIVE)
                .set("Authorization", &format!("Bearer {}", self.token))
                .query("spaces", "appDataFolder")
                // appDataFolder 按 GCP 项目共享（和 navill 同项目），只看 reicky 自己的文件
                .query(
                    "q",
                    "appProperties has { key='app' and value='reicky' } and trashed = false",
                )
                .query("pageSize", "1000")
                .query("fields", "nextPageToken,files(id,name,appProperties)");
            if let Some(p) = &page {
                req = req.query("pageToken", p);
            }
            let l: FileList = req
                .call()
                .map_err(http_err)?
                .into_json()
                .map_err(|e| e.to_string())?;
            for f in l.files {
                if let Some(id) = f
                    .name
                    .strip_prefix("reicky-")
                    .and_then(|n| n.strip_suffix(".json"))
                {
                    let modified = f
                        .app_properties
                        .get("modified")
                        .and_then(|m| m.parse::<u64>().ok())
                        .unwrap_or(0);
                    out.push(Entry {
                        id: id.to_string(),
                        modified,
                        key: f.id,
                        stale: vec![],
                    });
                }
            }
            match l.next_page_token {
                Some(p) => page = Some(p),
                None => return Ok(out),
            }
        }
    }

    fn put(&mut self, note: &Note, prev: Option<&Entry>) -> Res<()> {
        let boundary = format!("reicky-{}", uuid::Uuid::new_v4().simple());
        let mut meta = serde_json::json!({ "appProperties": { "app": "reicky", "modified": note.modified.to_string() } });
        if prev.is_none() {
            meta["name"] = format!("reicky-{}.json", note.id).into();
            meta["parents"] = serde_json::json!(["appDataFolder"]);
        }
        let body = format!(
            "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n--{boundary}\r\nContent-Type: application/json\r\n\r\n{}\r\n--{boundary}--",
            serde_json::to_string(note).map_err(|e| e.to_string())?
        );
        let req = match prev {
            Some(p) => agent().request("PATCH", &format!("{UPLOAD}/{}", p.key)),
            None => agent().post(UPLOAD),
        };
        req.query("uploadType", "multipart")
            .set("Authorization", &format!("Bearer {}", self.token))
            .set(
                "Content-Type",
                &format!("multipart/related; boundary={boundary}"),
            )
            .send_string(&body)
            .map_err(http_err)?;
        Ok(())
    }

    fn get(&mut self, e: &Entry) -> Res<Note> {
        agent()
            .get(&format!("{DRIVE}/{}", e.key))
            .query("alt", "media")
            .set("Authorization", &format!("Bearer {}", self.token))
            .call()
            .map_err(http_err)?
            .into_json()
            .map_err(|e| e.to_string())
    }
}

/// 合并：笔记不删除，所以按修改时间取新——本地新就上传，远端新就下载
fn merge(app: &AppHandle, remote: &mut dyn Remote) -> Res<()> {
    let st = app.state::<AppState>();
    let entries = remote.list()?;
    let local: HashMap<String, Note> = st
        .store
        .lock()
        .unwrap()
        .all()
        .into_iter()
        .map(|n| (n.id.clone(), n))
        .collect();
    let by_id: HashMap<String, &Entry> = entries.iter().map(|e| (e.id.clone(), e)).collect();
    // 本地有：比修改时间
    for (id, n) in &local {
        // 空白笔记不上传（主便签新建时就是空的）
        let blank = n.text.trim().is_empty() && n.paths.is_empty();
        match by_id.get(id).copied() {
            None if blank => {}
            None => remote.put(n, None)?,
            Some(e) if n.modified > e.modified => remote.put(n, Some(e))?,
            Some(e) if e.modified > n.modified => pull(app, remote, e)?,
            Some(_) => {}
        }
    }
    // 只在远端有：下载
    for (id, e) in &by_id {
        if !local.contains_key(id) {
            pull(app, remote, e)?;
        }
    }
    Ok(())
}

fn sync_once(app: &AppHandle) -> Res<()> {
    let st = app.state::<AppState>();
    let dir = &st.data_dir;
    if load(dir).refresh_token.is_some() {
        let token = st.sync.access_token(dir)?;
        merge(app, &mut Drive { token })?;
        let mut s = load(dir);
        s.last_sync = Some(now_ms());
        save(dir, &s)
    } else if let Some(d) = crate::dav::load(dir) {
        merge(app, &mut crate::dav::Dav::new(&d)?)?;
        // 重新读一次：同步期间可能刚改过配置
        if let Some(mut d) = crate::dav::load(dir) {
            d.last_sync = Some(now_ms());
            crate::dav::save(dir, &d)?;
        }
        Ok(())
    } else {
        Ok(())
    }
}

fn pull(app: &AppHandle, remote: &mut dyn Remote, e: &Entry) -> Res<()> {
    let n = remote.get(e)?;
    let st = app.state::<AppState>();
    {
        let store = st.store.lock().unwrap();
        // 下载期间本地又改了就不覆盖，留给下一轮上传
        if let Ok(cur) = store.get(&n.id) {
            if cur.modified >= n.modified {
                return Ok(());
            }
        }
        store.put(&n)?;
    }
    let _ = app.emit("note-changed", &n);
    Ok(())
}

fn enabled(dir: &Path) -> bool {
    (client().is_some() && load(dir).refresh_token.is_some()) || crate::dav::load(dir).is_some()
}

/// 跑一次同步（已在跑就跳过），结果通过 sync-status 事件告诉前端
pub fn run(app: &AppHandle) {
    let st = app.state::<AppState>();
    if !enabled(&st.data_dir) {
        return;
    }
    let Ok(_guard) = st.sync.run.try_lock() else {
        crate::dlog!("sync: 上一轮还在跑，跳过");
        return;
    };
    let t0 = Instant::now();
    crate::dlog!("sync: 开始");
    st.sync.set(app, true, None);
    let r = sync_once(app);
    crate::dlog!(
        "sync: 结束 用时 {:?} 结果 {:?}",
        t0.elapsed(),
        r.as_ref().err()
    );
    if r.is_err() {
        // access token 可能被吊销了，下次重新换
        *st.sync.access.lock().unwrap() = None;
    }
    st.sync.set(app, false, r.err());
    let _ = app.emit("notes-synced", ());
}

/// 连上 WebDAV：先检查账号、建目录，通过才保存；会退出 Google
pub fn dav_connect(app: &AppHandle, url: &str, user: &str, password: &str) -> Res<()> {
    let saved = crate::dav::Saved {
        url: crate::dav::normalize(url)?,
        user: user.trim().to_string(),
        password: password.to_string(),
        last_sync: None,
    };
    crate::dav::Dav::new(&saved)?.check()?;
    let st = app.state::<AppState>();
    if load(&st.data_dir).refresh_token.is_some() {
        sign_out(app)?;
    }
    let _guard = st.sync.run.lock().unwrap();
    crate::dav::save(&st.data_dir, &saved)?;
    drop(_guard);
    st.sync.set(app, false, None);
    Ok(())
}

pub fn dav_disconnect(app: &AppHandle) -> Res<()> {
    let st = app.state::<AppState>();
    let _guard = st.sync.run.lock().unwrap();
    crate::dav::remove(&st.data_dir);
    drop(_guard);
    st.sync.set(app, false, None);
    Ok(())
}

/// 启动后台定时同步
pub fn start(app: &AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new()
        .name("reicky-sync".into())
        .spawn(move || {
            std::thread::sleep(Duration::from_secs(5));
            loop {
                run(&app);
                std::thread::sleep(EVERY);
            }
        });
}
