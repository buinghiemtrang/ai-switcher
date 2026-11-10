//! Native tray menu for quota summaries and quick account access.
//!
//! The native menu renders text + a checkmark (no custom bars/badges). The active
//! account uses a real native checkmark (CheckMenuItem, kept enabled so it isn't greyed
//! out); every row trails its quota + plan as `· 96% · Plus`. Tools are grouped under a
//! disabled header with separators between them.
//!
//! Claude Code, Codex and Antigravity match the main application navigation.
//! Rebuilt from a fresh snapshot whenever the app state changes via `rebuild`.

use crate::app_state::ManagedState;
use crate::models::{Account, AccountState, AppSnapshot, SwitchAccountInput, ToolId, ToolStatus};
use tauri::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_notification::NotificationExt;

/// Keep the native menu in the same order as the visible application tools.
const TRAY_TOOLS: [ToolId; 3] = [ToolId::Claude, ToolId::Codex, ToolId::Antigravity];

fn tray_tools(tools: &[ToolStatus]) -> Vec<&ToolStatus> {
    TRAY_TOOLS.iter().filter_map(|id| tools.iter().find(|tool| tool.id == *id)).collect()
}

const OPEN_ACCOUNT_PREFIX: &str = "account:open:";
const SWITCH_PREFIX: &str = "switch:";
const REFRESH_PREFIX: &str = "refresh:";
const OVERLAY_ID: &str = "tray:overlay";
const OPEN_ID: &str = "tray:open";
const QUIT_ID: &str = "tray:quit";

/// Creates the tray icon once at startup.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;
    let snapshot = app.state::<ManagedState>().snapshot().ok();
    let menu = build_menu(app, snapshot.as_ref())?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        // Monochrome template — macOS recolours it per theme (white in dark menu bar) and
        // brightens it when the menu is open, matching the system icons (wifi/clock).
        .icon_as_template(true)
        .tooltip("AI switcher")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event)
        .build(app)?;
    Ok(())
}

/// Rebuilds the menu from the current snapshot so checkmarks + quota stay in sync.
pub fn rebuild(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main-tray") else {
        return;
    };
    let snapshot = app.state::<ManagedState>().snapshot().ok();
    if let Ok(menu) = build_menu(app, snapshot.as_ref()) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn build_menu(app: &AppHandle, snapshot: Option<&AppSnapshot>) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;

    if let Some(snapshot) = snapshot {
        let mut any_tool = false;
        let mut tools_in_menu = Vec::new();
        for tool in tray_tools(&snapshot.tools) {
            tools_in_menu.push(tool.id.clone());
            if any_tool {
                menu.append(&PredefinedMenuItem::separator(app)?)?;
            }
            any_tool = true;
            append_tool_section(app, &menu, tool)?;
        }
        if any_tool {
            menu.append(&PredefinedMenuItem::separator(app)?)?;
            append_refresh_section(app, &menu, &tools_in_menu)?;
            menu.append(&PredefinedMenuItem::separator(app)?)?;
        }
    }

    // Floating quota overlay on/off, checked when the window is up.
    let overlay_on = app
        .state::<ManagedState>()
        .overlay_settings()
        .map(|settings| settings.enabled)
        .unwrap_or(false);
    menu.append(&CheckMenuItem::with_id(
        app,
        OVERLAY_ID,
        "All quotas window",
        true,
        overlay_on,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        OPEN_ID,
        "Open AI Switcher…",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        QUIT_ID,
        "Quit",
        true,
        Some("Cmd+Q"),
    )?)?;
    Ok(menu)
}

/// Parse the `<tool>` part of a tray menu id back into the tool it names.
fn tray_tool_from_str(value: &str) -> Option<ToolId> {
    TRAY_TOOLS
        .iter()
        .find(|tool_id| tool_id.as_str() == value)
        .cloned()
}

fn append_refresh_section(
    app: &AppHandle,
    menu: &Menu<Wry>,
    tools_in_menu: &[ToolId],
) -> tauri::Result<()> {
    let refresh_menu = Submenu::with_id(app, "refresh:menu", "Refresh Quotas", true)?;

    for tool_id in TRAY_TOOLS {
        if !tools_in_menu.contains(&tool_id) {
            continue;
        }
        refresh_menu.append(&MenuItem::with_id(
            app,
            format!("{REFRESH_PREFIX}{}", tool_id.as_str()),
            tool_id.display_name(),
            true,
            None::<&str>,
        )?)?;
    }
    if tools_in_menu.len() > 1 {
        refresh_menu.append(&PredefinedMenuItem::separator(app)?)?;
        refresh_menu.append(&MenuItem::with_id(
            app,
            "refresh:all",
            "All",
            true,
            None::<&str>,
        )?)?;
    }

    menu.append(&refresh_menu)
}

