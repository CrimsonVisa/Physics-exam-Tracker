mod date_only;
mod db;
mod error;
mod sessions;
mod settings;
mod syllabus;
mod syllabus_catalog;
mod timer;

use chrono::{Local, Utc};
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

#[tauri::command]
fn get_timer_snapshot(
    state: tauri::State<'_, db::AppState>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| timer::snapshot(connection, Utc::now()))
}

#[tauri::command]
fn update_timer_settings(
    state: tauri::State<'_, db::AppState>,
    settings: timer::TimerSettingsInput,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| {
        timer::update_settings(connection, settings)?;
        timer::snapshot(connection, Utc::now())
    })
}

#[tauri::command]
fn start_study_timer(
    state: tauri::State<'_, db::AppState>,
    mode: String,
    topic_id: Option<i64>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| {
        let now = Utc::now();
        timer::start(connection, now, Local::now().date_naive(), &mode, topic_id)
    })
}

#[tauri::command]
fn pause_study_timer(
    state: tauri::State<'_, db::AppState>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| timer::pause(connection, Utc::now()))
}

#[tauri::command]
fn resume_study_timer(
    state: tauri::State<'_, db::AppState>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| timer::resume(connection, Utc::now()))
}

#[tauri::command]
fn finish_study_timer(
    state: tauri::State<'_, db::AppState>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| timer::finish(connection, Utc::now()))
}

#[tauri::command]
fn discard_study_timer(
    state: tauri::State<'_, db::AppState>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| timer::discard(connection, Utc::now()))
}

#[tauri::command]
fn get_study_sessions(
    state: tauri::State<'_, db::AppState>,
    filter: sessions::StudySessionFilter,
) -> Result<Vec<sessions::StudySession>, AppError> {
    state.with_connection(|connection| sessions::list(connection, filter))
}

#[tauri::command]
fn get_session_topics(
    state: tauri::State<'_, db::AppState>,
) -> Result<Vec<sessions::SessionTopic>, AppError> {
    state.with_connection(|connection| sessions::list_topics(connection))
}

#[tauri::command]
fn create_manual_session(
    state: tauri::State<'_, db::AppState>,
    session: sessions::StudySessionInput,
) -> Result<sessions::StudySession, AppError> {
    state.with_connection(|connection| sessions::create_manual(connection, session))
}

#[tauri::command]
fn update_study_session(
    state: tauri::State<'_, db::AppState>,
    session_id: i64,
    session: sessions::StudySessionInput,
) -> Result<sessions::StudySession, AppError> {
    state.with_connection(|connection| sessions::update(connection, session_id, session))
}

#[tauri::command]
fn delete_study_session(
    state: tauri::State<'_, db::AppState>,
    session_id: i64,
) -> Result<(), AppError> {
    state.with_connection(|connection| sessions::delete(connection, session_id))
}

#[tauri::command]
fn get_study_totals(
    state: tauri::State<'_, db::AppState>,
    from_date: String,
    to_date: String,
    topic_id: Option<i64>,
) -> Result<sessions::StudyTotals, AppError> {
    state.with_connection(|connection| {
        sessions::totals(
            connection,
            &from_date,
            &to_date,
            Local::now().date_naive(),
            topic_id,
        )
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
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
            if app
                .state::<db::AppState>()
                .with_connection(|_| Ok(()))
                .is_ok()
            {
                timer::start_heartbeat(app.handle().clone());
            }
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
            get_syllabus_summary,
            get_timer_snapshot,
            update_timer_settings,
            start_study_timer,
            pause_study_timer,
            resume_study_timer,
            finish_study_timer,
            discard_study_timer,
            get_study_sessions,
            get_session_topics,
            create_manual_session,
            update_study_session,
            delete_study_session,
            get_study_totals
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
