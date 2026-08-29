use fs2::FileExt;
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Serialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub published_at: Option<String>,
    pub update_mode: UpdateMode,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateMode {
    Installer,
    Portable,
}

#[derive(Clone, Debug, Serialize)]
pub struct UpdateProgressEvent {
    pub request_id: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub phase: &'static str,
}

const MAX_RELEASE_NOTES_CHARS: usize = 16_000;
const UPDATE_LOCK_FILE: &str = "update.lock";

struct UpdateLock(File);

impl UpdateLock {
    fn acquire(path: &Path) -> Result<Self, String> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .map_err(|error| format!("创建更新锁失败：{error}"))?;
        file.try_lock_exclusive()
            .map_err(|_| "已有另一个阿飘正在安装更新，请等待完成后重试".to_string())?;
        Ok(Self(file))
    }
}

impl Drop for UpdateLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

fn acquire_app_update_lock(app: &AppHandle) -> Result<UpdateLock, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("定位更新锁目录失败：{error}"))?;
    fs::create_dir_all(&data_dir).map_err(|error| format!("创建更新锁目录失败：{error}"))?;
    UpdateLock::acquire(&data_dir.join(UPDATE_LOCK_FILE))
}

fn sanitize_release_notes(notes: Option<String>) -> Option<String> {
    notes.map(|notes| {
        if notes.chars().count() <= MAX_RELEASE_NOTES_CHARS {
            return notes;
        }
        let mut truncated: String = notes.chars().take(MAX_RELEASE_NOTES_CHARS).collect();
        truncated.push_str("\n\n[更新说明过长，已截断]");
        truncated
    })
}

fn validate_request_id(request_id: &str) -> Result<(), String> {
    if request_id.is_empty()
        || request_id.len() > 64
        || !request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("更新请求标识格式无效".into());
    }
    Ok(())
}

fn validate_expected_version(version: &str) -> Result<(), String> {
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
    {
        return Err("预期更新版本格式无效".into());
    }
    Ok(())
}

fn current_update_mode() -> UpdateMode {
    if cfg!(target_os = "windows") && tauri::utils::platform::bundle_type().is_none() {
        UpdateMode::Portable
    } else {
        UpdateMode::Installer
    }
}

fn localized_update_error() -> String {
    "检查更新失败，请检查网络或稍后重试".into()
}

fn localized_download_error() -> String {
    "下载更新失败，请检查网络或稍后重试".into()
}

fn localized_install_error() -> String {
    "安装更新失败，应用未重启，请稍后重试".into()
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> Result<UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let update_mode = current_update_mode();
    let updater = app
        .updater()
        .map_err(|_| "初始化更新检查失败，请稍后重试".to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|_| localized_update_error())?;

    Ok(match update {
        Some(update) => UpdateCheckResult {
            current_version,
            available: true,
            version: Some(update.version),
            notes: sanitize_release_notes(update.body),
            published_at: update.date.map(|date| date.to_string()),
            update_mode,
        },
        None => UpdateCheckResult {
            current_version,
            available: false,
            version: None,
            notes: None,
            published_at: None,
            update_mode,
        },
    })
}

#[tauri::command]
pub async fn update_download_and_install(
    app: AppHandle,
    request_id: String,
    expected_version: String,
) -> Result<(), String> {
    validate_request_id(&request_id)?;
    validate_expected_version(&expected_version)?;

    if matches!(current_update_mode(), UpdateMode::Portable) {
        return Err("Windows 便携版不支持应用内安装，请下载安装包后手动覆盖".into());
    }

    // All summoned pets share the same app-data directory. Keep the lock alive
    // through the check, download and install so update.install cannot race
    // across processes.
    let _update_lock = acquire_app_update_lock(&app)?;

    let updater = app
        .updater()
        .map_err(|_| "初始化更新下载失败，请稍后重试".to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|_| localized_update_error())?
        .ok_or_else(|| "当前已是最新版本，无需下载安装".to_string())?;

    if update.version != expected_version {
        return Err(format!(
            "可用更新已从 v{expected_version} 变更为 v{}，请重新检查后再安装",
            update.version
        ));
    }

    let progress_app = app.clone();
    let installing_app = app.clone();
    let installing_request_id = request_id.clone();
    let mut downloaded = 0_u64;
    let bytes = update
        .download(
            move |chunk_size, total| {
                downloaded = downloaded.saturating_add(chunk_size as u64);
                let _ = progress_app.emit(
                    "update-progress",
                    UpdateProgressEvent {
                        request_id: request_id.clone(),
                        downloaded,
                        total,
                        phase: "downloading",
                    },
                );
            },
            move || {
                let _ = installing_app.emit(
                    "update-progress",
                    UpdateProgressEvent {
                        request_id: installing_request_id.clone(),
                        downloaded: 0,
                        total: None,
                        phase: "installing",
                    },
                );
            },
        )
        .await
        .map_err(|_| localized_download_error())?;

    update.install(bytes).map_err(|_| localized_install_error())
}

#[tauri::command]
pub fn update_restart(app: AppHandle) -> Result<(), String> {
    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_lock_rejects_concurrent_acquisition_and_releases_on_drop() {
        let path =
            std::env::temp_dir().join(format!("grass-pet-update-lock-{}", std::process::id()));
        let first = UpdateLock::acquire(&path).expect("first lock should be acquired");
        assert!(UpdateLock::acquire(&path).is_err());
        drop(first);
        assert!(UpdateLock::acquire(&path).is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn validates_update_inputs() {
        assert!(validate_request_id("attempt-1").is_ok());
        assert!(validate_request_id("../unsafe").is_err());
        assert!(validate_expected_version("1.2.3-beta.1+build.2").is_ok());
        assert!(validate_expected_version("1.2.3\nunsafe").is_err());
    }

    #[test]
    fn truncates_release_notes_without_splitting_utf8() {
        let notes = sanitize_release_notes(Some("更".repeat(MAX_RELEASE_NOTES_CHARS + 1))).unwrap();
        assert!(notes.ends_with("\n\n[更新说明过长，已截断]"));
        assert_eq!(
            notes
                .trim_end_matches("\n\n[更新说明过长，已截断]")
                .chars()
                .count(),
            MAX_RELEASE_NOTES_CHARS
        );
    }
}
