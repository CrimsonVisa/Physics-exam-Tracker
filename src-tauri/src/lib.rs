mod date_only;
mod db;
mod error;
mod settings;
mod syllabus;
mod syllabus_catalog;

use chrono::Local;
use serde::Serialize;
use tauri::Manager;

use error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

fn app_info() -> AppInfo {
    AppInfo {
        name: "CSIR-NET Physics Tracker".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    app_info()
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, db::AppState>) -> Result<settings::Settings, AppError> {
    state.with_connection(|connection| settings::load(connection))
}

#[tauri::command]
fn update_settings(
    state: tauri::State<'_, db::AppState>,
    settings: settings::SettingsInput,
) -> Result<settings::Settings, AppError> {
    state.with_connection(|connection| crate::settings::update(connection, settings))
}

#[tauri::command]
fn get_syllabus_catalog(
    state: tauri::State<'_, db::AppState>,
) -> Result<syllabus::SyllabusCatalog, AppError> {
    state.with_connection(|connection| syllabus::load_catalog(connection))
}

#[tauri::command]
fn search_syllabus_topics(
    state: tauri::State<'_, db::AppState>,
    filter: syllabus::SyllabusFilter,
) -> Result<Vec<syllabus::SyllabusTopic>, AppError> {
    state.with_connection(|connection| syllabus::search_topics(connection, filter))
}

#[tauri::command]
fn get_topic_progress(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
) -> Result<syllabus::TopicProgress, AppError> {
    state.with_connection(|connection| syllabus::load_progress(connection, topic_id))
}

#[tauri::command]
fn update_topic_progress(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
    progress: syllabus::TopicProgressInput,
) -> Result<syllabus::TopicProgress, AppError> {
    state.with_connection(|connection| syllabus::update_progress(connection, topic_id, progress))
}

#[tauri::command]
fn get_syllabus_summary(
    state: tauri::State<'_, db::AppState>,
) -> Result<syllabus::SyllabusSummary, AppError> {
    state.with_connection(|connection| syllabus::summary(connection))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let today = Local::now().date_naive();
            let state = match app.path().app_data_dir() {
                Ok(data_dir) => db::AppState::initialize(&data_dir, today),
                Err(error) => {
                    eprintln!("Application data directory could not be resolved: {error}");
                    db::AppState::unavailable(AppError::database_unavailable(
                        "The application-data folder could not be located. Restart the app; \
                         if the problem continues, check your Windows user profile.",
                    ))
                }
            };

            let _ = app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_settings,
            update_settings,
            get_syllabus_catalog,
            search_syllabus_topics,
            get_topic_progress,
            update_topic_progress,
            get_syllabus_summary
        ])
        .run(tauri::generate_context!())
        .expect("error while running the CSIR-NET Physics Tracker application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_exposes_the_product_identity() {
        let info = app_info();

        assert_eq!(info.name, "CSIR-NET Physics Tracker");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }
}
