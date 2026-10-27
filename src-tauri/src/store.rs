//! 本机存储：每条贴纸一个 JSON 文件（`<app_data>/notes/<id>.json`），
//! 写入先写临时文件再 rename，保证不会写坏。贴纸只能归档，不提供删除。

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const COLORS: &[&str] = &[
    "yellow", "cream", "orange", "coral", "pink", "purple", "blue", "cyan", "mint", "green",
    "white", "gray",
];
pub const DEFAULT_COLOR: &str = "yellow";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub text: String,
    pub color: String,
    /// 拖进来的文件/文件夹，只存绝对路径引用，不复制文件
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub archived: bool,
    /// 毫秒时间戳
    pub created: u64,
    pub modified: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummary {
    pub id: String,
    pub preview: String,
    pub color: String,
    pub archived: bool,
    pub path_count: usize,
    pub modified: u64,
    /// 是否已在某个窗口里开着（由 lib.rs 填）
    pub open: bool,
    /// 是否是主贴纸那条（由 lib.rs 填）
    pub main: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub items: Vec<NoteSummary>,
    pub has_more: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct State {
    current_note_id: Option<String>,
}

pub struct Store {
    root: PathBuf,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

impl Store {
    pub fn open(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(root.join("notes")).map_err(|e| e.to_string())?;
        Ok(Self { root })
    }

    fn note_path(&self, id: &str) -> Result<PathBuf, String> {
        if !valid_id(id) {
            return Err(format!("无效的贴纸 id：{id}"));
        }
        Ok(self.root.join("notes").join(format!("{id}.json")))
    }

    fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }

    fn read_state(&self) -> State {
        fs::read(self.state_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn write_state(&self, s: &State) -> Result<(), String> {
        let b = serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?;
        write_atomic(&self.state_path(), &b)
    }

    pub fn get(&self, id: &str) -> Result<Note, String> {
        let b = fs::read(self.note_path(id)?).map_err(|_| format!("找不到贴纸：{id}"))?;
        serde_json::from_slice(&b).map_err(|e| e.to_string())
    }

    pub fn put(&self, note: &Note) -> Result<(), String> {
        let b = serde_json::to_vec_pretty(note).map_err(|e| e.to_string())?;
        write_atomic(&self.note_path(&note.id)?, &b)
    }

    pub fn create(&self, color: Option<&str>) -> Result<Note, String> {
        let t = now_ms();
        let note = Note {
            id: uuid::Uuid::new_v4().to_string(),
            text: String::new(),
            color: color.unwrap_or(DEFAULT_COLOR).to_string(),
            paths: vec![],
            archived: false,
            created: t,
            modified: t,
        };
        self.put(&note)?;
        Ok(note)
    }

    /// 主编辑窗口显示的贴纸：上次那条；不存在或已归档就新建一条。
    pub fn current(&self) -> Result<Note, String> {
        let st = self.read_state();
        if let Some(id) = st.current_note_id.as_deref() {
            if let Ok(n) = self.get(id) {
                if !n.archived {
                    return Ok(n);
                }
            }
        }
        let n = self.create(None)?;
        self.write_state(&State {
            current_note_id: Some(n.id.clone()),
        })?;
        Ok(n)
    }

    pub fn is_current(&self, id: &str) -> bool {
        self.read_state().current_note_id.as_deref() == Some(id)
    }

    /// 把主窗口换成一条新的空白贴纸（归档当前贴纸后调用）
    pub fn new_current(&self, color: Option<&str>) -> Result<Note, String> {
        let n = self.create(color)?;
        self.write_state(&State {
            current_note_id: Some(n.id.clone()),
        })?;
        Ok(n)
    }

    pub fn update<F: FnOnce(&mut Note)>(&self, id: &str, f: F) -> Result<Note, String> {
        let mut n = self.get(id)?;
        f(&mut n);
        n.modified = now_ms();
        self.put(&n)?;
        Ok(n)
    }

    pub fn all(&self) -> Vec<Note> {
        let Ok(rd) = fs::read_dir(self.root.join("notes")) else {
            return vec![];
        };
        rd.filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .filter_map(|e| fs::read(e.path()).ok())
            .filter_map(|b| serde_json::from_slice::<Note>(&b).ok())
            .collect()
    }

    /// 按 modified 倒序分页；空白且无路径的贴纸不进列表。
    /// query：文字或路径包含（不分大小写）；color：只看这个颜色。
    pub fn list(
        &self,
        offset: usize,
        limit: usize,
        archived: bool,
        query: Option<&str>,
        color: Option<&str>,
    ) -> Page {
        let q = query
            .map(|q| q.trim().to_lowercase())
            .filter(|q| !q.is_empty());
        let mut v: Vec<Note> = self
            .all()
            .into_iter()
            .filter(|n| n.archived == archived)
            .filter(|n| !n.text.trim().is_empty() || !n.paths.is_empty())
            .filter(|n| color.map_or(true, |c| n.color == c))
            .filter(|n| {
                q.as_deref().map_or(true, |q| {
                    n.text.to_lowercase().contains(q)
                        || n.paths.iter().any(|p| p.to_lowercase().contains(q))
                })
            })
            .collect();
        v.sort_by_key(|a| std::cmp::Reverse(a.modified));
        let has_more = v.len() > offset + limit;
        let items = v
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|n| NoteSummary {
                preview: n.text.trim().chars().take(120).collect(),
                id: n.id,
                color: n.color,
                archived: n.archived,
                path_count: n.paths.len(),
                modified: n.modified,
                open: false,
                main: false,
            })
            .collect();
        Page { items, has_more }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store() -> Store {
        let dir = std::env::temp_dir().join(format!("reicky-test-{}", uuid::Uuid::new_v4()));
        Store::open(dir).unwrap()
    }

    #[test]
    fn current_is_stable_and_replaced_after_archive() {
        let s = tmp_store();
        let a = s.current().unwrap();
        assert_eq!(s.current().unwrap().id, a.id);
        s.update(&a.id, |n| n.archived = true).unwrap();
        let b = s.current().unwrap();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn list_sorts_by_modified_desc_and_paginates() {
        let s = tmp_store();
        for i in 0..5 {
            let n = s.create(None).unwrap();
            s.update(&n.id, |n| n.text = format!("note {i}")).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let p = s.list(0, 3, false, None, None);
        assert_eq!(p.items.len(), 3);
        assert!(p.has_more);
        assert_eq!(p.items[0].preview, "note 4");
        let p2 = s.list(3, 3, false, None, None);
        assert_eq!(p2.items.len(), 2);
        assert!(!p2.has_more);
    }

    #[test]
    fn list_filters_by_query_and_color() {
        let s = tmp_store();
        let a = s.create(Some("blue")).unwrap();
        s.update(&a.id, |n| n.text = "Hello World".into()).unwrap();
        let b = s.create(None).unwrap();
        s.update(&b.id, |n| n.paths = vec!["/Users/rei/Report.pdf".into()])
            .unwrap();
        assert_eq!(s.list(0, 10, false, Some("hello"), None).items.len(), 1);
        assert_eq!(s.list(0, 10, false, Some("report"), None).items[0].id, b.id);
        assert_eq!(s.list(0, 10, false, None, Some("blue")).items[0].id, a.id);
        assert_eq!(
            s.list(0, 10, false, Some("hello"), Some("yellow"))
                .items
                .len(),
            0
        );
    }

    #[test]
    fn rejects_path_traversal_ids() {
        let s = tmp_store();
        assert!(s.get("../state").is_err());
    }
}