fn append_tool_section(app: &AppHandle, menu: &Menu<Wry>, tool: &ToolStatus) -> tauri::Result<()> {
    // Bold-ish tool header as a disabled row.
    menu.append(&MenuItem::with_id(
        app,
        format!("header:{}", tool.id.as_str()),
        tool.name.clone(),
        false,
        None::<&str>,
    )?)?;

    if tool.accounts.iter().all(|account| account.hidden) {
        menu.append(&MenuItem::with_id(
            app,
            format!("empty:{}", tool.id.as_str()),
            if tool.installed { "   No saved accounts" } else { "   No saved accounts — setup in app" },
            false,
            None::<&str>,
        )?)?;
        return Ok(());
    }

    for account in &tool.accounts {
        if account.hidden {
            continue;
        }
        let is_active = Some(account.id.as_str()) == tool.active_account_id.as_deref()
            || account.state == AccountState::Active;
        let needs_login = account.state == AccountState::NeedsLogin;
        let locked = account.is_locked();
        // Opening an Antigravity account from the tray must not stop a live IDE.
        // Explicit switching remains available in the main window.
        let open_in_app = tool.id == ToolId::Antigravity || !tool.installed;
        let prefix = if open_in_app { OPEN_ACCOUNT_PREFIX } else { SWITCH_PREFIX };
        let id = format!("{prefix}{}:{}", tool.id.as_str(), account.id);
        let label = if open_in_app {
            format!("{}  ·  Open in app", account_label(account))
        } else {
            account_label(account)
        };

        if is_active {
            // Native checkmark for the account in use. Kept ENABLED so it renders in the
            // normal (not greyed-out) text colour; clicking it just re-selects the same
            // account, which is a harmless no-op.
            menu.append(&CheckMenuItem::with_id(
                app,
                id,
                label,
                tool.installed || open_in_app,
                true,
                None::<&str>,
            )?)?;
        } else {
            menu.append(&MenuItem::with_id(
                app,
                id,
                label,
                open_in_app || (tool.installed && !needs_login && !locked),
                None::<&str>,
            )?)?;
        }
    }
    Ok(())
}

/// `Work  ·  96% · Plus` — name trailed by quota + plan (no status emoji).
fn account_label(account: &Account) -> String {
    let mut trailer: Vec<String> = Vec::new();
    if let Some(quota) = &account.quota {
        if account.tool_id == ToolId::Antigravity {
            if let Some(window) = quota.models.as_ref().and_then(|models| models.iter()
                .filter(|window| window.percent_used.is_some())
                .max_by(|a, b| a.percent_used.unwrap_or(0.0).total_cmp(&b.percent_used.unwrap_or(0.0)))) {
                trailer.push(format!("{} {}%", window.label, window.percent_used.unwrap_or(0.0).round() as i64));
            }
        }
        if trailer.is_empty() {
            if let Some(percent) = quota.five_hour.percent_used {
                trailer.push(format!("5h {}%", percent.round() as i64));
            }
            if let Some(percent) = quota.weekly.percent_used {
                trailer.push(format!("7d {}%", percent.round() as i64));
            }
        }
        if trailer.is_empty() {
            if let Some(models) = &quota.models {
                if let Some(window) = models.iter().find(|window| window.percent_used.is_some()) {
                    trailer.push(format!("{} {}%", window.label, window.percent_used.unwrap().round() as i64));
                }
            }
        }
        if trailer.is_empty() { trailer.push("Quota unavailable".to_string()); }
        if let Some(plan) = &quota.plan {
            trailer.push(plan.clone());
        }
    } else if account.api_provider.is_some() {
        trailer.push("API".to_string());
    } else {
        trailer.push("Quota unavailable".to_string());
    }

    if account.is_locked() {
        trailer.push("Locked".to_string());
    }

    if trailer.is_empty() {
        account.name.clone()
    } else {
        format!("{}  ·  {}", account.name, trailer.join(" · "))
    }
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id();
    match id.as_ref() {
        OPEN_ID => show_main_window(app),
        other if other.starts_with(OPEN_ACCOUNT_PREFIX) => show_main_window(app),
        OVERLAY_ID => toggle_overlay(app),
        QUIT_ID => app.exit(0),
        other if other.starts_with(SWITCH_PREFIX) => switch_from_id(app, id),
        other if other.starts_with(REFRESH_PREFIX) => refresh_from_id(app, id),
        _ => {}
    }
}

