use crate::{commands::StateActivity, state::AppState};
use std::{thread, time::Duration};
use tauri::{Emitter, Manager, PhysicalPosition, WindowEvent};

pub fn install_close_to_hide(app: &tauri::AppHandle) {
    for label in ["pet", "dashboard", "reminder"] {
        if let Some(window) = app.get_webview_window(label) {
            let owned = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = owned.hide();
                }
            });
        }
    }
}

pub fn place_pet_near_bottom_right(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("pet") else {
        return;
    };
    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };
    let area = monitor.work_area();
    let size = window.outer_size().unwrap_or_default();
    let x = area.position.x + area.size.width as i32 - size.width as i32 - 32;
    let y = area.position.y + area.size.height as i32 - size.height as i32 - 24;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

pub fn spawn_auto_walk(app: tauri::AppHandle) {
    thread::spawn(move || {
        let mut direction = -1_i32;
        let mut last_emitted = 0_i32;
        let mut was_paused = false;
        let mut last_horizontal_bounds: Option<(i32, i32)> = None;
        loop {
            thread::sleep(Duration::from_millis(32));
            let Some(window) = app.get_webview_window("pet") else {
                break;
            };
            if !window.is_visible().unwrap_or(false) {
                continue;
            }
            let paused = app
                .state::<AppState>()
                .paused_walk
                .lock()
                .map(|value| !value.is_empty())
                .unwrap_or(true);
            if paused {
                was_paused = true;
                continue;
            }
            let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) else {
                continue;
            };
            if let Ok(Some(monitor)) = window.current_monitor() {
                let area = monitor.work_area();
                last_horizontal_bounds = Some((
                    area.position.x,
                    area.position.x + area.size.width as i32 - size.width as i32,
                ));
            }
            // macOS can briefly return no current monitor for a transparent borderless
            // window exactly on a display edge. Keep using the last valid work area so
            // the pet can reverse direction instead of freezing at that edge.
            let Some((min_x, max_x)) = last_horizontal_bounds else {
                continue;
            };
            let step = physical_walk_step(window.scale_factor().unwrap_or(1.0));
            let mut next_x = position.x + direction * step;
            if next_x <= min_x {
                next_x = min_x;
                direction = 1;
            }
            if next_x >= max_x {
                next_x = max_x;
                direction = -1;
            }
            if should_emit_walk_state(direction, last_emitted, was_paused) {
                let emitted = app
                    .emit(
                        "state-activity",
                        StateActivity {
                            kind: "walk".into(),
                            state_id: Some(
                                if direction < 0 {
                                    "walk-left"
                                } else {
                                    "walk-right"
                                }
                                .into(),
                            ),
                            duration_ms: Some(60_000),
                            feedback: None,
                        },
                    )
                    .is_ok();
                if emitted {
                    last_emitted = direction;
                    was_paused = false;
                }
            }
            let _ = window.set_position(PhysicalPosition::new(next_x, position.y));
        }
    });
}

fn physical_walk_step(scale_factor: f64) -> i32 {
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return 1;
    }
    (scale_factor.ceil() as i32).max(1)
}

fn should_emit_walk_state(direction: i32, last_emitted: i32, resumed: bool) -> bool {
    resumed || direction != last_emitted
}

pub fn spawn_companion_tracker(app: tauri::AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(60));
        let state = app.state::<AppState>();
        if state.persist().is_err() {
            continue;
        }
        if let Ok(stats) = state.public_stats() {
            let _ = app.emit("stats-updated", stats);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{physical_walk_step, should_emit_walk_state};

    #[test]
    fn walk_step_advances_at_least_one_logical_pixel() {
        assert_eq!(physical_walk_step(1.0), 1);
        assert_eq!(physical_walk_step(1.25), 2);
        assert_eq!(physical_walk_step(2.0), 2);
        assert_eq!(physical_walk_step(f64::NAN), 1);
    }

    #[test]
    fn resumed_walk_reemits_an_unchanged_direction() {
        assert!(!should_emit_walk_state(-1, -1, false));
        assert!(should_emit_walk_state(-1, -1, true));
        assert!(should_emit_walk_state(1, -1, false));
    }
}
