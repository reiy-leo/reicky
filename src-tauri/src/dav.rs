//! WebDAV 同步：贴纸存到用户自己 WebDAV 服务上的 `<地址>/reicky/` 目录。
//!
//! 每条贴纸一个文件，文件名带修改时间：`reicky-<id>-<modified>.json`。
//! 这样列一次目录（PROPFIND）就知道每条贴纸远端的修改时间，不用额外的索引文件，
//! 两台设备同时写也不会互相覆盖索引。上传新版本后删掉旧文件；删失败也没关系，
//! 下一轮按时间取最新的，再顺手清掉旧的。
//!
//! 登录信息（地址、用户名、密码）存在 app_data/webdav-sync.json，和 Google 的 refresh token 一样只存本机。

use crate::store::Note;
use crate::sync::{agent, http_err, Entry, Remote, Res};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const DIR: &str = "reicky";

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    pub url: String,
    pub user: String,
    pub password: String,
    pub last_sync: Option<u64>,
}

fn saved_path(dir: &Path) -> PathBuf {
    dir.join("webdav-sync.json")
}
pub fn load(dir: &Path) -> Option<Saved> {
    std::fs::read(saved_path(dir))
        .ok()
        .and_then(|b| serde_json::from_slice::<Saved>(&b).ok())
        .filter(|s| !s.url.is_empty())
}
pub fn save(dir: &Path, s: &Saved) -> Res<()> {
    let b = serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(saved_path(dir), b).map_err(|e| e.to_string())
}
pub fn remove(dir: &Path) {
    let _ = std::fs::remove_file(saved_path(dir));
}

/// 状态栏里显示的账号：`用户名@主机`
pub fn label(s: &Saved) -> String {
    let host = url::Url::parse(&s.url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
        .unwrap_or_else(|| s.url.clone());
    if s.user.is_empty() {
        host
    } else {
        format!("{}@{host}", s.user)
    }
}

pub struct Dav {
    /// 以 `/` 结尾的 reicky 目录地址
    base: String,
    auth: String,
}

/// 补全地址：没写协议就当 https，末尾补 `/`
pub fn normalize(url: &str) -> Res<String> {
    let mut u = url.trim().to_string();
    if !u.contains("://") {
        u = format!("https://{u}");
    }
    if !u.ends_with('/') {
        u.push('/');
    }
    let p = url::Url::parse(&u).map_err(|_| "WebDAV 地址格式不对".to_string())?;
    if !matches!(p.scheme(), "http" | "https") {
        return Err("WebDAV 地址要以 http:// 或 https:// 开头".into());
    }
    Ok(u)
}

impl Dav {
    pub fn new(s: &Saved) -> Res<Dav> {
        let root = normalize(&s.url)?;
        Ok(Dav {
            base: format!("{root}{DIR}/"),
            auth: format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", s.user, s.password))
            ),
        })
    }

    #[allow(clippy::result_large_err)] // 只在本文件内部用，直接用 ureq::Error 就行
    fn propfind(&self, url: &str, depth: &str) -> Result<ureq::Response, ureq::Error> {
        agent()
            .request("PROPFIND", url)
            .set("Authorization", &self.auth)
            .set("Depth", depth)
            .set("Content-Type", "application/xml; charset=utf-8")
            .send_string(r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:"><d:prop><d:resourcetype/></d:prop></d:propfind>"#)
    }

    /// 检查账号能用，没有 reicky 目录就建一个
    pub fn check(&self) -> Res<()> {
        let root = self
            .base
            .trim_end_matches('/')
            .trim_end_matches(DIR)
            .to_string();
        self.propfind(&root, "0").map_err(dav_err)?;
        match self.propfind(&self.base, "0") {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(404, _)) => {
                match agent()
                    .request("MKCOL", &self.base)
                    .set("Authorization", &self.auth)
                    .call()
                {
                    // 405 方法不允许：多数服务器表示目录已存在
                    Ok(_) | Err(ureq::Error::Status(405, _)) => Ok(()),
                    Err(e) => Err(dav_err(e)),
                }
            }
            Err(e) => Err(dav_err(e)),
        }
    }

    fn delete(&self, name: &str) {
        let _ = agent()
            .delete(&format!("{}{name}", self.base))
            .set("Authorization", &self.auth)
            .call();
    }
}

