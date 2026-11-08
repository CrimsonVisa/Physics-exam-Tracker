#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod analytics;
mod app_paths;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod backup;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod csv_import;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod dashboard;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod date_only;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod db;
mod error;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod exams;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod mock_tests;
#[cfg(test)]
mod performance_audit;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod planner;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod portability;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod pyq;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod reschedule;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod resources;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod revision;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod revision_plan;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod sessions;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod settings;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod study_analytics;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod syllabus;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod syllabus_catalog;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod timer;
#[allow(dead_code)] // Legacy V1 module retained but not registered in the V2-only binary.
mod topic_intelligence;
mod v2;

use std::path::Path;

use chrono::{Local, Utc};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use error::AppError;
use v2::{
    V2State, v2_acquire_pyq_source, v2_apply_exam_date_change, v2_apply_pyq_import,
    v2_apply_restore_backup, v2_archive_mock_test, v2_archive_pyq_question, v2_archive_resource,
    v2_check_integrity, v2_complete_revision, v2_create_manual_backup, v2_create_manual_session,
    v2_defer_revision, v2_delete_block, v2_delete_quick_start_config, v2_delete_study_session,
    v2_delete_task, v2_discard_timer, v2_extract_pyq_source, v2_finish_timer,
    v2_generate_revision_daily_plan, v2_get_agenda, v2_get_analytics, v2_get_app_settings,
    v2_get_backup_overview, v2_get_concept_detail, v2_get_dashboard_sections,
    v2_get_database_status, v2_get_exam_syllabus, v2_get_flashcard_detail,
    v2_get_flashcard_review_queue, v2_get_flashcard_stats, v2_get_planner_day,
    v2_get_practice_analytics, v2_get_practice_options, v2_get_pyq_answer_key,
    v2_get_pyq_extraction_candidate, v2_get_pyq_question_detail, v2_get_pyq_source_counts,
    v2_get_pyq_source_detail, v2_get_question_mock_session, v2_get_restore_recovery,
    v2_get_revision_daily_plan, v2_get_revision_dashboard, v2_get_study_totals, v2_get_timer,
    v2_global_search, v2_heartbeat_timer, v2_import_resource_file, v2_initialize_database,
    v2_link_block_session, v2_link_resource_exam, v2_link_revision_plan_block,
    v2_list_exam_profiles, v2_list_exams, v2_list_flashcards, v2_list_mock_tests,
    v2_list_pyq_answer_key_conflicts, v2_list_pyq_answer_keys, v2_list_pyq_attempts,
    v2_list_pyq_extraction_candidates, v2_list_pyq_questions, v2_list_pyq_sources,
    v2_list_question_mock_candidates, v2_list_quick_start_configs, v2_list_resources,
    v2_list_sessions, v2_move_revision_plan_item, v2_pause_timer, v2_preview_exam_date_change,
    v2_preview_pyq_answer_key, v2_preview_pyq_import, v2_preview_restore_backup,
    v2_rebalance_revision_daily_plan, v2_reconcile_pyq_source, v2_record_pyq_attempt,
    v2_reject_pyq_extraction_candidate, v2_remove_resource_file, v2_replay_concept_mastery,
    v2_reschedule_revision, v2_reschedule_task, v2_reset_dashboard_sections,
    v2_restore_recovery_safety, v2_resume_timer, v2_review_flashcard,
    v2_run_automatic_backup_if_due, v2_save_block, v2_save_dashboard_sections, v2_save_flashcard,
    v2_save_mock_test, v2_save_pyq_answer_key, v2_save_pyq_question, v2_save_pyq_source,
    v2_save_question_mock_state, v2_save_quick_start_config, v2_save_resource, v2_save_task,
    v2_schedule_revision_concept, v2_search_concepts, v2_set_block_status, v2_set_exam_active,
    v2_set_flashcard_archived, v2_set_mock_test_status, v2_set_pyq_source_status,
    v2_set_revision_status, v2_set_session_allocations, v2_set_task_status,
    v2_set_timer_allocations, v2_start_question_mock, v2_start_timer, v2_submit_question_mock,
    v2_update_app_settings, v2_update_backup_settings, v2_update_concept_mapping,
    v2_update_concept_mastery, v2_update_pyq_extraction_candidate, v2_update_resource_progress,
    v2_update_revision_settings, v2_update_study_session, v2_update_syllabus_node_progress,
    v2_update_timer_settings, v2_verify_mastery_run, v2_verify_pyq_extraction_candidate,
};

async fn run_blocking<T, F>(app: AppHandle, operation: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&db::AppState) -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<db::AppState>();
        operation(state.inner())
    })
    .await
    .map_err(|_| {
        AppError::internal(
            "The background operation ended unexpectedly. Refresh the screen and try again.",
        )
    })?
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

