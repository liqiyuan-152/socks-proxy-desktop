use std::sync::Mutex;

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

const MAIN_WINDOW_LABEL: &str = "main";
const COMPANY_PROXY_NAME: &str = "公司代理";

#[derive(Clone, Copy)]
enum ProxyMode {
    Rules,
    Global,
    Direct,
}

impl ProxyMode {
    fn label(self) -> &'static str {
        match self {
            Self::Rules => "规则代理",
            Self::Global => "全局代理",
            Self::Direct => "全局直连",
        }
    }

    fn short_label(self) -> &'static str {
        match self {
            Self::Rules => "规则",
            Self::Global => "全局",
            Self::Direct => "直连",
        }
    }
}

struct TrayMenuState {
    title: MenuItem<tauri::Wry>,
    rules_mode: CheckMenuItem<tauri::Wry>,
    global_mode: CheckMenuItem<tauri::Wry>,
    direct_mode: CheckMenuItem<tauri::Wry>,
}

fn show_main_window(app: &AppHandle<tauri::Wry>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn tray_title(mode: ProxyMode) -> String {
    format!(
        "Socks Proxy · {} · {COMPANY_PROXY_NAME}",
        mode.short_label()
    )
}

fn update_proxy_mode(app: &AppHandle<tauri::Wry>, mode: ProxyMode) {
    let tray_menu_state = app.state::<Mutex<TrayMenuState>>();
    let Ok(menu_state) = tray_menu_state.lock() else {
        return;
    };

    let _ = menu_state.title.set_text(tray_title(mode));
    let _ = menu_state
        .rules_mode
        .set_checked(matches!(mode, ProxyMode::Rules));
    let _ = menu_state
        .global_mode
        .set_checked(matches!(mode, ProxyMode::Global));
    let _ = menu_state
        .direct_mode
        .set_checked(matches!(mode, ProxyMode::Direct));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let title = MenuItem::with_id(
                app,
                "tray-title",
                tray_title(ProxyMode::Rules),
                false,
                None::<&str>,
            )?;
            let rules_mode = CheckMenuItem::with_id(
                app,
                "mode-rules",
                ProxyMode::Rules.label(),
                true,
                true,
                None::<&str>,
            )?;
            let global_mode = CheckMenuItem::with_id(
                app,
                "mode-global",
                ProxyMode::Global.label(),
                true,
                false,
                None::<&str>,
            )?;
            let direct_mode = CheckMenuItem::with_id(
                app,
                "mode-direct",
                ProxyMode::Direct.label(),
                true,
                false,
                None::<&str>,
            )?;
            let proxy_mode_menu = Submenu::with_id_and_items(
                app,
                "proxy-mode",
                "代理模式",
                true,
                &[&rules_mode, &global_mode, &direct_mode],
            )?;

            let company_proxy = CheckMenuItem::with_id(
                app,
                "proxy-company",
                COMPANY_PROXY_NAME,
                true,
                true,
                None::<&str>,
            )?;
            let proxy_switch_menu = Submenu::with_id_and_items(
                app,
                "proxy-switch",
                "切换代理",
                true,
                &[&company_proxy],
            )?;

            let status = MenuItem::with_id(app, "status", "状态", true, None::<&str>)?;
            let proxy_management =
                MenuItem::with_id(app, "proxy-management", "代理管理...", false, None::<&str>)?;
            let rule_management =
                MenuItem::with_id(app, "rule-management", "规则管理...", false, None::<&str>)?;
            let connection_log = MenuItem::with_id(
                app,
                "connection-log",
                "查看连接日志...",
                false,
                None::<&str>,
            )?;
            let settings = MenuItem::with_id(app, "settings", "设置...", false, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let exit_separator = PredefinedMenuItem::separator(app)?;
            let exit = MenuItem::with_id(app, "exit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &title,
                    &proxy_mode_menu,
                    &proxy_switch_menu,
                    &separator,
                    &status,
                    &proxy_management,
                    &rule_management,
                    &connection_log,
                    &settings,
                    &exit_separator,
                    &exit,
                ],
            )?;

            app.manage(Mutex::new(TrayMenuState {
                title,
                rules_mode,
                global_mode,
                direct_mode,
            }));

            TrayIconBuilder::with_id("main-tray")
                .icon(
                    app.default_window_icon()
                        .expect("application icon is missing")
                        .clone(),
                )
                .tooltip("Socks Proxy")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "mode-rules" => update_proxy_mode(app, ProxyMode::Rules),
                    "mode-global" => update_proxy_mode(app, ProxyMode::Global),
                    "mode-direct" => update_proxy_mode(app, ProxyMode::Direct),
                    "status" => show_main_window(app),
                    "exit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
