mod store;

use std::sync::Mutex;
use std::time::{Duration, Instant};
use store::{Note, Page, Store, COLORS};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

struct AppState {
    store: Mutex<Store>,
    /// 列表面板上次因失焦收起的时间：点菜单栏图标时失焦先触发，避免“刚收起又弹出”
    list_hidden_at: Mutex<Option<Instant>>,
}

const LIST_W: f64 = 320.0;
const LIST_H: f64 = 480.0;

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
fn list_notes(
    app: AppHandle,
    state: State<AppState>,
    offset: usize,
    limit: usize,
    archived: bool,
    query: Option<String>,
    color: Option<String>,
) -> Res<Page> {
    let store = state.store.lock().unwrap();
    let mut page = store.list(offset, limit.min(100), archived, query.as_deref(), color.as_deref());
    let main_visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    for it in &mut page.items {
        it.open = app.get_webview_window(&format!("note-{}", it.id)).is_some()
            || (main_visible && store.is_current(&it.id));
        it.main = store.is_current(&it.id);
    }
    Ok(page)
}

/// 便签窗口：始终置顶；macOS 上不进台前调度、跟随所有桌面空间、全屏应用上方也能看到，
/// 并藏掉红绿灯（mac 用「透明标题栏 overlay」来保留原生圆角和阴影，所以要手动藏按钮）
fn pin_window(w: &tauri::WebviewWindow) {
    let _ = w.set_always_on_top(true);
    #[cfg(target_os = "macos")]
    {
        let w2 = w.clone();
        let _ = w.run_on_main_thread(move || {
            use objc2::msg_send;
            use objc2::runtime::AnyObject;
            // NSWindowCollectionBehavior
            const CAN_JOIN_ALL_SPACES: usize = 1 << 0;
            const STATIONARY: usize = 1 << 4;
            const FULL_SCREEN_AUXILIARY: usize = 1 << 8;
            const AUXILIARY: usize = 1 << 17; // 不单独成为台前调度的一组
            const CLEAR: usize = (1 << 1) | (1 << 2) | (1 << 3) | (1 << 7) | (1 << 16);
            let Ok(ptr) = w2.ns_window() else { return };
            let ns = ptr as *mut AnyObject;
            if ns.is_null() {
                return;
            }
            unsafe {
                let cur: usize = msg_send![ns, collectionBehavior];
                let bits = (cur & !CLEAR) | CAN_JOIN_ALL_SPACES | STATIONARY | FULL_SCREEN_AUXILIARY | AUXILIARY;
                let _: () = msg_send![ns, setCollectionBehavior: bits];
                // NSWindowButton: close=0, miniaturize=1, zoom=2
                for kind in 0usize..3 {
                    let btn: *mut AnyObject = msg_send![ns, standardWindowButton: kind];
                    if !btn.is_null() {
                        let _: () = msg_send![btn, setHidden: true];
                    }
                }
            }
        });
    }
}

/// macOS：保留系统窗框但标题栏透明叠在内容上 → 原生圆角 + 阴影（红绿灯在 pin_window 里藏掉）；其他平台无边框
fn chrome<'a, R: tauri::Runtime, M: Manager<R>>(b: WebviewWindowBuilder<'a, R, M>) -> WebviewWindowBuilder<'a, R, M> {
    #[cfg(target_os = "macos")]
    return b
        .decorations(true)
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    #[cfg(not(target_os = "macos"))]
    return b.decorations(false);
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn build_note_window(app: &AppHandle, id: &str, pos: Option<(f64, f64)>) -> Res<()> {
    // 主便签那条在主窗口里，不另开
    if app.state::<AppState>().store.lock().unwrap().is_current(id) {
        show_main(app);
        return Ok(());
    }
    let label = format!("note-{id}");
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }
    let mut b = chrome(
        WebviewWindowBuilder::new(app, label, WebviewUrl::App(format!("index.html?note={id}").into()))
            .title("reicky")
            .inner_size(300.0, 300.0)
            .min_inner_size(220.0, 180.0)
            .always_on_top(true),
    );
    if let Some((x, y)) = pos {
        b = b.position(x, y);
    }
    let w = b.build().map_err(|e| e.to_string())?;
    pin_window(&w);
    let _ = w.set_focus();
    Ok(())
}

