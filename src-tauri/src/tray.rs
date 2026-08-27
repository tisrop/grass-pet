use crate::{commands, context_menu, state::AppState};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

pub fn install(app: &tauri::App) -> tauri::Result<()> {
    let click_through_enabled = app
        .state::<AppState>()
        .data
        .lock()
        .map(|data| data.settings.click_through)
        .unwrap_or(false);
    let show_pet = MenuItem::with_id(app, "show-pet", "显示桌宠", true, None::<&str>)?;
    let summon_pet = MenuItem::with_id(app, "summon-pet", "再召唤一个阿飘", true, None::<&str>)?;
    let dashboard = MenuItem::with_id(app, "dashboard", "打开道观", true, None::<&str>)?;
    let reminder = MenuItem::with_id(app, "reminder", "添加提醒", true, None::<&str>)?;
    let click_through = MenuItem::with_id(
        app,
        "toggle-click-through",
        if click_through_enabled {
            "关闭鼠标穿透"
        } else {
            "开启鼠标穿透"
        },
        true,
        None::<&str>,
    )?;
    let hide_pet = MenuItem::with_id(app, "hide-pet", "隐藏桌宠", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show_pet,
            &summon_pet,
            &dashboard,
            &reminder,
            &click_through,
            &hide_pet,
            &quit,
        ],
    )?;
    let icon = Image::from_bytes(include_bytes!("../../src/assets/tray/tray-icon.png"))?;
    let click_through_item = click_through.clone();

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("阿飘道长桌宠")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show-pet" => {
                if let Some(window) = app.get_webview_window("pet") {
                    let _ = window.show();
                }
            }
            "summon-pet" => {
                let _ = commands::spawn_new_pet_process();
            }
            "dashboard" => {
                let _ = commands::show_window(app, "dashboard");
            }
            "reminder" => {
                let _ = commands::show_reminder(app);
            }
            "toggle-click-through" => {
                if let Ok(enabled) = context_menu::toggle_click_through(app) {
                    let _ = click_through_item.set_text(if enabled {
                        "关闭鼠标穿透"
                    } else {
                        "开启鼠标穿透"
                    });
                }
            }
            "hide-pet" => {
                if let Some(window) = app.get_webview_window("pet") {
                    let _ = window.hide();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
