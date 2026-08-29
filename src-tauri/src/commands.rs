use crate::{
    context_menu,
    geometry::{snap_rect, Rect},
    state::{AppState, DragSession, PetStats, Reminder, Settings},
};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    process::{Child, Command, Stdio},
};
use tauri::{Emitter, Manager, State, WebviewWindow};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    edge_snap: Option<bool>,
    always_on_top: Option<bool>,
    click_through: Option<bool>,
    pet_scale: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderInput {
    text: String,
    due_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionSpec {
    pub id: String,
    pub emoji: String,
    pub label: String,
    pub state_id: String,
    pub duration_ms: u64,
    pub affection_gain: i64,
    pub feedback: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionResult {
    interaction: InteractionSpec,
    feedback: String,
    stats: PetStats,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateActivity {
    pub kind: String,
    pub state_id: Option<String>,
    pub duration_ms: Option<u64>,
    pub feedback: Option<String>,
}

pub(crate) fn spawn_new_pet_process() -> Result<Child, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut command = Command::new(executable);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(feature = "e2e")]
    command.env("WDIO_EMBEDDED_PORT", "45555");
    command.spawn().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn summon_new_pet(state: State<'_, AppState>) -> Result<u32, String> {
    let child = spawn_new_pet_process()?;
    state.track_spawned_pet(child)
}

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Result<Settings, String> {
    Ok(state
        .data
        .lock()
        .map_err(|_| "state lock is poisoned")?
        .settings
        .clone())
}

#[tauri::command]
pub fn settings_update(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<Settings, String> {
    let settings = {
        let mut data = state.data.lock().map_err(|_| "state lock is poisoned")?;
        if let Some(value) = patch.edge_snap {
            data.settings.edge_snap = value;
        }
        if let Some(value) = patch.always_on_top {
            data.settings.always_on_top = value;
        }
        if let Some(value) = patch.click_through {
            data.settings.click_through = value;
        }
        if let Some(value) = patch.pet_scale {
            if !value.is_finite() {
                return Err("petScale must be finite".into());
            }
            data.settings.pet_scale = value.clamp(0.65, 1.0);
        }
        data.settings.clone()
    };
    apply_pet_settings(&app, &settings)?;
    state.persist()?;
    Ok(settings)
}

pub fn apply_pet_settings(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or("pet window is unavailable")?;
    pet.set_always_on_top(settings.always_on_top)
        .map_err(|error| error.to_string())?;
    pet.set_ignore_cursor_events(settings.click_through)
        .map_err(|error| error.to_string())?;
    let side = (240.0 * settings.pet_scale).round();
    pet.set_size(tauri::LogicalSize::new(side, side + 80.0))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reminders_list(state: State<'_, AppState>) -> Result<Vec<Reminder>, String> {
    Ok(state
        .data
        .lock()
        .map_err(|_| "state lock is poisoned")?
        .reminders
        .clone())
}

#[tauri::command]
pub fn reminders_save(
    state: State<'_, AppState>,
    input: ReminderInput,
) -> Result<Reminder, String> {
    let text = input.text.trim();
    if text.is_empty() || text.chars().count() > 200 {
        return Err("reminder text must contain 1-200 characters".into());
    }
    DateTime::parse_from_rfc3339(&input.due_at)
        .map_err(|_| "dueAt must be an ISO-8601 timestamp")?;
    let reminder = Reminder {
        id: Uuid::new_v4().to_string(),
        text: text.to_string(),
        due_at: input.due_at,
        created_at: Utc::now().to_rfc3339(),
        notified_at: None,
    };
    state
        .data
        .lock()
        .map_err(|_| "state lock is poisoned")?
        .reminders
        .push(reminder.clone());
    state.persist()?;
    Ok(reminder)
}

#[tauri::command]
pub fn reminders_remove(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let removed = {
        let mut data = state.data.lock().map_err(|_| "state lock is poisoned")?;
        let before = data.reminders.len();
        data.reminders.retain(|reminder| reminder.id != id);
        before != data.reminders.len()
    };
    if removed {
        state.persist()?;
    }
    Ok(removed)
}

fn interactions() -> Result<Vec<InteractionSpec>, String> {
    let spec: serde_json::Value = serde_json::from_str(include_str!("../../pet-spec.json"))
        .map_err(|error| error.to_string())?;
    serde_json::from_value(spec["experience"]["interactions"].clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn interactions_list() -> Result<Vec<InteractionSpec>, String> {
    interactions()
}

#[tauri::command]
pub fn interactions_stats(state: State<'_, AppState>) -> Result<PetStats, String> {
    state.normalize_stats_day(&Local::now().date_naive().to_string())?;
    state.public_stats()
}

#[tauri::command]
pub fn interactions_trigger(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<InteractionResult, String> {
    let interaction = interactions()?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or("unknown interaction")?;
    state.normalize_stats_day(&Local::now().date_naive().to_string())?;
    let feedback = interaction.feedback.first().cloned().unwrap_or_default();
    let stats = {
        let mut data = state.data.lock().map_err(|_| "state lock is poisoned")?;
        data.stats.affection += interaction.affection_gain;
        data.stats.mood = (data.stats.mood + 2).min(100);
        data.stats.today_interactions += 1;
        data.stats.last_interaction_date = Local::now().date_naive().to_string();
        data.stats.clone()
    };
    state.persist()?;
    let stats = state.public_stats().unwrap_or(stats);
    app.emit(
        "state-activity",
        StateActivity {
            kind: "interaction".into(),
            state_id: Some(interaction.state_id.clone()),
            duration_ms: Some(interaction.duration_ms),
            feedback: Some(feedback.clone()),
        },
    )
    .map_err(|error| error.to_string())?;
    app.emit("stats-updated", &stats)
        .map_err(|error| error.to_string())?;
    Ok(InteractionResult {
        interaction,
        feedback,
        stats,
    })
}

#[tauri::command]
pub fn window_start_dragging(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let cursor = window
        .cursor_position()
        .map_err(|error| error.to_string())?;
    let session = DragSession {
        id: state.next_drag_id(),
        window_x: position.x,
        window_y: position.y,
        cursor_x: cursor.x,
        cursor_y: cursor.y,
    };
    *state
        .drag_session
        .lock()
        .map_err(|_| "drag lock is poisoned")? = Some(session);
    state
        .paused_walk
        .lock()
        .map_err(|_| "walk lock is poisoned")?
        .insert("drag".into());
    spawn_drag_tracker(app, window, session.id);
    Ok(())
}

fn spawn_drag_tracker(app: tauri::AppHandle, window: WebviewWindow, session_id: u64) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(8));
        let session = app
            .state::<AppState>()
            .drag_session
            .lock()
            .ok()
            .and_then(|value| *value);
        let Some(session) = session.filter(|value| value.id == session_id) else {
            break;
        };
        let Ok(cursor) = window.cursor_position() else {
            continue;
        };
        let position = dragged_window_position(session, cursor.x, cursor.y);
        let still_active = app
            .state::<AppState>()
            .drag_session
            .lock()
            .map(|value| value.as_ref().is_some_and(|value| value.id == session_id))
            .unwrap_or(false);
        if !still_active {
            break;
        }
        let _ = window.set_position(position);
    });
}

#[tauri::command]
pub fn window_finish_drag(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .drag_session
        .lock()
        .map_err(|_| "drag lock is poisoned")?
        .take();
    let edge_snap = state
        .data
        .lock()
        .map_err(|_| "state lock is poisoned")?
        .settings
        .edge_snap;
    let result = (|| -> Result<(), String> {
        if !edge_snap {
            return Ok(());
        }
        let position = window.outer_position().map_err(|error| error.to_string())?;
        let size = window.outer_size().map_err(|error| error.to_string())?;
        let monitor = window
            .current_monitor()
            .map_err(|error| error.to_string())?
            .ok_or("pet window is not on a monitor")?;
        let area = monitor.work_area();
        let bounds = Rect {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        };
        let work_area = Rect {
            x: area.position.x,
            y: area.position.y,
            width: area.size.width,
            height: area.size.height,
        };
        let threshold = (20.0 * window.scale_factor().unwrap_or(1.0)).round() as i32;
        let snapped = snap_rect(bounds, work_area, threshold);
        if snapped == bounds {
            return Ok(());
        }
        window
            .set_position(tauri::PhysicalPosition::new(snapped.x, snapped.y))
            .map_err(|error| error.to_string())?;
        state
            .paused_walk
            .lock()
            .map_err(|_| "walk lock is poisoned")?
            .insert("edge-snap".into());
        app.emit(
            "state-activity",
            StateActivity {
                kind: "edge-snap".into(),
                state_id: Some("peek".into()),
                duration_ms: Some(900),
                feedback: None,
            },
        )
        .map_err(|error| error.to_string())?;
        let handle = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(900));
            if let Ok(mut reasons) = handle.state::<AppState>().paused_walk.lock() {
                reasons.remove("edge-snap");
            }
        });
        Ok(())
    })();
    state
        .paused_walk
        .lock()
        .map_err(|_| "walk lock is poisoned")?
        .remove("drag");
    result
}