/// 在独立小窗口打开某条笔记；已开着就聚焦
#[tauri::command]
// 建窗口必须用 async 命令，Windows 上同步命令里建窗口会死锁
async fn open_note_window(app: AppHandle, id: String) -> Res<()> {
    build_note_window(&app, &id, None)
}

/// 新建一张便签并在独立小窗口打开，位置相对调用它的窗口错开一点，可连续开多个
#[tauri::command]
async fn new_note_window(
    app: AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    color: Option<String>,
) -> Res<Note> {
    // 不再沿用当前颜色：随机挑一个，避开正开着的便签已用的颜色（都用过了就挑用得最少的）
    let _ = color;
    let store = state.store.lock().unwrap();
    let mut open_colors: Vec<String> = app
        .webview_windows()
        .keys()
        .filter_map(|label| label.strip_prefix("note-"))
        .filter_map(|id| store.get(id).ok())
        .map(|n| n.color)
        .collect();
    if app.get_webview_window("main").and_then(|w| w.is_visible().ok()).unwrap_or(false) {
        if let Ok(n) = store.current() {
            open_colors.push(n.color);
        }
    }
    let pick = pick_color(&open_colors, uuid::Uuid::new_v4().as_bytes()[0] as usize);
    let n = store.create(Some(pick))?;
    drop(store);
    let pos = match (window.outer_position(), window.scale_factor()) {
        (Ok(p), Ok(sf)) => {
            let p = p.to_logical::<f64>(sf);
            Some((p.x + 28.0, p.y + 28.0))
        }
        _ => None,
    };
    build_note_window(&app, &n.id, pos)?;
    Ok(n)
}

/// 在用得最少的颜色里随机挑一个（没开着的颜色优先）
fn pick_color(used: &[String], rnd: usize) -> &'static str {
    let count = |c: &str| used.iter().filter(|u| u.as_str() == c).count();
    let min = COLORS.iter().map(|c| count(c)).min().unwrap_or(0);
    let pool: Vec<&'static str> = COLORS.iter().copied().filter(|c| count(c) == min).collect();
    pool[rnd % pool.len()]
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// 把列表面板放到 (x, y) 附近并显示；位置夹在所在显示器的可用区域内
fn place_list(app: &AppHandle, anchor: (f64, f64), below: bool) {
    let Some(w) = app.get_webview_window("list") else { return };
    let sf = w.scale_factor().unwrap_or(1.0);
    let (lw, lh) = (LIST_W * sf, LIST_H * sf);
    let (mut x, mut y) = (anchor.0 - lw / 2.0, if below { anchor.1 } else { anchor.1 - lh });
    let mon = app
        .monitor_from_point(anchor.0, anchor.1)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    if let Some(m) = mon {
        let wa = m.work_area();
        let (l, t) = (wa.position.x as f64, wa.position.y as f64);
        let (r, b) = (l + wa.size.width as f64, t + wa.size.height as f64);
        x = x.clamp(l + 8.0, (r - lw - 8.0).max(l));
        y = y.clamp(t + 4.0, (b - lh - 4.0).max(t));
    }
    let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = w.show();
    let _ = w.set_focus();
    let _ = w.emit("list-shown", ());
}

