//! 全局快捷键：在哪都能按，按下就新建一张贴纸。
//! 组合键存在数据目录的 hotkey 文件里（如 `Control+Alt+Minus`），设置页可以改，也可以恢复默认。

use std::path::Path;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

/// 默认 ⌃⌥-（Windows / Linux 上是 Ctrl+Alt+-）
pub const DEFAULT: &str = "Control+Alt+Minus";
const FILE: &str = "hotkey";

fn parse(s: &str) -> Result<Shortcut, String> {
    s.parse::<Shortcut>()
        .map_err(|e| format!("快捷键格式不对：{e}"))
}

/// 存下来的组合键；没存过或读出来不合法就用默认
pub fn saved(dir: &Path) -> String {
    std::fs::read_to_string(dir.join(FILE))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| parse(s).is_ok())
        .unwrap_or_else(|| DEFAULT.into())
}

/// 当前组合有没有注册上（被别的应用占了就是 false）
pub fn active(app: &AppHandle, dir: &Path) -> bool {
    parse(&saved(dir))
        .map(|k| app.global_shortcut().is_registered(k))
        .unwrap_or(false)
}

/// 注册存下来的组合；注册不上只记日志，设置页会提示
pub fn register_saved(app: &AppHandle, dir: &Path) {
    let s = saved(dir);
    let r = parse(&s).and_then(|k| app.global_shortcut().register(k).map_err(|e| e.to_string()));
    if let Err(e) = r {
        crate::dlog!("hotkey: 注册 {s} 失败：{e}");
    }
}

/// 换成新组合：先注销旧的；新的注册不上就把旧的恢复，不保存
pub fn set(app: &AppHandle, dir: &Path, accel: &str) -> Result<(), String> {
    let next = parse(accel)?;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if let Err(e) = gs.register(next) {
        register_saved(app, dir);
        return Err(format!("这个组合注册不上，可能被别的应用占用了（{e}）"));
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(FILE), accel).map_err(|e| e.to_string())
}

/// 设置页录新组合时先停用，免得按到旧组合直接新建了贴纸；录完再恢复
pub fn suspend(app: &AppHandle, dir: &Path, on: bool) {
    let _ = app.global_shortcut().unregister_all();
    if !on {
        register_saved(app, dir);
    }
}
