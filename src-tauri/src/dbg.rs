//! 调试日志：写到 app_data/debug.log（超过 512KB 先清空），带本机毫秒时间戳和线程名。
//! 用来查「断网后点菜单栏图标不弹列表」这类时序问题；平时开销只是一次 append。

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static PATH: OnceLock<PathBuf> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

pub fn init(dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(dir);
    let _ = PATH.set(dir.join("debug.log"));
    log("==== reicky 启动 ====");
}

pub fn log(msg: impl AsRef<str>) {
    let Some(p) = PATH.get() else { return };
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if std::fs::metadata(p).is_ok_and(|m| m.len() > 512 * 1024) {
        let _ = std::fs::remove_file(p);
    }
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let th = std::thread::current();
    let name = th.name().unwrap_or("?").to_string();
    let line = format!("{ms} [{name}] {}\n", msg.as_ref());
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)
    {
        let _ = f.write_all(line.as_bytes());
    }
    #[cfg(debug_assertions)]
    eprint!("{line}");
}

#[macro_export]
macro_rules! dlog {
    ($($t:tt)*) => { $crate::dbg::log(format!($($t)*)) };
}
