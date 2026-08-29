use crate::{commands::StateActivity, state::AppState};
use chrono::Utc;
use std::{thread, time::Duration};
use tauri::{Emitter, Manager};

pub fn spawn_scheduler(app: tauri::AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(5));
        let reminders = match app.state::<AppState>().claim_due_reminders(Utc::now()) {
            Ok(reminders) => reminders,
            Err(_) => continue,
        };
        for reminder in reminders {
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