/// 从 PROPFIND 返回的 XML 里找出 `reicky-<id>-<modified>.json` 文件名。
/// 不解析 XML：各家服务器的命名空间前缀不一样，但文件名只含字母数字和 `-`，不会被转义，直接扫文本最稳。
fn names_in(xml: &str) -> Vec<(String, u64, String)> {
    let mut out = vec![];
    let mut rest = xml;
    while let Some(i) = rest.find("reicky-") {
        let tail = &rest[i..];
        let end = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '.'))
            .unwrap_or(tail.len());
        let name = &tail[..end];
        if let Some(stem) = name
            .strip_prefix("reicky-")
            .and_then(|n| n.strip_suffix(".json"))
        {
            if let Some((id, m)) = stem.rsplit_once('-') {
                if let Ok(m) = m.parse::<u64>() {
                    if !id.is_empty() {
                        out.push((id.to_string(), m, name.to_string()));
                    }
                }
            }
        }
        rest = &tail[end.max(1)..];
    }
    out
}

fn dav_err(e: ureq::Error) -> String {
    let code = match &e {
        ureq::Error::Status(c, _) => *c,
        _ => 0,
    };
    // 207 多状态是 PROPFIND 的正常返回，不会走到这里
    match code {
        401 => "401 未授权：用户名或密码不对".into(),
        404 => "404 找不到：WebDAV 地址不对".into(),
        405 => "405 方法不允许：这个地址不是 WebDAV 目录".into(),
        507 => "507 存储空间不足".into(),
        _ => http_err(e),
    }
}

impl Remote for Dav {
    fn list(&mut self) -> Res<Vec<Entry>> {
        let xml = self
            .propfind(&self.base, "1")
            .map_err(dav_err)?
            .into_string()
            .map_err(|e| e.to_string())?;
        // 同一条贴纸可能残留多个版本：取最新的，旧的记下来稍后删
        let mut best: HashMap<String, Entry> = HashMap::new();
        let mut seen = std::collections::HashSet::new();
        for (id, m, name) in names_in(&xml) {
            if !seen.insert(name.clone()) {
                continue;
            }
            match best.get_mut(&id) {
                Some(e) if e.modified >= m => e.stale.push(name),
                Some(e) => {
                    let old = std::mem::replace(&mut e.key, name);
                    e.stale.push(old);
                    e.modified = m;
                }
                None => {
                    best.insert(
                        id.clone(),
                        Entry {
                            id,
                            modified: m,
                            key: name,
                            stale: vec![],
                        },
                    );
                }
            }
        }
        // 清理旧版本
        for e in best.values_mut() {
            for s in e.stale.drain(..) {
                self.delete(&s);
            }
        }
        Ok(best.into_values().collect())
    }

    fn put(&mut self, note: &Note, prev: Option<&Entry>) -> Res<()> {
        let name = format!("reicky-{}-{}.json", note.id, note.modified);
        agent()
            .put(&format!("{}{name}", self.base))
            .set("Authorization", &self.auth)
            .set("Content-Type", "application/json")
            .send_string(&serde_json::to_string(note).map_err(|e| e.to_string())?)
            .map_err(dav_err)?;
        if let Some(p) = prev {
            if p.key != name {
                self.delete(&p.key);
            }
        }
        Ok(())
    }

    fn get(&mut self, e: &Entry) -> Res<Note> {
        agent()
            .get(&format!("{}{}", self.base, e.key))
            .set("Authorization", &self.auth)
            .call()
            .map_err(dav_err)?
            .into_json()
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_names() {
        let xml = r#"<d:multistatus xmlns:d="DAV:"><d:response><d:href>/dav/reicky/</d:href></d:response>
<d:response><d:href>/dav/reicky/reicky-6f1c2a3b-1111-4abc-9def-0123456789ab-1729900000000.json</d:href></d:response>
<D:response><D:href>https://x.com/dav/reicky/reicky-6f1c2a3b-1111-4abc-9def-0123456789ab-1729900000500.json</D:href></D:response>
<d:response><d:href>/dav/reicky/other.txt</d:href></d:response></d:multistatus>"#;
        let n = names_in(xml);
        assert_eq!(n.len(), 2);
        assert_eq!(n[0].0, "6f1c2a3b-1111-4abc-9def-0123456789ab");
        assert_eq!(n[1].1, 1729900000500);
    }

    #[test]
    fn normalize_url() {
        assert_eq!(
            normalize("dav.example.com/remote.php/dav").unwrap(),
            "https://dav.example.com/remote.php/dav/"
        );
        assert!(normalize("ftp://x").is_err());
    }
}
