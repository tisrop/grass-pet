use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::Instant,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub edge_snap: bool,
    pub always_on_top: bool,
    pub click_through: bool,
    pub pet_scale: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            edge_snap: true,
            always_on_top: true,
            click_through: false,
            pet_scale: 0.8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub text: String,
    pub due_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetStats {
    pub affection: i64,
    pub mood: i64,
    pub today_interactions: u64,
    pub companion_minutes: u64,
    pub last_interaction_date: String,
}

impl Default for PetStats {
    fn default() -> Self {
        Self {
            affection: 0,
            mood: 80,
            today_interactions: 0,
            companion_minutes: 0,
            last_interaction_date: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PersistedData {
    pub settings: Settings,
    pub reminders: Vec<Reminder>,
    pub stats: PetStats,
    pub companion_seconds: u64,
}

pub struct AppState {
    pub data: Mutex<PersistedData>,
    pub paused_walk: Mutex<HashSet<String>>,
    pub drag_session: Mutex<Option<DragSession>>,
    pub notified_reminders: Mutex<HashSet<String>>,
    drag_sequence: AtomicU64,
    companion_checkpoint: Mutex<Instant>,
    path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub struct DragSession {
    pub id: u64,
    pub window_x: i32,
    pub window_y: i32,
    pub cursor_x: f64,
    pub cursor_y: f64,
}

impl AppState {
    pub fn load(path: PathBuf) -> Self {
        let data = fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default();
        Self {
            data: Mutex::new(data),
            paused_walk: Mutex::new(HashSet::new()),
            drag_session: Mutex::new(None),
            notified_reminders: Mutex::new(HashSet::new()),
            drag_sequence: AtomicU64::new(0),
            companion_checkpoint: Mutex::new(Instant::now()),
            path,
        }
    }

    pub fn load_shared(data_root: &Path) -> Result<Self, String> {
        fs::create_dir_all(data_root).map_err(|error| error.to_string())?;
        let path = data_root.join("state.json");
        restore_isolated_state(data_root, &path)?;
        Ok(Self::load(path))
    }

    pub fn next_drag_id(&self) -> u64 {
        self.drag_sequence.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn persist(&self) -> Result<(), String> {
        self.checkpoint_companion()?;
        let snapshot = self
            .data
            .lock()
            .map_err(|_| "state lock is poisoned")?
            .clone();
        write_json_atomic(&self.path, &snapshot)
    }

    pub fn normalize_stats_day(&self, today: &str) -> Result<(), String> {
        let mut data = self.data.lock().map_err(|_| "state lock is poisoned")?;
        if data.stats.last_interaction_date != today {
            data.stats.today_interactions = 0;
            data.stats.last_interaction_date = today.to_string();
        }
        Ok(())
    }

    pub fn public_stats(&self) -> Result<PetStats, String> {
        let pending_seconds = self
            .companion_checkpoint
            .lock()
            .map_err(|_| "companion clock lock is poisoned")?
            .elapsed()
            .as_secs();
        let data = self.data.lock().map_err(|_| "state lock is poisoned")?;
        let mut stats = data.stats.clone();
        stats.companion_minutes = (data.companion_seconds + pending_seconds) / 60;
        Ok(stats)
    }

    fn checkpoint_companion(&self) -> Result<(), String> {
        let mut checkpoint = self
            .companion_checkpoint
            .lock()
            .map_err(|_| "companion clock lock is poisoned")?;
        let elapsed = checkpoint.elapsed().as_secs();
        if elapsed > 0 {
            self.data
                .lock()
                .map_err(|_| "state lock is poisoned")?
                .companion_seconds += elapsed;
            *checkpoint = Instant::now();
        }
        Ok(())
    }
}

fn write_json_atomic(path: &Path, value: &PersistedData) -> Result<(), String> {
    let parent = path.parent().ok_or("settings path has no parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("{}.json.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let mut file = fs::File::create(&temporary).map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.write_all(b"\n").map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

fn restore_isolated_state(data_root: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Ok(());
    }
    let isolated = data_root
        .join("instances")
        .join("instance-1")
        .join("state.json");
    if !isolated.exists() {
        return Ok(());
    }
    match fs::rename(&isolated, destination) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(&isolated, destination).map_err(|error| error.to_string())?;
            fs::remove_file(isolated).map_err(|error| error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    fn state() -> AppState {
        AppState::load(PathBuf::from("/tmp/grass-pet-state-unit-test.json"))
    }

    #[test]
    fn normalizes_daily_interactions_on_local_date_change() {
        let state = state();
        {
            let mut data = state.data.lock().expect("state lock");
            data.stats.today_interactions = 4;
            data.stats.last_interaction_date = "2026-08-25".into();
        }
        state
            .normalize_stats_day("2026-08-26")
            .expect("normalize stats");
        let stats = state.public_stats().expect("public stats");
        assert_eq!(stats.today_interactions, 0);
        assert_eq!(stats.last_interaction_date, "2026-08-26");
    }

    #[test]
    fn exposes_persisted_companion_seconds_as_whole_minutes() {
        let state = state();
        state.data.lock().expect("state lock").companion_seconds = 125;
        assert_eq!(
            state
                .public_stats()
                .expect("public stats")
                .companion_minutes,
            2
        );
    }

    #[test]
    fn restores_instance_one_data_to_the_shared_state_file() {
        let root = std::env::temp_dir().join(format!("grass-pet-shared-test-{}", Uuid::new_v4()));
        let isolated = root.join("instances/instance-1/state.json");
        fs::create_dir_all(isolated.parent().expect("isolated parent")).expect("create dirs");
        fs::write(&isolated, b"{\"settings\":{}}\n").expect("write isolated state");

        let state = AppState::load_shared(&root).expect("load shared state");

        assert!(root.join("state.json").exists());
        assert!(!isolated.exists());
        drop(state);
        fs::remove_dir_all(root).expect("remove temp root");
    }
}