fn app_info() -> AppInfo {
    AppInfo {
        name: "Physics Tracker".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_app_info() -> AppInfo {
    app_info()
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_exam_profiles(
    state: tauri::State<'_, db::AppState>,
) -> Result<Vec<exams::ExamProfile>, AppError> {
    state.with_connection(|connection| exams::list(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_exam_date(
    state: tauri::State<'_, db::AppState>,
    exam_id: i64,
    exam_date: Option<String>,
) -> Result<exams::ExamProfile, AppError> {
    state
        .with_connection(|connection| exams::update_date(connection, exam_id, exam_date.as_deref()))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn set_exam_active(
    state: tauri::State<'_, db::AppState>,
    exam_id: i64,
    is_active: bool,
) -> Result<exams::ExamProfile, AppError> {
    state.with_connection(|connection| exams::set_active(connection, exam_id, is_active))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn confirm_legacy_exam_date(
    state: tauri::State<'_, db::AppState>,
    exam_id: i64,
) -> Result<exams::ExamProfile, AppError> {
    state.with_connection(|connection| exams::confirm_legacy_date(connection, exam_id))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn dismiss_legacy_exam_date_notice(
    state: tauri::State<'_, db::AppState>,
    exam_id: i64,
) -> Result<exams::ExamProfile, AppError> {
    state.with_connection(|connection| exams::dismiss_legacy_date_notice(connection, exam_id))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_restore_recovery(
    state: tauri::State<'_, db::AppState>,
) -> Result<Option<backup::RestoreRecovery>, AppError> {
    let data_dir = state.data_dir()?;
    let live_path = state
        .database_path()
        .ok_or_else(AppError::database_unavailable_message)?;
    // Hold the same database lock used by restore/Keep while inspecting the
    // sidecar and filesystem, so the banner cannot race a mutation.
    state.with_connection(|connection| {
        let schema = backup::schema_version(connection)?;
        let recovery = backup::restore_recovery(data_dir, live_path, schema);
        Ok(
            if recovery.stage == "unknown"
                && recovery.safety_path.is_none()
                && recovery.leftover_files.is_empty()
            {
                None
            } else {
                Some(recovery)
            },
        )
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn keep_restore_recovery(
    state: tauri::State<'_, db::AppState>,
    confirmed: bool,
) -> Result<backup::KeepRecoveryResult, AppError> {
    let data_dir = state.data_dir()?;
    let live_path = state
        .database_path()
        .ok_or_else(AppError::database_unavailable_message)?;
    // Keep holds the database lock across revalidation, candidate deletion,
    // and sidecar cleanup. Restore and safety restore use the same lock.
    state.with_connection(|connection| {
        let schema = backup::schema_version(connection)?;
        backup::keep_restore_recovery(data_dir, live_path, schema, confirmed)
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn restore_recovery_safety(
    state: tauri::State<'_, db::AppState>,
) -> Result<backup::BackupResult, AppError> {
    // Reads the sidecar and performs the restore under one database lock.
    state.restore_recovery_safety(Local::now().date_naive())
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_backup_overview(
    state: tauri::State<'_, db::AppState>,
) -> Result<backup::BackupOverview, AppError> {
    let data_dir = state.data_dir()?.to_path_buf();
    state.with_connection(|connection| backup::overview(connection, &data_dir))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_backup_settings(
    state: tauri::State<'_, db::AppState>,
    settings: backup::BackupSettingsInput,
) -> Result<backup::BackupSettings, AppError> {
    state.with_connection(|connection| backup::update_settings(connection, settings))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn create_manual_backup(
    app: AppHandle,
    destination: Option<String>,
) -> Result<backup::BackupResult, AppError> {
    run_blocking(app, move |state| {
        let data_dir = state.data_dir()?.to_path_buf();
        state.with_connection(|connection| {
            backup::create_manual_backup(connection, destination.as_deref(), &data_dir)
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn run_automatic_backup_if_due(
    app: AppHandle,
) -> Result<backup::AutoBackupOutcome, AppError> {
    run_blocking(app, move |state| {
        let data_dir = state.data_dir()?.to_path_buf();
        state.with_connection(|connection| {
            backup::automatic_backup(connection, &data_dir, Utc::now())
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn preview_restore_backup(
    app: AppHandle,
    path: String,
) -> Result<backup::RestorePreview, AppError> {
    run_blocking(app, move |state| {
        let current_path = state.database_path().map(Path::to_path_buf);
        state.with_connection(|connection| {
            backup::preview_restore(
                Path::new(&path),
                backup::schema_version(connection)?,
                current_path.as_deref(),
            )
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn apply_restore_backup(
    app: AppHandle,
    path: String,
) -> Result<backup::BackupResult, AppError> {
    run_blocking(app, move |state| {
        state.restore_from_backup(Path::new(&path), Local::now().date_naive())
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn export_csv_data(
    app: AppHandle,
    destination: String,
) -> Result<portability::CsvExportResult, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            portability::export_csv_datasets(connection, Path::new(&destination))
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn export_json_data(
    app: AppHandle,
    destination: String,
) -> Result<portability::JsonExportResult, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            portability::export_json(connection, Path::new(&destination))
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn preview_json_import(
    app: AppHandle,
    path: String,
) -> Result<portability::JsonImportPreview, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            portability::preview_json_import(connection, Path::new(&path))
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn apply_json_import(
    app: AppHandle,
    path: String,
    strategy: portability::JsonImportStrategy,
    expected_sha256: String,
) -> Result<portability::JsonImportResult, AppError> {
    run_blocking(app, move |state| {
        let data_dir = state.data_dir()?.to_path_buf();
        state.with_connection(|connection| {
            portability::apply_json_import(
                connection,
                Path::new(&path),
                strategy,
                &expected_sha256,
                &data_dir,
            )
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_resource_catalogue(
    state: tauri::State<'_, db::AppState>,
) -> Result<Vec<resources::Resource>, AppError> {
    state.with_connection(|connection| resources::list_resources(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_resource_overview(
    state: tauri::State<'_, db::AppState>,
) -> Result<resources::ResourceOverview, AppError> {
    state.with_connection(|connection| resources::overview(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn refresh_resource_catalogue(
    app: AppHandle,
) -> Result<resources::DiscoverySummary, AppError> {
    run_blocking(app, |state| {
        let mut fetcher = resources::HttpFetcher::new()?;
        let fetched = resources::fetch_catalogue(&mut fetcher);
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        state.with_connection(|connection| {
            resources::apply_fetched_discovery(connection, fetched, &now)
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn create_manual_resource(
    state: tauri::State<'_, db::AppState>,
    resource: resources::ManualResourceInput,
) -> Result<resources::Resource, AppError> {
    state.with_connection(|connection| resources::create_manual(connection, resource))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_resource_progress(
    state: tauri::State<'_, db::AppState>,
    resource_id: i64,
    progress: resources::ResourceProgressInput,
) -> Result<resources::Resource, AppError> {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    state.with_connection(|connection| {
        resources::update_progress(connection, resource_id, progress, &now)
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn mark_resource_opened(
    state: tauri::State<'_, db::AppState>,
    resource_id: i64,
) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    state.with_connection(|connection| resources::mark_opened(connection, resource_id, &now))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn import_resource_pdf(
    app: AppHandle,
    resource_id: i64,
    source_path: String,
    destination_dir: String,
) -> Result<resources::LocalResourceFile, AppError> {
    run_blocking(app, move |state| {
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        state.with_connection(|connection| {
            resources::import_local_pdf(
                connection,
                resource_id,
                Path::new(&source_path),
                Path::new(&destination_dir),
                &now,
            )
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn remove_resource_pdf(app: AppHandle, resource_id: i64) -> Result<(), AppError> {
    run_blocking(app, move |state| {
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        state.with_connection(|connection| {
            resources::remove_local_pdf(connection, resource_id, &now)
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_resource_local_file(
    state: tauri::State<'_, db::AppState>,
    resource_id: i64,
) -> Result<resources::LocalResourceFile, AppError> {
    state.with_connection(|connection| resources::local_pdf_status(connection, resource_id))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn relink_resource_pdf(
    app: AppHandle,
    resource_id: i64,
    source_path: String,
) -> Result<resources::LocalResourceFile, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            resources::relink_local_pdf(connection, resource_id, Path::new(&source_path))
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn replace_resource_pdf(
    app: AppHandle,
    resource_id: i64,
    source_path: String,
    destination_dir: String,
) -> Result<resources::LocalResourceFile, AppError> {
    run_blocking(app, move |state| {
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        state.with_connection(|connection| {
            resources::replace_local_pdf(
                connection,
                resource_id,
                Path::new(&source_path),
                Path::new(&destination_dir),
                &now,
            )
        })
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_settings(state: tauri::State<'_, db::AppState>) -> Result<settings::Settings, AppError> {
    state.with_connection(|connection| settings::load(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_settings(
    state: tauri::State<'_, db::AppState>,
    settings: settings::SettingsInput,
) -> Result<settings::Settings, AppError> {
    state.with_connection(|connection| {
        crate::settings::update(connection, settings, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn preview_exam_date_change(
    state: tauri::State<'_, db::AppState>,
    settings: settings::SettingsInput,
) -> Result<reschedule::ExamReschedulePreview, AppError> {
    state.with_connection(|connection| {
        reschedule::preview(connection, settings, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn apply_exam_date_change(
    state: tauri::State<'_, db::AppState>,
    change: reschedule::ApplyExamDateChangeInput,
) -> Result<reschedule::ApplyExamDateChangeResult, AppError> {
    state.with_connection(|connection| {
        reschedule::apply(connection, change, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_pyq_options(state: tauri::State<'_, db::AppState>) -> Result<pyq::PyqOptions, AppError> {
    state.with_connection(|connection| pyq::options(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_pyq_attempts(
    state: tauri::State<'_, db::AppState>,
    filter: pyq::PyqFilter,
) -> Result<Vec<pyq::PyqAttempt>, AppError> {
    state.with_connection(|connection| pyq::list(connection, filter))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn create_pyq_attempt(
    state: tauri::State<'_, db::AppState>,
    attempt: pyq::PyqInput,
) -> Result<pyq::PyqAttempt, AppError> {
    state.with_connection(|connection| pyq::create(connection, attempt))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_pyq_attempt(
    state: tauri::State<'_, db::AppState>,
    attempt_id: i64,
    attempt: pyq::PyqInput,
) -> Result<pyq::PyqAttempt, AppError> {
    state.with_connection(|connection| pyq::update(connection, attempt_id, attempt))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn delete_pyq_attempt(
    state: tauri::State<'_, db::AppState>,
    attempt_id: i64,
) -> Result<(), AppError> {
    state.with_connection(|connection| pyq::delete(connection, attempt_id))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn save_pyq_csv_template(app: tauri::AppHandle) -> Result<String, AppError> {
    save_csv_template(&app, "csir-net-pyq-template.csv", &pyq::template())
}

fn save_csv_template(
    app: &tauri::AppHandle,
    file_name: &str,
    content: &str,
) -> Result<String, AppError> {
    let directory = app.path().download_dir().map_err(|error| {
        AppError::file_operation("The Downloads folder could not be located", error)
    })?;

    Ok(write_csv_template(&directory, file_name, content)?
        .display()
        .to_string())
}

fn write_csv_template(
    directory: &Path,
    file_name: &str,
    content: &str,
) -> Result<std::path::PathBuf, AppError> {
    std::fs::create_dir_all(directory).map_err(|error| {
        AppError::file_operation("The Downloads folder could not be prepared", error)
    })?;
    let path = unique_template_path(directory, file_name);
    std::fs::write(&path, content).map_err(|error| {
        AppError::file_operation("The CSV template could not be saved to Downloads", error)
    })?;

    Ok(path)
}

fn unique_template_path(directory: &Path, file_name: &str) -> std::path::PathBuf {
    let candidate = directory.join(file_name);
    if !candidate.exists() {
        return candidate;
    }

    let source = Path::new(file_name);
    let stem = source
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "template".to_owned());
    let extension = source
        .extension()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();

    for index in 1..=1_000 {
        let candidate = if extension.is_empty() {
            directory.join(format!("{stem} ({index})"))
        } else {
            directory.join(format!("{stem} ({index}).{extension}"))
        };
        if !candidate.exists() {
            return candidate;
        }
    }

    let fallback = format!("{stem}-{}", std::process::id());
    if extension.is_empty() {
        directory.join(fallback)
    } else {
        directory.join(format!("{fallback}.{extension}"))
    }
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn preview_pyq_import(
    app: AppHandle,
    csv: String,
) -> Result<csv_import::ImportPreview, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| pyq::preview_import(connection, &csv))
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn apply_pyq_import(
    app: AppHandle,
    csv: String,
) -> Result<csv_import::ImportResult, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| pyq::apply_import(connection, &csv))
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_pyq_analytics(
    state: tauri::State<'_, db::AppState>,
) -> Result<analytics::PyqAnalytics, AppError> {
    state.with_connection(|connection| analytics::pyq(connection, Local::now().date_naive()))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_mock_tests(
    state: tauri::State<'_, db::AppState>,
) -> Result<Vec<mock_tests::MockTest>, AppError> {
    state.with_connection(|connection| mock_tests::list(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn create_mock_test(
    state: tauri::State<'_, db::AppState>,
    test: mock_tests::MockTestInput,
) -> Result<mock_tests::MockTest, AppError> {
    state.with_connection(|connection| mock_tests::create(connection, test))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_mock_test(
    state: tauri::State<'_, db::AppState>,
    test_id: i64,
    test: mock_tests::MockTestInput,
) -> Result<mock_tests::MockTest, AppError> {
    state.with_connection(|connection| mock_tests::update(connection, test_id, test))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn delete_mock_test(state: tauri::State<'_, db::AppState>, test_id: i64) -> Result<(), AppError> {
    state.with_connection(|connection| mock_tests::delete(connection, test_id))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn save_mock_csv_template(app: tauri::AppHandle) -> Result<String, AppError> {
    save_csv_template(
        &app,
        "csir-net-mock-test-template.csv",
        &mock_tests::template(),
    )
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn preview_mock_import(
    app: AppHandle,
    csv: String,
) -> Result<csv_import::ImportPreview, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| mock_tests::preview_import(connection, &csv))
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
async fn apply_mock_import(
    app: AppHandle,
    csv: String,
) -> Result<csv_import::ImportResult, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| mock_tests::apply_import(connection, &csv))
    })
    .await
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_mock_analytics(
    state: tauri::State<'_, db::AppState>,
) -> Result<analytics::MockAnalytics, AppError> {
    state.with_connection(|connection| analytics::mocks(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_study_performance(
    state: tauri::State<'_, db::AppState>,
) -> Result<analytics::StudyPerformance, AppError> {
    state.with_connection(|connection| analytics::study_performance(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_performance_analytics(
    state: tauri::State<'_, db::AppState>,
    from_date: String,
    to_date: String,
) -> Result<analytics::PerformanceAnalytics, AppError> {
    state.with_connection(|connection| {
        analytics::performance(connection, &from_date, &to_date, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_preparation_dashboard(
    state: tauri::State<'_, db::AppState>,
) -> Result<dashboard::PreparationDashboard, AppError> {
    state.with_connection(|connection| {
        dashboard::preparation_dashboard(connection, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_dashboard_layout(
    state: tauri::State<'_, db::AppState>,
    sections: Vec<dashboard::DashboardSectionInput>,
) -> Result<dashboard::DashboardLayout, AppError> {
    state.with_connection(|connection| dashboard::update_layout(connection, sections))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn reset_dashboard_layout(
    state: tauri::State<'_, db::AppState>,
) -> Result<dashboard::DashboardLayout, AppError> {
    state.with_connection(dashboard::reset_layout)
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_study_analytics(
    state: tauri::State<'_, db::AppState>,
    from_date: String,
    to_date: String,
) -> Result<study_analytics::StudyAnalytics, AppError> {
    state.with_connection(|connection| {
        study_analytics::study_analytics(
            connection,
            &from_date,
            &to_date,
            Local::now().date_naive(),
        )
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_topic_intelligence(
    state: tauri::State<'_, db::AppState>,
) -> Result<topic_intelligence::TopicIntelligence, AppError> {
    state.with_connection(|connection| {
        topic_intelligence::topic_intelligence(connection, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_preparation_timeline(
    state: tauri::State<'_, db::AppState>,
    from_date: String,
    to_date: String,
) -> Result<dashboard::PreparationTimeline, AppError> {
    state.with_connection(|connection| {
        dashboard::timeline(connection, &from_date, &to_date, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_revision_dashboard(
    state: tauri::State<'_, db::AppState>,
) -> Result<revision::RevisionDashboard, AppError> {
    state.with_connection(|connection| revision::dashboard(connection, Local::now().date_naive()))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_revision_settings(
    state: tauri::State<'_, db::AppState>,
    settings: revision::RevisionSettingsInput,
) -> Result<revision::RevisionSettings, AppError> {
    state.with_connection(|connection| revision::update_settings(connection, settings))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_revision_history(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
) -> Result<Vec<revision::RevisionHistoryEntry>, AppError> {
    state.with_connection(|connection| revision::history(connection, topic_id))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn schedule_revision_topic(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
    next_date: String,
) -> Result<revision::RevisionTopicState, AppError> {
    state.with_connection(|connection| {
        revision::schedule_topic(connection, topic_id, &next_date, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn reschedule_revision(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
    next_date: String,
) -> Result<revision::RevisionTopicState, AppError> {
    state.with_connection(|connection| {
        revision::reschedule(connection, topic_id, &next_date, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_revision_notes(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
    notes: String,
) -> Result<revision::RevisionTopicState, AppError> {
    state.with_connection(|connection| {
        revision::update_notes(connection, topic_id, &notes, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn suspend_revision(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
) -> Result<revision::RevisionTopicState, AppError> {
    state.with_connection(|connection| {
        revision::set_status(connection, topic_id, "suspended", Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn reactivate_revision(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
) -> Result<revision::RevisionTopicState, AppError> {
    state.with_connection(|connection| {
        revision::set_status(connection, topic_id, "active", Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn complete_revision(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
    input: revision::CompleteRevisionInput,
) -> Result<revision::RevisionCompletion, AppError> {
    state.with_connection(|connection| {
        revision::complete(connection, topic_id, input, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_revision_summaries(
    state: tauri::State<'_, db::AppState>,
) -> Result<Vec<revision::RevisionSummary>, AppError> {
    state.with_connection(|connection| revision::summaries(connection, Local::now().date_naive()))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn check_revision_reminder(
    app: tauri::AppHandle,
    state: tauri::State<'_, db::AppState>,
) -> Result<Option<revision::RevisionReminder>, AppError> {
    let reminder = state.with_connection(|connection| {
        revision::prepare_reminder(connection, Local::now().date_naive())
    })?;

    if let Some(reminder) = &reminder {
        use tauri_plugin_notification::NotificationExt;
        let body = if reminder.overdue > 0 && reminder.due_today > 0 {
            format!(
                "{} revisions need review: {} overdue and {} due today.",
                reminder.total, reminder.overdue, reminder.due_today
            )
        } else if reminder.overdue > 0 {
            format!(
                "{} revision{} overdue for review.",
                reminder.overdue,
                if reminder.overdue == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "{} revision{} due today.",
                reminder.due_today,
                if reminder.due_today == 1 { "" } else { "s" }
            )
        };
        if let Err(error) = app
            .notification()
            .builder()
            .title("Revision review due")
            .body(body)
            .show()
        {
            eprintln!("Revision reminder notification could not be shown: {error}");
        }
    }

    Ok(reminder)
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_revision_daily_plan(
    state: tauri::State<'_, db::AppState>,
) -> Result<revision_plan::RevisionDailyPlan, AppError> {
    state.with_connection(|connection| {
        revision_plan::get_daily_plan(connection, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn generate_revision_daily_plan(
    state: tauri::State<'_, db::AppState>,
) -> Result<revision_plan::RevisionDailyPlan, AppError> {
    state.with_connection(|connection| {
        revision_plan::generate_daily_plan(connection, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn rebalance_revision_daily_plan(
    state: tauri::State<'_, db::AppState>,
) -> Result<revision_plan::RevisionDailyPlan, AppError> {
    state.with_connection(|connection| {
        revision_plan::rebalance_daily_plan(connection, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_revision_plan_history(
    state: tauri::State<'_, db::AppState>,
    from_date: String,
    to_date: String,
) -> Result<Vec<revision_plan::RevisionPlanHistoryEntry>, AppError> {
    state
        .with_connection(|connection| revision_plan::plan_history(connection, &from_date, &to_date))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn add_revision_plan_item(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
) -> Result<revision_plan::RevisionDailyPlan, AppError> {
    state.with_connection(|connection| {
        revision_plan::add_revision_plan_item(connection, Local::now().date_naive(), topic_id)
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn move_revision_plan_item(
    state: tauri::State<'_, db::AppState>,
    plan_item_id: i64,
    target: String,
) -> Result<revision_plan::RevisionDailyPlan, AppError> {
    state.with_connection(|connection| {
        revision_plan::move_revision_plan_item(
            connection,
            Local::now().date_naive(),
            plan_item_id,
            &target,
        )
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn reorder_revision_plan_items(
    state: tauri::State<'_, db::AppState>,
    plan_id: i64,
    ordered_plan_item_ids: Vec<i64>,
) -> Result<revision_plan::RevisionDailyPlan, AppError> {
    state.with_connection(|connection| {
        revision_plan::reorder_revision_plan_items(
            connection,
            Local::now().date_naive(),
            plan_id,
            &ordered_plan_item_ids,
        )
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_revision_plan_estimate(
    state: tauri::State<'_, db::AppState>,
    plan_item_id: i64,
    estimated_minutes: i64,
) -> Result<revision_plan::RevisionDailyPlan, AppError> {
    state.with_connection(|connection| {
        revision_plan::update_revision_plan_estimate(
            connection,
            Local::now().date_naive(),
            plan_item_id,
            estimated_minutes,
        )
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn link_revision_plan_block(
    state: tauri::State<'_, db::AppState>,
    plan_item_id: i64,
    block_id: i64,
) -> Result<revision_plan::RevisionDailyPlan, AppError> {
    state.with_connection(|connection| {
        revision_plan::link_revision_plan_block(
            connection,
            Local::now().date_naive(),
            plan_item_id,
            block_id,
        )
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn defer_revision(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
    new_date: String,
    reason: String,
    notes: Option<String>,
) -> Result<revision_plan::RevisionDeferralResult, AppError> {
    state.with_connection(|connection| {
        revision_plan::defer_revision(
            connection,
            Local::now().date_naive(),
            topic_id,
            &new_date,
            &reason,
            notes.as_deref().unwrap_or(""),
        )
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn override_deferred_revision(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
    override_date: String,
    reason: String,
    notes: Option<String>,
) -> Result<revision_plan::RevisionDeferralResult, AppError> {
    state.with_connection(|connection| {
        revision_plan::override_deferred_revision(
            connection,
            Local::now().date_naive(),
            topic_id,
            &override_date,
            &reason,
            notes.as_deref().unwrap_or(""),
        )
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_revision_interval_preview(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
    outcome: String,
    revision_date: Option<String>,
) -> Result<revision_plan::RevisionIntervalDecision, AppError> {
    state.with_connection(|connection| {
        revision_plan::interval_preview(
            connection,
            Local::now().date_naive(),
            topic_id,
            &outcome,
            revision_date.as_deref(),
        )
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_planner_topics(
    state: tauri::State<'_, db::AppState>,
) -> Result<Vec<planner::PlannerTopic>, AppError> {
    state.with_connection(|connection| planner::list_topics(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_checklist_tasks(
    state: tauri::State<'_, db::AppState>,
    filter: planner::TaskFilter,
) -> Result<Vec<planner::ChecklistTask>, AppError> {
    state.with_connection(|connection| {
        planner::list_tasks(connection, filter, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn create_checklist_task(
    state: tauri::State<'_, db::AppState>,
    task: planner::TaskInput,
) -> Result<planner::ChecklistTask, AppError> {
    state.with_connection(|connection| {
        planner::create_task(connection, task, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_checklist_task(
    state: tauri::State<'_, db::AppState>,
    task_id: i64,
    task: planner::TaskInput,
) -> Result<planner::ChecklistTask, AppError> {
    state.with_connection(|connection| {
        planner::update_task(connection, task_id, task, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn complete_checklist_task(
    state: tauri::State<'_, db::AppState>,
    task_id: i64,
) -> Result<planner::ChecklistTask, AppError> {
    state.with_connection(|connection| {
        planner::complete_task(connection, task_id, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn reopen_checklist_task(
    state: tauri::State<'_, db::AppState>,
    task_id: i64,
) -> Result<planner::ChecklistTask, AppError> {
    state.with_connection(|connection| {
        planner::reopen_task(connection, task_id, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn reschedule_checklist_task(
    state: tauri::State<'_, db::AppState>,
    task_id: i64,
    scheduled_date: String,
) -> Result<planner::ChecklistTask, AppError> {
    state.with_connection(|connection| {
        planner::reschedule_task(
            connection,
            task_id,
            &scheduled_date,
            Local::now().date_naive(),
        )
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn delete_checklist_task(
    state: tauri::State<'_, db::AppState>,
    task_id: i64,
) -> Result<(), AppError> {
    state.with_connection(|connection| planner::delete_task(connection, task_id))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_calendar(
    state: tauri::State<'_, db::AppState>,
    from_date: String,
    to_date: String,
) -> Result<planner::CalendarData, AppError> {
    state.with_connection(|connection| {
        planner::calendar(connection, &from_date, &to_date, Local::now().date_naive())
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_agenda(state: tauri::State<'_, db::AppState>) -> Result<planner::Agenda, AppError> {
    state.with_connection(|connection| planner::agenda(connection, Local::now().date_naive()))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn create_planned_block(
    state: tauri::State<'_, db::AppState>,
    block: planner::PlannedBlockInput,
) -> Result<planner::PlannedBlock, AppError> {
    state.with_connection(|connection| planner::create_block(connection, block))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_planned_block(
    state: tauri::State<'_, db::AppState>,
    block_id: i64,
    block: planner::PlannedBlockInput,
) -> Result<planner::PlannedBlock, AppError> {
    state.with_connection(|connection| planner::update_block(connection, block_id, block))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_planned_block_status(
    state: tauri::State<'_, db::AppState>,
    block_id: i64,
    status: String,
) -> Result<planner::PlannedBlock, AppError> {
    state.with_connection(|connection| planner::update_block_status(connection, block_id, &status))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn link_planned_block_session(
    state: tauri::State<'_, db::AppState>,
    block_id: i64,
    session_id: Option<i64>,
) -> Result<planner::PlannedBlock, AppError> {
    state
        .with_connection(|connection| planner::link_block_session(connection, block_id, session_id))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn delete_planned_block(
    state: tauri::State<'_, db::AppState>,
    block_id: i64,
) -> Result<(), AppError> {
    state.with_connection(|connection| planner::delete_block(connection, block_id))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_syllabus_catalog(
    state: tauri::State<'_, db::AppState>,
) -> Result<syllabus::SyllabusCatalog, AppError> {
    state.with_connection(|connection| syllabus::load_catalog(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn search_syllabus_topics(
    state: tauri::State<'_, db::AppState>,
    filter: syllabus::SyllabusFilter,
) -> Result<Vec<syllabus::SyllabusTopic>, AppError> {
    state.with_connection(|connection| syllabus::search_topics(connection, filter))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_topic_progress(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
) -> Result<syllabus::TopicProgress, AppError> {
    state.with_connection(|connection| syllabus::load_progress(connection, topic_id))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_topic_progress(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
    progress: syllabus::TopicProgressInput,
) -> Result<syllabus::TopicProgress, AppError> {
    state.with_connection(|connection| syllabus::update_progress(connection, topic_id, progress))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_syllabus_summary(
    state: tauri::State<'_, db::AppState>,
) -> Result<syllabus::SyllabusSummary, AppError> {
    state.with_connection(|connection| syllabus::summary(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_timer_snapshot(
    state: tauri::State<'_, db::AppState>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| timer::snapshot(connection, Utc::now()))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
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

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn start_study_timer(
    state: tauri::State<'_, db::AppState>,
    mode: String,
    topic_id: Option<i64>,
    plan_item_id: Option<i64>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| {
        let now = Utc::now();
        timer::start_with_plan(
            connection,
            now,
            Local::now().date_naive(),
            &mode,
            topic_id,
            plan_item_id,
        )
    })
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn pause_study_timer(
    state: tauri::State<'_, db::AppState>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| timer::pause(connection, Utc::now()))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn resume_study_timer(
    state: tauri::State<'_, db::AppState>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| timer::resume(connection, Utc::now()))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn finish_study_timer(
    state: tauri::State<'_, db::AppState>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| timer::finish(connection, Utc::now()))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn discard_study_timer(
    state: tauri::State<'_, db::AppState>,
) -> Result<timer::TimerSnapshot, AppError> {
    state.with_connection(|connection| timer::discard(connection, Utc::now()))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_study_sessions(
    state: tauri::State<'_, db::AppState>,
    filter: sessions::StudySessionFilter,
) -> Result<Vec<sessions::StudySession>, AppError> {
    state.with_connection(|connection| sessions::list(connection, filter))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn get_session_topics(
    state: tauri::State<'_, db::AppState>,
) -> Result<Vec<sessions::SessionTopic>, AppError> {
    state.with_connection(|connection| sessions::list_topics(connection))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn create_manual_session(
    state: tauri::State<'_, db::AppState>,
    session: sessions::StudySessionInput,
) -> Result<sessions::StudySession, AppError> {
    state.with_connection(|connection| sessions::create_manual(connection, session))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn update_study_session(
    state: tauri::State<'_, db::AppState>,
    session_id: i64,
    session: sessions::StudySessionInput,
) -> Result<sessions::StudySession, AppError> {
    state.with_connection(|connection| sessions::update(connection, session_id, session))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
#[tauri::command]
fn delete_study_session(
    state: tauri::State<'_, db::AppState>,
    session_id: i64,
) -> Result<(), AppError> {
    state.with_connection(|connection| sessions::delete(connection, session_id))
}

#[allow(dead_code)] // Legacy V1 command retained but not registered in the V2-only binary.
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let resolved = app
                .path()
                .data_dir()
                .map_err(|error| {
                    eprintln!("Platform data directory could not be resolved: {error}");
                    AppError::database_unavailable(
                        "The platform data folder could not be located. Restart the app; \
                         if the problem continues, check your Windows user profile.",
                    )
                })
                .and_then(|base| app_paths::resolve_v2_data_dir(&base));

            let v2_state = match resolved {
                Ok(v2_data_dir) => V2State::initialize(&v2_data_dir),
                Err(error) => V2State::unavailable(error),
            };

            let _ = app.manage(v2_state);
            if app.state::<V2State>().status().initialized {
                v2::study::start_heartbeat(app.handle().clone());
                let backup_handle = app.handle().clone();
                std::thread::spawn(move || {
                    let state = backup_handle.state::<V2State>();
                    if let Some(data_dir) = state.data_directory()
                        && let Err(error) = state.with_connection(|connection| {
                            crate::v2::backup::run_automatic_backup_if_due(
                                connection,
                                &data_dir,
                                Utc::now(),
                            )
                            .map(|_| ())
                        })
                    {
                        eprintln!("V2 automatic backup check failed: {error:?}");
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            v2_get_database_status,
            v2_initialize_database,
            v2_list_exams,
            v2_check_integrity,
            v2_get_timer,
            v2_start_timer,
            v2_set_timer_allocations,
            v2_pause_timer,
            v2_resume_timer,
            v2_finish_timer,
            v2_discard_timer,
            v2_heartbeat_timer,
            v2_update_timer_settings,
            v2_list_sessions,
            v2_set_session_allocations,
            v2_get_study_totals,
            v2_search_concepts,
            v2_get_concept_detail,
            v2_update_concept_mapping,
            v2_update_concept_mastery,
            v2_replay_concept_mastery,
            v2_verify_mastery_run,
            v2_get_app_settings,
            v2_update_app_settings,
            v2_list_exam_profiles,
            v2_set_exam_active,
            v2_preview_exam_date_change,
            v2_apply_exam_date_change,
            v2_create_manual_session,
            v2_update_study_session,
            v2_delete_study_session,
            v2_get_planner_day,
            v2_get_agenda,
            v2_save_task,
            v2_set_task_status,
            v2_reschedule_task,
            v2_delete_task,
            v2_save_block,
            v2_set_block_status,
            v2_link_block_session,
            v2_delete_block,
            v2_list_resources,
            v2_save_resource,
            v2_archive_resource,
            v2_update_resource_progress,
            v2_link_resource_exam,
            v2_import_resource_file,
            v2_remove_resource_file,
            v2_get_backup_overview,
            v2_update_backup_settings,
            v2_create_manual_backup,
            v2_run_automatic_backup_if_due,
            v2_preview_restore_backup,
            v2_apply_restore_backup,
            v2_get_restore_recovery,
            v2_restore_recovery_safety,
            v2_get_exam_syllabus,
            v2_update_syllabus_node_progress,
            v2_get_revision_dashboard,
            v2_update_revision_settings,
            v2_schedule_revision_concept,
            v2_reschedule_revision,
            v2_set_revision_status,
            v2_complete_revision,
            v2_defer_revision,
            v2_get_revision_daily_plan,
            v2_generate_revision_daily_plan,
            v2_rebalance_revision_daily_plan,
            v2_move_revision_plan_item,
            v2_link_revision_plan_block,
            v2_get_analytics,
            v2_get_dashboard_sections,
            v2_save_dashboard_sections,
            v2_reset_dashboard_sections,
            v2_list_quick_start_configs,
            v2_save_quick_start_config,
            v2_delete_quick_start_config,
            v2_global_search,
            v2_list_flashcards,
            v2_get_flashcard_detail,
            v2_save_flashcard,
            v2_set_flashcard_archived,
            v2_get_flashcard_review_queue,
            v2_review_flashcard,
            v2_get_flashcard_stats,
            v2_get_practice_options,
            v2_list_pyq_questions,
            v2_get_pyq_question_detail,
            v2_save_pyq_question,
            v2_archive_pyq_question,
            v2_record_pyq_attempt,
            v2_list_pyq_attempts,
            v2_preview_pyq_import,
            v2_apply_pyq_import,
            v2_list_mock_tests,
            v2_save_mock_test,
            v2_set_mock_test_status,
            v2_archive_mock_test,
            v2_get_practice_analytics,
            v2_list_question_mock_candidates,
            v2_start_question_mock,
            v2_get_question_mock_session,
            v2_save_question_mock_state,
            v2_submit_question_mock,
            v2_list_pyq_sources,
            v2_get_pyq_source_detail,
            v2_save_pyq_source,
            v2_acquire_pyq_source,
            v2_set_pyq_source_status,
            v2_preview_pyq_answer_key,
            v2_list_pyq_answer_keys,
            v2_get_pyq_answer_key,
            v2_save_pyq_answer_key,
            v2_list_pyq_answer_key_conflicts,
            v2_extract_pyq_source,
            v2_reconcile_pyq_source,
            v2_list_pyq_extraction_candidates,
            v2_get_pyq_extraction_candidate,
            v2_update_pyq_extraction_candidate,
            v2_verify_pyq_extraction_candidate,
            v2_reject_pyq_extraction_candidate,
            v2_get_pyq_source_counts
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Physics Tracker application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_exposes_the_product_identity() {
        let info = app_info();

        assert_eq!(info.name, "Physics Tracker");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn v2_startup_source_does_not_initialize_or_heartbeat_v1() {
        let source = include_str!("lib.rs");
        let run_source = source.split("pub fn run()").nth(1).expect("run function");
        assert!(run_source.contains("resolve_v2_data_dir"));
        assert!(!run_source.contains("db::AppState::initialize"));
        assert!(!run_source.contains("db::AppState::unavailable"));
        assert!(!run_source.contains("timer::start_heartbeat"));
        assert!(!run_source.contains("app.manage(state)"));
    }

    #[test]
    fn v2_handler_registers_only_v2_commands() {
        let source = include_str!("lib.rs");
        let handler = source
            .split("tauri::generate_handler![")
            .nth(1)
            .expect("handler");
        let handler = handler.split("])").next().expect("handler end");
        for command in handler
            .lines()
            .map(str::trim)
            .map(|line| line.trim_end_matches(','))
            .filter(|line| !line.is_empty())
        {
            assert!(
                command.starts_with("v2_"),
                "V2 handler must not register V1 command {command}"
            );
        }
    }

    #[test]
    fn csv_template_writer_avoids_overwriting_existing_files() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let nested = directory.path().join("downloads");
        let first = write_csv_template(&nested, "template.csv", "a,b\r\n")
            .expect("template should be written");
        let second = write_csv_template(&nested, "template.csv", "c,d\r\n")
            .expect("second template should be written");

        assert_eq!(first, nested.join("template.csv"));
        assert_eq!(second, nested.join("template (1).csv"));
        assert_eq!(
            std::fs::read_to_string(&first).expect("first template should be readable"),
            "a,b\r\n"
        );
        assert_eq!(
            std::fs::read_to_string(&second).expect("second template should be readable"),
            "c,d\r\n"
        );
    }
}
