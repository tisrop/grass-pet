use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Child,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::Instant,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub text: String,
    pub due_at: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notified_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
    spawned_pets: Mutex<HashMap<u32, Child>>,
    drag_sequence: AtomicU64,
    companion_checkpoint: Mutex<Instant>,
    last_persisted: Mutex<PersistedData>,
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
        let data = read_persisted(&path);
        let baseline = data.clone();
        Self {
            data: Mutex::new(data),
            paused_walk: Mutex::new(HashSet::new()),
            drag_session: Mutex::new(None),
            spawned_pets: Mutex::new(HashMap::new()),
            drag_sequence: AtomicU64::new(0),
            companion_checkpoint: Mutex::new(Instant::now()),
            last_persisted: Mutex::new(baseline),
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

    pub fn track_spawned_pet(&self, mut child: Child) -> Result<u32, String> {
        let pid = child.id();
        let mut pets = match self.spawned_pets.lock() {
            Ok(pets) => pets,
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("spawned pets lock is poisoned".into());
            }
        };
        reap_spawned_pets(&mut pets);
        pets.insert(pid, child);
        Ok(pid)
    }

    pub fn reap_spawned_pets(&self) -> Result<(), String> {
        let mut pets = self
            .spawned_pets
            .lock()
            .map_err(|_| "spawned pets lock is poisoned")?;
        reap_spawned_pets(&mut pets);
        Ok(())
    }

    pub fn stop_spawned_pets(&self) -> Result<(), String> {
        let mut pets = self
            .spawned_pets
            .lock()
            .map_err(|_| "spawned pets lock is poisoned")?;
        for child in pets.values_mut() {
            let running = child
                .try_wait()
                .map(|status| status.is_none())
                .unwrap_or(true);
            if running {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        pets.clear();
        Ok(())
    }

    pub fn persist(&self) -> Result<(), String> {
        self.checkpoint_companion()?;
        let local = self
            .data
            .lock()
            .map_err(|_| "state lock is poisoned")?
            .clone();

        let _lock = self.lock_state_file()?;
        let baseline = self
            .last_persisted
            .lock()
            .map_err(|_| "persisted state lock is poisoned")?
            .clone();
        let persisted = read_persisted(&self.path);
        let merged = merge_persisted_data(&persisted, &baseline, &local);

        write_json_atomic(&self.path, &merged)?;
        *self.data.lock().map_err(|_| "state lock is poisoned")? = merged.clone();
        *self
            .last_persisted
            .lock()
            .map_err(|_| "persisted state lock is poisoned")? = merged;
        Ok(())
    }

    /// Atomically claims all reminders that are due and have not been claimed
    /// by any process. The claim is persisted before the caller emits events,
    /// so restarting or running multiple pets cannot notify the same reminder
    /// more than once.
    pub fn claim_due_reminders(&self, now: DateTime<Utc>) -> Result<Vec<Reminder>, String> {
        let _lock = self.lock_state_file()?;
        let mut persisted = read_persisted(&self.path);
        let mut claimed = Vec::new();
        let notified_at = now.to_rfc3339();

        for reminder in &mut persisted.reminders {
            let due = DateTime::parse_from_rfc3339(&reminder.due_at)
                .map(|value| value.with_timezone(&Utc))
                .ok();
            if reminder.notified_at.is_none() && due.is_some_and(|value| value <= now) {
                reminder.notified_at = Some(notified_at.clone());
                claimed.push(reminder.clone());
            }
        }

        if claimed.is_empty() {
            return Ok(claimed);
        }

        write_json_atomic(&self.path, &persisted)?;

        // Keep this process's in-memory view consistent without replacing
        // unrelated, potentially newer local state.
        let mut data = self.data.lock().map_err(|_| "state lock is poisoned")?;
        for claimed_reminder in &claimed {
            if let Some(reminder) = data
                .reminders
                .iter_mut()
                .find(|reminder| reminder.id == claimed_reminder.id)
            {
                reminder.notified_at = claimed_reminder.notified_at.clone();
            }
        }
        Ok(claimed)
    }

    fn lock_state_file(&self) -> Result<fs::File, String> {
        let parent = self.path.parent().ok_or("settings path has no parent")?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let lock_path = self.path.with_extension("json.lock");
        let file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|error| error.to_string())?;
        file.lock().map_err(|error| error.to_string())?;
        Ok(file)
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

fn reap_spawned_pets(pets: &mut HashMap<u32, Child>) {
    pets.retain(|_, child| match child.try_wait() {
        Ok(Some(_)) | Err(_) => false,
        Ok(None) => true,
    });
}

fn read_persisted(path: &Path) -> PersistedData {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn merge_persisted_data(
    persisted: &PersistedData,
    baseline: &PersistedData,
    local: &PersistedData,
) -> PersistedData {
    let mut merged = persisted.clone();

    // An unchanged stale snapshot must not overwrite newer settings from
    // another instance. Explicit local changes remain last-writer-wins.
    if local.settings != baseline.settings {
        merged.settings = local.settings.clone();
    }

    merged.reminders = merge_reminders(&persisted.reminders, &baseline.reminders, &local.reminders);
    merged.stats = merge_stats(&persisted.stats, &baseline.stats, &local.stats);
    merged.companion_seconds = persisted.companion_seconds.saturating_add(
        local
            .companion_seconds
            .saturating_sub(baseline.companion_seconds),
    );
    merged
}

fn merge_reminders(
    persisted: &[Reminder],
    baseline: &[Reminder],
    local: &[Reminder],
) -> Vec<Reminder> {
    let mut merged = persisted.to_vec();

    // A reminder removed locally is a deliberate change, so apply the
    // deletion against the latest on-disk snapshot.
    for previous in baseline {
        if !local.iter().any(|reminder| reminder.id == previous.id) {
            merged.retain(|reminder| reminder.id != previous.id);
        }
    }

    // Apply only new or modified reminders. Unchanged local reminders stay as
    // persisted, preserving fields such as notifiedAt written by another
    // process.
    for reminder in local {
        let changed = baseline
            .iter()
            .find(|previous| previous.id == reminder.id)
            .is_none_or(|previous| previous != reminder);
        if !changed {
            continue;
        }
        if let Some(existing) = merged.iter_mut().find(|item| item.id == reminder.id) {
            *existing = reminder.clone();
        } else {
            merged.push(reminder.clone());
        }
    }

    merged
}

fn merge_stats(persisted: &PetStats, baseline: &PetStats, local: &PetStats) -> PetStats {
    let mut merged = persisted.clone();
    merged.affection = persisted
        .affection
        .saturating_add(local.affection.saturating_sub(baseline.affection));
    merged.mood = persisted
        .mood
        .saturating_add(local.mood.saturating_sub(baseline.mood))
        .clamp(0, 100);

    let (date, interactions) = merge_daily_interactions(persisted, baseline, local);
    merged.last_interaction_date = date;
    merged.today_interactions = interactions;
    merged
}

fn merge_daily_interactions(
    persisted: &PetStats,
    baseline: &PetStats,
    local: &PetStats,
) -> (String, u64) {
    if local.last_interaction_date == baseline.last_interaction_date {
        return (
            persisted.last_interaction_date.clone(),
            persisted.today_interactions.saturating_add(
                local
                    .today_interactions
                    .saturating_sub(baseline.today_interactions),
            ),
        );
    }

    if local.last_interaction_date == persisted.last_interaction_date {
        return (
            persisted.last_interaction_date.clone(),
            persisted
                .today_interactions
                .saturating_add(local.today_interactions),
        );
    }

    if persisted.last_interaction_date == baseline.last_interaction_date {
        return (
            local.last_interaction_date.clone(),
            local.today_interactions,
        );
    }

    if persisted.last_interaction_date >= local.last_interaction_date {
        (
            persisted.last_interaction_date.clone(),
            persisted.today_interactions,
        )
    } else {
        (
            local.last_interaction_date.clone(),
            local.today_interactions,
        )
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
    use super::{AppState, Reminder};
    use chrono::{DateTime, Utc};
    use std::fs;
    use uuid::Uuid;

    fn state() -> AppState {
        AppState::load(
            std::env::temp_dir().join(format!("grass-pet-state-unit-test-{}.json", Uuid::new_v4())),
        )
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

    #[test]
    fn claims_a_due_reminder_only_once_after_reloading_state() {
        let root = std::env::temp_dir().join(format!("grass-pet-reminder-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("state.json");
        let state = AppState::load(path.clone());
        state
            .data
            .lock()
            .expect("state lock")
            .reminders
            .push(Reminder {
                id: "reminder-1".into(),
                text: "drink water".into(),
                due_at: "2026-08-28T12:00:00Z".into(),
                created_at: "2026-08-28T11:00:00Z".into(),
                notified_at: None,
            });
        state.persist().expect("persist reminder");

        let now = DateTime::parse_from_rfc3339("2026-08-29T12:00:00Z")
            .expect("parse now")
            .with_timezone(&Utc);
        assert_eq!(
            state.claim_due_reminders(now).expect("first claim").len(),
            1
        );

        let reloaded = AppState::load(path);
        assert!(reloaded
            .claim_due_reminders(now)
            .expect("second claim")
            .is_empty());
        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn concurrent_processes_claim_a_due_reminder_once() {
        use std::sync::Arc;
        use std::thread;

        let root = std::env::temp_dir().join(format!("grass-pet-reminder-race-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("state.json");
        let state = AppState::load(path.clone());
        state
            .data
            .lock()
            .expect("state lock")
            .reminders
            .push(Reminder {
                id: "reminder-race".into(),
                text: "stand up".into(),
                due_at: "2026-08-28T12:00:00Z".into(),
                created_at: "2026-08-28T11:00:00Z".into(),
                notified_at: None,
            });
        state.persist().expect("persist reminder");

        let first = Arc::new(AppState::load(path.clone()));
        let second = Arc::new(AppState::load(path));
        let now = DateTime::parse_from_rfc3339("2026-08-29T12:00:00Z")
            .expect("parse now")
            .with_timezone(&Utc);
        let first_thread = {
            let state = Arc::clone(&first);
            thread::spawn(move || state.claim_due_reminders(now).expect("first claim"))
        };
        let second_thread = {
            let state = Arc::clone(&second);
            thread::spawn(move || state.claim_due_reminders(now).expect("second claim"))
        };
        let claimed = first_thread.join().expect("first thread").len()
            + second_thread.join().expect("second thread").len();
        assert_eq!(claimed, 1);
        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn merges_counter_deltas_from_stale_instance_snapshots() {
        let root = std::env::temp_dir().join(format!("grass-pet-counter-race-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("state.json");

        let seed = AppState::load(path.clone());
        {
            let mut data = seed.data.lock().expect("state lock");
            data.stats.affection = 10;
            data.stats.mood = 80;
            data.stats.today_interactions = 4;
            data.stats.last_interaction_date = "2026-08-29".into();
            data.companion_seconds = 100;
        }
        seed.persist().expect("persist seed");

        // Both instances intentionally load the same baseline before either
        // one writes, reproducing the stale-snapshot race.
        let first = AppState::load(path.clone());
        let second = AppState::load(path.clone());
        {
            let mut data = first.data.lock().expect("first state lock");
            data.stats.affection += 3;
            data.stats.mood += 2;
            data.stats.today_interactions += 1;
            data.companion_seconds += 60;
        }
        first.persist().expect("persist first delta");

        {
            let mut data = second.data.lock().expect("second state lock");
            data.stats.affection += 5;
            data.stats.mood += 2;
            data.stats.today_interactions += 2;
            data.companion_seconds += 120;
        }
        second.persist().expect("persist second delta");

        let merged = AppState::load(path);
        let data = merged.data.lock().expect("merged state lock");
        assert_eq!(data.stats.affection, 18);
        assert_eq!(data.stats.mood, 84);
        assert_eq!(data.stats.today_interactions, 7);
        assert_eq!(data.companion_seconds, 280);
        fs::remove_dir_all(root).expect("remove temp root");
    }
}
