use crate::{commands, geometry::Rect, state::AppState};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

const WINDOW_LABEL: &str = "context-menu";
const PAUSE_REASON: &str = "context-menu";

pub fn popup(app: &AppHandle, pet: &WebviewWindow, state: &AppState) -> Result<(), String> {
    let menu = app
        .get_webview_window(WINDOW_LABEL)
        .ok_or("context menu window is unavailable")?;
    let pet_position = pet.outer_position().map_err(|error| error.to_string())?;
    let pet_size = pet.outer_size().map_err(|error| error.to_string())?;
    let menu_size = menu.outer_size().map_err(|error| error.to_string())?;
    let monitor = pet
        .current_monitor()
        .map_err(|error| error.to_string())?
        .ok_or("pet window is not on a monitor")?;
    let area = monitor.work_area();
    let scale = pet.scale_factor().unwrap_or(1.0);
    let gap = (8.0 * scale).round().max(1.0) as i32;
    let position = context_menu_position(
        Rect {
            x: pet_position.x,
            y: pet_position.y,
            width: pet_size.width,
            height: pet_size.height,
        },
        Rect {
            x: 0,
            y: 0,
            width: menu_size.width,
            height: menu_size.height,
        },
        Rect {
            x: area.position.x,
            y: area.position.y,
            width: area.size.width,
            height: area.size.height,
        },
        gap,
    );

    menu.set_position(PhysicalPosition::new(position.x, position.y))
        .map_err(|error| error.to_string())?;
    state
        .paused_walk
        .lock()
        .map_err(|_| "walk lock is poisoned")?
        .insert(PAUSE_REASON.into());
    if let Err(error) = menu.show().and_then(|_| menu.set_focus()) {
        clear_pause(state);
        return Err(error.to_string());
    }
    Ok(())
}

pub fn hide(app: &AppHandle, state: &AppState) -> Result<(), String> {
    clear_pause(state);
    app.get_webview_window(WINDOW_LABEL)
        .ok_or("context menu window is unavailable")?
        .hide()
        .map_err(|error| error.to_string())
}

pub fn clear_pause(state: &AppState) {
    if let Ok(mut reasons) = state.paused_walk.lock() {
        reasons.remove(PAUSE_REASON);
    }
}

pub fn toggle_click_through(app: &AppHandle) -> Result<bool, String> {
    let state = app.state::<AppState>();
    let settings = {
        let mut data = state.data.lock().map_err(|_| "state lock is poisoned")?;
        data.settings.click_through = !data.settings.click_through;
        data.settings.clone()
    };
    commands::apply_pet_settings(app, &settings)?;
    state.persist()?;
    Ok(settings.click_through)
}

fn context_menu_position(
    pet: Rect,
    menu: Rect,
    work_area: Rect,
    gap: i32,
) -> PhysicalPosition<i32> {
    let work_right = work_area.x + work_area.width as i32;
    let work_bottom = work_area.y + work_area.height as i32;
    let right_x = pet.x + pet.width as i32 + gap;
    let left_x = pet.x - menu.width as i32 - gap;
    let x = if right_x + menu.width as i32 <= work_right {
        right_x
    } else if left_x >= work_area.x {
        left_x
    } else {
        right_x.clamp(work_area.x, work_right - menu.width as i32)
    };
    let bottom_aligned_y = pet.y + pet.height as i32 - menu.height as i32;
    let y = bottom_aligned_y.clamp(work_area.y, work_bottom - menu.height as i32);
    PhysicalPosition::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::context_menu_position;
    use crate::geometry::Rect;

    const AREA: Rect = Rect {
        x: 0,
        y: 25,
        width: 1440,
        height: 875,
    };
    const MENU: Rect = Rect {
        x: 0,
        y: 0,
        width: 154,
        height: 158,
    };

    #[test]
    fn places_menu_left_of_a_pet_near_the_right_edge() {
        let position = context_menu_position(
            Rect {
                x: 1248,
                y: 628,
                width: 192,
                height: 272,
            },
            MENU,
            AREA,
            8,
        );
        assert_eq!(position.x, 1086);
        assert_eq!(position.y, 742);
    }

    #[test]
    fn places_menu_right_of_a_pet_when_space_is_available() {
        let position = context_menu_position(
            Rect {
                x: 200,
                y: 400,
                width: 192,
                height: 272,
            },
            MENU,
            AREA,
            8,
        );
        assert_eq!(position.x, 400);
        assert_eq!(position.y, 514);
    }
}
