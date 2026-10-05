use crate::{
    models::RuntimeMode,
    runtime::{RuntimeSnapshot, SessionHealth},
    services::ApplicationService,
};
use std::sync::Arc;
use std::sync::Mutex;

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

const MAIN_WINDOW_LABEL: &str = "main";
#[derive(Clone, Debug, Eq, PartialEq)]
struct TrayPresentation {
    title: String,
    checked: Option<RuntimeMode>,
}

impl From<&RuntimeSnapshot> for TrayPresentation {
    fn from(snapshot: &RuntimeSnapshot) -> Self {
        let applied = match snapshot.applied_mode {
            Some(RuntimeMode::Rules) => "规则代理",
            Some(RuntimeMode::Global) => "全局代理",
            Some(RuntimeMode::Direct) => "全局直连",
            None => "未运行",
        };
        let title = if snapshot.session_health == SessionHealth::RecoveryRequired {
            format!("Socks Proxy · {applied} · 恢复未完成（打开主窗口查看）")
        } else if let Some(error) = snapshot
            .last_operation
            .error
            .as_ref()
            .or(snapshot.last_error.as_ref())
        {
            let error: String = error.chars().take(120).collect();
            format!("Socks Proxy · {applied} · {error}（打开主窗口查看）")
        } else {
            format!("Socks Proxy · {applied}")
        };
        Self {
            title,
            checked: snapshot.applied_mode,
        }
    }
}

struct TrayMenuState<R: Runtime> {
    title: MenuItem<R>,
    rules: CheckMenuItem<R>,
    global: CheckMenuItem<R>,
    direct: CheckMenuItem<R>,
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn sync_snapshot<R: Runtime>(app: &AppHandle<R>, snapshot: &RuntimeSnapshot) {
    let display = TrayPresentation::from(snapshot);
    let state = app.state::<Mutex<TrayMenuState<R>>>();
    let Ok(state) = state.lock() else {
        return;
    };
    let _ = state.title.set_text(display.title);
    let _ = state
        .rules
        .set_checked(display.checked == Some(RuntimeMode::Rules));
    let _ = state
        .global
        .set_checked(display.checked == Some(RuntimeMode::Global));
    let _ = state
        .direct
        .set_checked(display.checked == Some(RuntimeMode::Direct));
}

fn request_mode<R: Runtime>(app: &AppHandle<R>, mode: RuntimeMode) {
    let service = app.state::<Arc<ApplicationService>>().inner().clone();
    // A checkable menu can optimistically toggle itself before the callback.
    // Immediately restore the committed backend state while the request runs.
    sync_snapshot(app, &service.runtime_snapshot());
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = service.request_mode(mode);
        let snapshot = service.runtime_snapshot();
        sync_snapshot(&app, &snapshot);
        if result.is_err() {
            show_main_window(&app);
        }
    });
}

fn request_exit<R: Runtime>(app: &AppHandle<R>) {
    let service = app.state::<Arc<ApplicationService>>().inner().clone();
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if service.stop_runtime().is_ok() {
            app.exit(0);
        } else {
            sync_snapshot(&app, &service.runtime_snapshot());
            show_main_window(&app);
        }
    });
}

pub fn install<R: Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
    let title = MenuItem::with_id(
        app,
        "tray-title",
        "Socks Proxy · 未运行",
        false,
        None::<&str>,
    )?;
    let rules = CheckMenuItem::with_id(app, "mode-rules", "规则代理", true, false, None::<&str>)?;
    let global = CheckMenuItem::with_id(app, "mode-global", "全局代理", true, false, None::<&str>)?;
    let direct = CheckMenuItem::with_id(app, "mode-direct", "全局直连", true, false, None::<&str>)?;
    let modes = Submenu::with_id_and_items(
        app,
        "proxy-mode",
        "代理模式",
        true,
        &[&rules, &global, &direct],
    )?;
    let status = MenuItem::with_id(app, "status", "状态", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "退出", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&title, &modes, &separator, &status, &exit])?;
    app.manage(Mutex::new(TrayMenuState {
        title,
        rules,
        global,
        direct,
    }));
    #[cfg(target_os = "macos")]
    let icon = tauri::include_image!("icons/icon-template.png");
    #[cfg(not(target_os = "macos"))]
    let icon = app
        .default_window_icon()
        .expect("application icon is missing")
        .clone();
    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Socks Proxy")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "mode-rules" => request_mode(app, RuntimeMode::Rules),
            "mode-global" => request_mode(app, RuntimeMode::Global),
            "mode-direct" => request_mode(app, RuntimeMode::Direct),
            "status" => show_main_window(app),
            "exit" => request_exit(app),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{RuntimePhase, TrafficCoverage};

    #[test]
    fn tray_never_checks_a_requested_mode_before_backend_commit_or_after_failure() {
        let mut snapshot = RuntimeSnapshot {
            configuration_revision: 0,
            runtime_plan_revision: 0,
            revision: 1,
            selected_mode: RuntimeMode::Rules,
            desired_mode: RuntimeMode::Rules,
            applied_mode: Some(RuntimeMode::Global),
            phase: RuntimePhase::Switching,
            active_profile_id: None,
            runtime_uptime_ms: Some(10),
            system_proxy_enabled: true,
            tun_enabled: false,
            coverage: TrafficCoverage::SystemProxyApps,
            session_health: crate::runtime::SessionHealth::Healthy,
            last_operation: crate::runtime::OperationResult::default(),
            last_error: None,
        };
        assert_eq!(
            TrayPresentation::from(&snapshot).checked,
            Some(RuntimeMode::Global)
        );
        snapshot.phase = RuntimePhase::Failed;
        snapshot.last_error = Some("模拟内核错误".into());
        let failed = TrayPresentation::from(&snapshot);
        assert_eq!(failed.checked, Some(RuntimeMode::Global));
        assert!(failed.title.contains("打开主窗口"));
        assert!(failed.title.contains("全局代理"));
        assert!(failed.title.contains("模拟内核错误"));
        snapshot.session_health = SessionHealth::RecoveryRequired;
        assert!(TrayPresentation::from(&snapshot)
            .title
            .contains("恢复未完成"));
        snapshot.applied_mode = None;
        assert_eq!(TrayPresentation::from(&snapshot).checked, None);
    }
}