fn dragged_window_position(
    session: DragSession,
    cursor_x: f64,
    cursor_y: f64,
) -> tauri::PhysicalPosition<i32> {
    tauri::PhysicalPosition::new(
        (f64::from(session.window_x) + cursor_x - session.cursor_x).round() as i32,
        (f64::from(session.window_y) + cursor_y - session.cursor_y).round() as i32,
    )
}

#[tauri::command]
pub fn window_show_context_menu(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    context_menu::popup(&app, &window, &state)
}

#[tauri::command]
pub fn window_hide_context_menu(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    context_menu::hide(&app, &state)
}

pub(crate) fn show_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| format!("{label} window is unavailable"))?;
    window.show().map_err(|error| error.to_string())?;
    window.unminimize().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn window_show_dashboard(app: tauri::AppHandle) -> Result<(), String> {
    show_window(&app, "dashboard")
}

#[tauri::command]
pub fn window_show_reminder(app: tauri::AppHandle) -> Result<(), String> {
    show_reminder(&app)
}

pub(crate) fn show_reminder(app: &tauri::AppHandle) -> Result<(), String> {
    show_window(app, "reminder")?;
    app.emit_to("reminder", "reminder-compose", ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn window_hide(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if !matches!(label.as_str(), "pet" | "dashboard" | "reminder") {
        return Err("unknown window label".into());
    }
    app.get_webview_window(&label)
        .ok_or("window is unavailable")?
        .hide()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn walk_set_paused(
    state: State<'_, AppState>,
    reason: String,
    paused: bool,
) -> Result<(), String> {
    let reason = reason.trim();
    if reason.is_empty() || reason.len() > 64 {
        return Err("invalid pause reason".into());
    }
    let mut reasons = state
        .paused_walk
        .lock()
        .map_err(|_| "walk lock is poisoned")?;
    update_walk_pause_reasons(&mut reasons, reason, paused);
    Ok(())
}

fn update_walk_pause_reasons(reasons: &mut HashSet<String>, reason: &str, paused: bool) {
    if paused {
        reasons.insert(reason.to_string());
    } else {
        reasons.remove(reason);
        if reason == "chant" {
            // A completed chant originates from a click, so no drag can still be active.
            // Clear a lost pointer-up marker atomically before auto-walk resumes.
            reasons.remove("drag");
        }
    }
}

#[tauri::command]
pub fn runtime_ready(window: WebviewWindow, report: serde_json::Value) -> Result<(), String> {
    println!("pet renderer ready: {report}");
    if window.label() == "pet" {
        window.show().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn runtime_fail(report: serde_json::Value) {
    eprintln!("pet renderer failed: {report}");
}

#[cfg(test)]
mod tests {
    use super::{dragged_window_position, update_walk_pause_reasons};
    use crate::state::DragSession;
    use std::collections::HashSet;

    #[test]
    fn completing_chant_clears_a_stale_drag_pause() {
        let mut reasons = HashSet::from(["drag".to_string(), "chant".to_string()]);

        update_walk_pause_reasons(&mut reasons, "chant", false);

        assert!(reasons.is_empty());
    }

    #[test]
    fn dragged_window_position_tracks_the_cursor_delta() {
        let position = dragged_window_position(
            DragSession {
                id: 1,
                window_x: 320,
                window_y: 480,
                cursor_x: 410.0,
                cursor_y: 550.0,
            },
            487.4,
            603.6,
        );

        assert_eq!(position.x, 397);
        assert_eq!(position.y, 534);
    }
}
