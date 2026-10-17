mod store;

use std::sync::Mutex;
use store::{Note, Page, Store, COLORS};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

struct AppState {
    store: Mutex<Store>,
}

type Res<T> = Result<T, String>;

/// 通知所有窗口某条笔记变了（主窗口与独立小窗口可能显示同一条）
fn broadcast(app: &AppHandle, note: &Note) {
    let _ = app.emit("note-changed", note);
}

#[tauri::command]
fn get_current_note(state: State<AppState>) -> Res<Note> {
    state.store.lock().unwrap().current()
}

#[tauri::command]
fn get_note(state: State<AppState>, id: String) -> Res<Note> {
    state.store.lock().unwrap().get(&id)
}

#[tauri::command]
fn save_text(app: AppHandle, state: State<AppState>, id: String, text: String) -> Res<Note> {
    let n = state.store.lock().unwrap().update(&id, |n| n.text = text)?;
    broadcast(&app, &n);
    Ok(n)
}

#[tauri::command]
fn set_color(app: AppHandle, state: State<AppState>, id: String, color: String) -> Res<Note> {
    if !COLORS.contains(&color.as_str()) {
        return Err(format!("不支持的颜色：{color}"));
    }
    let n = state.store.lock().unwrap().update(&id, |n| n.color = color)?;
    broadcast(&app, &n);
    Ok(n)
}

#[tauri::command]
fn add_paths(app: AppHandle, state: State<AppState>, id: String, paths: Vec<String>) -> Res<Note> {
    let n = state.store.lock().unwrap().update(&id, |n| {
        for p in paths {
            if !n.paths.contains(&p) {
                n.paths.push(p);
            }
        }
    })?;
    broadcast(&app, &n);
    Ok(n)
}

#[tauri::command]
fn remove_path(app: AppHandle, state: State<AppState>, id: String, path: String) -> Res<Note> {
    let n = state.store.lock().unwrap().update(&id, |n| n.paths.retain(|p| p != &path))?;
    broadcast(&app, &n);
    Ok(n)
}

/// 归档（唯一的“删除”方式）。若归档的是主窗口当前笔记，返回替代它的新空白笔记。
#[tauri::command]
fn archive_note(app: AppHandle, state: State<AppState>, id: String) -> Res<Option<Note>> {
    let store = state.store.lock().unwrap();
    let n = store.update(&id, |n| n.archived = true)?;
    broadcast(&app, &n);
    if store.is_current(&id) {
        let fresh = store.new_current(Some(&n.color))?;
        let _ = app.emit("current-replaced", &fresh);
        return Ok(Some(fresh));
    }
    Ok(None)
}

#[tauri::command]
fn unarchive_note(app: AppHandle, state: State<AppState>, id: String) -> Res<Note> {
    let n = state.store.lock().unwrap().update(&id, |n| n.archived = false)?;
    broadcast(&app, &n);
    Ok(n)
}

#[tauri::command]
fn list_notes(state: State<AppState>, offset: usize, limit: usize, archived: bool) -> Res<Page> {
    Ok(state.store.lock().unwrap().list(offset, limit.min(100), archived))
}

/// 在独立小窗口打开某条笔记；已开着就聚焦
#[tauri::command]
fn open_note_window(app: AppHandle, id: String) -> Res<()> {
    let label = format!("note-{id}");
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.unminimize();
        let _ = w.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, label, WebviewUrl::App(format!("index.html?note={id}").into()))
        .title("reicky")
        .inner_size(300.0, 300.0)
        .min_inner_size(220.0, 180.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// 菜单栏图标左键：主窗口没在前台就显示并打开笔记列表；已在前台就收起
fn toggle_list(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else { return };
    let front = w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false);
    if front {
        let _ = w.hide();
    } else {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.emit("open-list", ());
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    // macOS 用黑白模板图标，跟随菜单栏明暗；其他平台用应用图标
    #[cfg(target_os = "macos")]
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
    #[cfg(not(target_os = "macos"))]
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    let builder = TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("reicky")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_list(tray.app_handle());
            }
        });

    // Linux 的托盘（AppIndicator）收不到点击事件，只能挂菜单兜底
    #[cfg(target_os = "linux")]
    let builder = {
        use tauri::menu::{Menu, MenuItem};
        let show = MenuItem::with_id(app, "list", "笔记列表 / Notes", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", "退出 / Quit", true, None::<&str>)?;
        builder
            .menu(&Menu::with_items(app, &[&show, &quit])?)
            .on_menu_event(|app, e| match e.id().as_ref() {
                "list" => toggle_list(app),
                "quit" => app.exit(0),
                _ => {}
            })
    };

    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            let store = Store::open(dir)?;
            app.manage(AppState { store: Mutex::new(store) });
            // 不在 Dock 显示（打包后 Info.plist 的 LSUIElement 也会生效，这里保证 dev 模式一致）
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            setup_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // 没有 Dock 图标，关主窗口只是收起，从菜单栏图标再唤出
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_current_note,
            get_note,
            save_text,
            set_color,
            add_paths,
            remove_path,
            archive_note,
            unarchive_note,
            list_notes,
            open_note_window,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("启动 reicky 失败");
}