/// 菜单栏图标左键：列表面板从图标下方弹出（任务栏在底部时从上方弹出）；再点一下收起
fn toggle_list_at_tray(app: &AppHandle, rect: tauri::Rect) {
    let Some(w) = app.get_webview_window("list") else { return };
    let just_hidden = app
        .state::<AppState>()
        .list_hidden_at
        .lock()
        .unwrap()
        .map_or(false, |t| t.elapsed() < Duration::from_millis(300));
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    if just_hidden {
        return;
    }
    let p = rect.position.to_physical::<f64>(1.0);
    let sz = rect.size.to_physical::<f64>(1.0);
    let cx = p.x + sz.width / 2.0;
    let bottom_bar = app
        .monitor_from_point(cx, p.y)
        .ok()
        .flatten()
        .map_or(false, |m| p.y > (m.position().y as f64) + (m.size().height as f64) / 2.0);
    if bottom_bar {
        place_list(app, (cx, p.y - 6.0), false);
    } else {
        place_list(app, (cx, p.y + sz.height + 4.0), true);
    }
}

/// 便签底栏的列表按钮：面板贴着这个便签旁边弹出
#[tauri::command]
fn show_list(app: AppHandle, window: tauri::WebviewWindow) {
    let (Ok(p), Ok(s)) = (window.outer_position(), window.outer_size()) else {
        return place_list(&app, (400.0, 200.0), true);
    };
    let sf = window.scale_factor().unwrap_or(1.0);
    let lw = LIST_W * sf;
    let right = p.x as f64 + s.width as f64 + 8.0 + lw / 2.0;
    let left = p.x as f64 - 8.0 - lw / 2.0;
    let fits_right = app
        .monitor_from_point(p.x as f64, p.y as f64)
        .ok()
        .flatten()
        .map_or(true, |m| right + lw / 2.0 <= (m.position().x as f64) + (m.size().width as f64));
    place_list(&app, (if fits_right { right } else { left }, p.y as f64), true);
}

/// 列表里点「主便签」那条，或 Linux 托盘菜单
#[tauri::command]
fn show_main_window(app: AppHandle) {
    show_main(&app);
}

fn build_list_window(app: &tauri::App) -> tauri::Result<()> {
    let w = chrome(
        WebviewWindowBuilder::new(app, "list", WebviewUrl::App("list.html".into()))
            .title("reicky")
            .inner_size(LIST_W, LIST_H)
            .min_inner_size(260.0, 300.0)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false),
    )
    .build()?;
    pin_window(&w);
    Ok(())
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
                rect,
                ..
            } = event
            {
                toggle_list_at_tray(tray.app_handle(), rect);
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
                // 拿不到图标位置：放到主显示器右上角
                "list" => place_list(app, (1.0e6, 0.0), true),
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
        // 窗口状态只记位置/大小等，不恢复 decorations/可见性（旧版本存下的“有标题栏”会把新配置盖掉）
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::all()
                        & !tauri_plugin_window_state::StateFlags::DECORATIONS
                        & !tauri_plugin_window_state::StateFlags::VISIBLE,
                )
                // 列表面板每次跟着图标/便签定位，不记位置
                .with_denylist(&["list"])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            let store = Store::open(dir)?;
            app.manage(AppState { store: Mutex::new(store), list_hidden_at: Mutex::new(None) });
            // 不在 Dock 显示（打包后 Info.plist 的 LSUIElement 也会生效，这里保证 dev 模式一致）
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            build_list_window(app)?;
            setup_tray(app)?;
            if let Some(w) = app.get_webview_window("main") {
                pin_window(&w);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // 没有 Dock 图标：关主窗口只是收起；列表面板也只隐藏不销毁
            WindowEvent::CloseRequested { api, .. } if matches!(window.label(), "main" | "list") => {
                api.prevent_close();
                let _ = window.hide();
            }
            // 列表面板像菜单栏弹出面板：点到别处就收起
            WindowEvent::Focused(false) if window.label() == "list" => {
                if window.is_visible().unwrap_or(false) {
                    let _ = window.hide();
                    *window.app_handle().state::<AppState>().list_hidden_at.lock().unwrap() = Some(Instant::now());
                }
            }
            _ => {}
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
            new_note_window,
            show_list,
            show_main_window,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("启动 reicky 失败");
}
