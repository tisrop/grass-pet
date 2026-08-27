use crate::{commands::StateActivity, state::AppState};
use chrono::{DateTime, Utc};
use std::{thread, time::Duration};
use tauri::{Emitter, Manager};

pub fn spawn_scheduler(app: tauri::AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(5));
        let reminders = app
            .state::<AppState>()
            .data
            .lock()
            .map(|data| data.reminders.clone())
            .unwrap_or_default();
        for reminder in reminders {
            let due = DateTime::parse_from_rfc3339(&reminder.due_at)
                .map(|value| value.with_timezone(&Utc))
                .ok();
            if due.is_none_or(|value| value > Utc::now()) {
                continue;
            }
            let is_new = app
                .state::<AppState>()
                .notified_reminders
                .lock()
                .map(|mut notified| notified.insert(reminder.id.clone()))
                .unwrap_or(false);
            if !is_new {
                continue;
            }
            if let Some(window) = app.get_webview_window("reminder") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            let _ = app.emit(
                "state-activity",
                StateActivity {
                    kind: "reminder".into(),
                    state_id: Some("notify".into()),
                    duration_ms: Some(2400),
                    feedback: Some(reminder.text.clone()),
                },
            );
            let _ = app.emit("reminder-due", reminder);
        }
    });
}