/// Parses `switch:<tool>:<accountId>` and switches on a worker thread (switching shells
/// out to the CLI, which must not block the menu event handler).
fn switch_from_id(app: &AppHandle, id: &MenuId) {
    let rest = &id.as_ref()[SWITCH_PREFIX.len()..];
    let Some((tool_str, account_id)) = rest.split_once(':') else {
        return;
    };
    let Some(tool_id) = tray_tool_from_str(tool_str) else {
        return;
    };
    let account_id = account_id.to_string();
    let app = app.clone();
    std::thread::spawn(move || {
        let result = app
            .state::<ManagedState>()
            .switch_account(SwitchAccountInput {
                tool_id,
                account_id,
            });
        if let Ok(snapshot) = result {
            let _ = app.emit("snapshot-changed", &snapshot);
        }
        rebuild(&app);
    });
}

/// Parses `refresh:<tool|all>` and refreshes quota on a worker thread so the native
/// menu event handler stays responsive while quota endpoints are queried.
fn refresh_from_id(app: &AppHandle, id: &MenuId) {
    let target = &id.as_ref()[REFRESH_PREFIX.len()..];
    let tools: Vec<ToolId> = if target == "all" {
        TRAY_TOOLS.to_vec()
    } else {
        match tray_tool_from_str(target) {
            Some(tool_id) => vec![tool_id],
            None => return,
        }
    };
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<ManagedState>();
        let mut latest_snapshot = None;
        let mut failed = false;

        for tool_id in tools {
            match state.refresh_tool(tool_id, Some(&app)) {
                Ok(snapshot) => latest_snapshot = Some(snapshot),
                Err(_) => failed = true,
            }
        }

        if let Some(snapshot) = latest_snapshot {
            let _ = app.emit("snapshot-changed", &snapshot);
        }
        if failed {
            let _ = app
                .notification()
                .builder()
                .title("Refresh failed")
                .body("Could not refresh one or more quota reports.")
                .show();
        }
        rebuild(&app);
    });
}

/// Tray toggle for the floating quota overlay. Persists the new state so it survives a restart,
/// then rebuilds the menu so the checkmark matches.
fn toggle_overlay(app: &AppHandle) {
    let state = app.state::<ManagedState>();
    let enabled = state
        .overlay_settings()
        .map(|settings| settings.enabled)
        .unwrap_or(false);
    if let Ok(settings) = state.set_overlay_enabled(!enabled) {
        crate::overlay::apply(app, &settings);
    }
    rebuild(app);
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(id: ToolId, installed: bool) -> ToolStatus {
        ToolStatus { name: id.display_name().to_string(), id, installed, active_account_id: None, accounts: Vec::new() }
    }

    #[test]
    fn tray_matches_visible_tools_even_when_cli_is_not_installed() {
        let tools = vec![tool(ToolId::Cursor, true), tool(ToolId::Codex, true),
            tool(ToolId::Opencode, true), tool(ToolId::Antigravity, false), tool(ToolId::Claude, false)];
        let selected: Vec<_> = tray_tools(&tools).into_iter().map(|tool| tool.id.clone()).collect();
        assert_eq!(selected, vec![ToolId::Claude, ToolId::Codex, ToolId::Antigravity]);
        assert!(tray_tool_from_str("cursor").is_none());
        assert!(tray_tool_from_str("opencode").is_none());
        assert_eq!(tray_tool_from_str("antigravity"), Some(ToolId::Antigravity));
    }

    #[test]
    fn tray_missing_tools_are_not_invented() {
        let tools = vec![tool(ToolId::Codex, false)];
        assert_eq!(tray_tools(&tools).len(), 1);
        assert_eq!(tray_tools(&tools)[0].id, ToolId::Codex);
    }

    #[test]
    fn account_summary_keeps_unknown_separate_from_zero_and_shows_both_windows() {
        let mut account: Account = serde_json::from_value(serde_json::json!({
            "id": "fixture", "toolId": "codex", "name": "Work", "state": "idle",
            "fingerprint": "", "createdAt": "", "updatedAt": "", "lastUsedAt": null,
            "quota": null
        })).unwrap();
        assert!(account_label(&account).contains("Quota unavailable"));
        assert!(!account_label(&account).contains("0%"));
        let mut quota = crate::models::QuotaInfo::with_message("fixture");
        quota.error = None;
        quota.five_hour.percent_used = Some(0.0);
        quota.weekly.percent_used = Some(15.0);
        account.quota = Some(quota);
        let label = account_label(&account);
        assert!(label.contains("5h 0%"));
        assert!(label.contains("7d 15%"));
    }

}
