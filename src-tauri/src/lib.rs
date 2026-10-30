mod analytics;
mod backup;
mod csv_import;
mod dashboard;
mod date_only;
mod db;
mod error;
mod mock_tests;
mod planner;
mod portability;
mod pyq;
mod reschedule;
mod revision;
mod sessions;
mod settings;
mod study_analytics;
mod syllabus;
mod syllabus_catalog;
mod timer;
mod topic_intelligence;

use std::path::Path;

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
fn get_backup_overview(
    state: tauri::State<'_, db::AppState>,
) -> Result<backup::BackupOverview, AppError> {
    let data_dir = state.data_dir()?.to_path_buf();
    state.with_connection(|connection| backup::overview(connection, &data_dir))
}

#[tauri::command]
fn update_backup_settings(
    state: tauri::State<'_, db::AppState>,
    settings: backup::BackupSettingsInput,
) -> Result<backup::BackupSettings, AppError> {
    state.with_connection(|connection| backup::update_settings(connection, settings))
}

#[tauri::command]
fn create_manual_backup(
    state: tauri::State<'_, db::AppState>,
    destination: Option<String>,
) -> Result<backup::BackupResult, AppError> {
    let data_dir = state.data_dir()?.to_path_buf();
    state.with_connection(|connection| {
        backup::create_manual_backup(connection, destination.as_deref(), &data_dir)
    })
}

#[tauri::command]
fn run_automatic_backup_if_due(
    state: tauri::State<'_, db::AppState>,
) -> Result<backup::AutoBackupOutcome, AppError> {
    let data_dir = state.data_dir()?.to_path_buf();
    state.with_connection(|connection| backup::automatic_backup(connection, &data_dir, Utc::now()))
}

#[tauri::command]
fn preview_restore_backup(
    state: tauri::State<'_, db::AppState>,
    path: String,
) -> Result<backup::RestorePreview, AppError> {
    let current_path = state.database_path().map(Path::to_path_buf);
    state.with_connection(|connection| {
        backup::preview_restore(
            Path::new(&path),
            backup::schema_version(connection)?,
            current_path.as_deref(),
        )
    })
}

#[tauri::command]
fn apply_restore_backup(
    state: tauri::State<'_, db::AppState>,
    path: String,
) -> Result<backup::BackupResult, AppError> {
    state.restore_from_backup(Path::new(&path), Local::now().date_naive())
}

#[tauri::command]
fn export_csv_data(
    state: tauri::State<'_, db::AppState>,
    destination: String,
) -> Result<portability::CsvExportResult, AppError> {
    state.with_connection(|connection| {
        portability::export_csv_datasets(connection, Path::new(&destination))
    })
}

#[tauri::command]
fn export_json_data(
    state: tauri::State<'_, db::AppState>,
    destination: String,
) -> Result<portability::JsonExportResult, AppError> {
    state
        .with_connection(|connection| portability::export_json(connection, Path::new(&destination)))
}

#[tauri::command]
fn preview_json_import(
    state: tauri::State<'_, db::AppState>,
    path: String,
) -> Result<portability::JsonImportPreview, AppError> {
    state.with_connection(|connection| {
        portability::preview_json_import(connection, Path::new(&path))
    })
}

#[tauri::command]
fn apply_json_import(
    state: tauri::State<'_, db::AppState>,
    path: String,
    strategy: portability::JsonImportStrategy,
    expected_sha256: String,
) -> Result<portability::JsonImportResult, AppError> {
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
    state.with_connection(|connection| {
        crate::settings::update(connection, settings, Local::now().date_naive())
    })
}

#[tauri::command]
fn preview_exam_date_change(
    state: tauri::State<'_, db::AppState>,
    settings: settings::SettingsInput,
) -> Result<reschedule::ExamReschedulePreview, AppError> {
    state.with_connection(|connection| {
        reschedule::preview(connection, settings, Local::now().date_naive())
    })
}

#[tauri::command]
fn apply_exam_date_change(
    state: tauri::State<'_, db::AppState>,
    change: reschedule::ApplyExamDateChangeInput,
) -> Result<reschedule::ApplyExamDateChangeResult, AppError> {
    state.with_connection(|connection| {
        reschedule::apply(connection, change, Local::now().date_naive())
    })
}

#[tauri::command]
fn get_pyq_options(state: tauri::State<'_, db::AppState>) -> Result<pyq::PyqOptions, AppError> {
    state.with_connection(|connection| pyq::options(connection))
}

#[tauri::command]
fn get_pyq_attempts(
    state: tauri::State<'_, db::AppState>,
    filter: pyq::PyqFilter,
) -> Result<Vec<pyq::PyqAttempt>, AppError> {
    state.with_connection(|connection| pyq::list(connection, filter))
}

#[tauri::command]
fn create_pyq_attempt(
    state: tauri::State<'_, db::AppState>,
    attempt: pyq::PyqInput,
) -> Result<pyq::PyqAttempt, AppError> {
    state.with_connection(|connection| pyq::create(connection, attempt))
}

#[tauri::command]
fn update_pyq_attempt(
    state: tauri::State<'_, db::AppState>,
    attempt_id: i64,
    attempt: pyq::PyqInput,
) -> Result<pyq::PyqAttempt, AppError> {
    state.with_connection(|connection| pyq::update(connection, attempt_id, attempt))
}

#[tauri::command]
fn delete_pyq_attempt(
    state: tauri::State<'_, db::AppState>,
    attempt_id: i64,
) -> Result<(), AppError> {
    state.with_connection(|connection| pyq::delete(connection, attempt_id))
}

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

#[tauri::command]
fn preview_pyq_import(
    state: tauri::State<'_, db::AppState>,
    csv: String,
) -> Result<csv_import::ImportPreview, AppError> {
    state.with_connection(|connection| pyq::preview_import(connection, &csv))
}

#[tauri::command]
fn apply_pyq_import(
    state: tauri::State<'_, db::AppState>,
    csv: String,
) -> Result<csv_import::ImportResult, AppError> {
    state.with_connection(|connection| pyq::apply_import(connection, &csv))
}

#[tauri::command]
fn get_pyq_analytics(
    state: tauri::State<'_, db::AppState>,
) -> Result<analytics::PyqAnalytics, AppError> {
    state.with_connection(|connection| analytics::pyq(connection))
}

#[tauri::command]
fn get_mock_tests(
    state: tauri::State<'_, db::AppState>,
) -> Result<Vec<mock_tests::MockTest>, AppError> {
    state.with_connection(|connection| mock_tests::list(connection))
}

#[tauri::command]
fn create_mock_test(
    state: tauri::State<'_, db::AppState>,
    test: mock_tests::MockTestInput,
) -> Result<mock_tests::MockTest, AppError> {
    state.with_connection(|connection| mock_tests::create(connection, test))
}

#[tauri::command]
fn update_mock_test(
    state: tauri::State<'_, db::AppState>,
    test_id: i64,
    test: mock_tests::MockTestInput,
) -> Result<mock_tests::MockTest, AppError> {
    state.with_connection(|connection| mock_tests::update(connection, test_id, test))
}

#[tauri::command]
fn delete_mock_test(state: tauri::State<'_, db::AppState>, test_id: i64) -> Result<(), AppError> {
    state.with_connection(|connection| mock_tests::delete(connection, test_id))
}

#[tauri::command]
fn save_mock_csv_template(app: tauri::AppHandle) -> Result<String, AppError> {
    save_csv_template(
        &app,
        "csir-net-mock-test-template.csv",
        &mock_tests::template(),
    )
}

#[tauri::command]
fn preview_mock_import(
    state: tauri::State<'_, db::AppState>,
    csv: String,
) -> Result<csv_import::ImportPreview, AppError> {
    state.with_connection(|connection| mock_tests::preview_import(connection, &csv))
}

#[tauri::command]
fn apply_mock_import(
    state: tauri::State<'_, db::AppState>,
    csv: String,
) -> Result<csv_import::ImportResult, AppError> {
    state.with_connection(|connection| mock_tests::apply_import(connection, &csv))
}

#[tauri::command]
fn get_mock_analytics(
    state: tauri::State<'_, db::AppState>,
) -> Result<analytics::MockAnalytics, AppError> {
    state.with_connection(|connection| analytics::mocks(connection))
}

#[tauri::command]
fn get_study_performance(
    state: tauri::State<'_, db::AppState>,
) -> Result<analytics::StudyPerformance, AppError> {
    state.with_connection(|connection| analytics::study_performance(connection))
}

#[tauri::command]
fn get_performance_analytics(
    state: tauri::State<'_, db::AppState>,
    from_date: String,
    to_date: String,
) -> Result<analytics::PerformanceAnalytics, AppError> {
    state.with_connection(|connection| analytics::performance(connection, &from_date, &to_date))
}

#[tauri::command]
fn get_preparation_dashboard(
    state: tauri::State<'_, db::AppState>,
) -> Result<dashboard::PreparationDashboard, AppError> {
    state.with_connection(|connection| {
        dashboard::preparation_dashboard(connection, Local::now().date_naive())
    })
}

#[tauri::command]
fn update_dashboard_layout(
    state: tauri::State<'_, db::AppState>,
    sections: Vec<dashboard::DashboardSectionInput>,
) -> Result<dashboard::DashboardLayout, AppError> {
    state.with_connection(|connection| dashboard::update_layout(connection, sections))
}

#[tauri::command]
fn reset_dashboard_layout(
    state: tauri::State<'_, db::AppState>,
) -> Result<dashboard::DashboardLayout, AppError> {
    state.with_connection(dashboard::reset_layout)
}

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

#[tauri::command]
fn get_topic_intelligence(
    state: tauri::State<'_, db::AppState>,
) -> Result<topic_intelligence::TopicIntelligence, AppError> {
    state.with_connection(|connection| {
        topic_intelligence::topic_intelligence(connection, Local::now().date_naive())
    })
}

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

#[tauri::command]
fn get_revision_dashboard(
    state: tauri::State<'_, db::AppState>,
) -> Result<revision::RevisionDashboard, AppError> {
    state.with_connection(|connection| revision::dashboard(connection, Local::now().date_naive()))
}

#[tauri::command]
fn update_revision_settings(
    state: tauri::State<'_, db::AppState>,
    settings: revision::RevisionSettingsInput,
) -> Result<revision::RevisionSettings, AppError> {
    state.with_connection(|connection| revision::update_settings(connection, settings))
}

#[tauri::command]
fn get_revision_history(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
) -> Result<Vec<revision::RevisionHistoryEntry>, AppError> {
    state.with_connection(|connection| revision::history(connection, topic_id))
}

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

#[tauri::command]
fn suspend_revision(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
) -> Result<revision::RevisionTopicState, AppError> {
    state.with_connection(|connection| {
        revision::set_status(connection, topic_id, "suspended", Local::now().date_naive())
    })
}

#[tauri::command]
fn reactivate_revision(
    state: tauri::State<'_, db::AppState>,
    topic_id: i64,
) -> Result<revision::RevisionTopicState, AppError> {
    state.with_connection(|connection| {
        revision::set_status(connection, topic_id, "active", Local::now().date_naive())
    })
}

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

#[tauri::command]
fn get_revision_summaries(
    state: tauri::State<'_, db::AppState>,
) -> Result<Vec<revision::RevisionSummary>, AppError> {
    state.with_connection(|connection| revision::summaries(connection, Local::now().date_naive()))
}

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

#[tauri::command]
fn get_planner_topics(
    state: tauri::State<'_, db::AppState>,
) -> Result<Vec<planner::PlannerTopic>, AppError> {
    state.with_connection(|connection| planner::list_topics(connection))
}

#[tauri::command]
fn get_checklist_tasks(
    state: tauri::State<'_, db::AppState>,
    filter: planner::TaskFilter,
) -> Result<Vec<planner::ChecklistTask>, AppError> {
    state.with_connection(|connection| {
        planner::list_tasks(connection, filter, Local::now().date_naive())
    })
}

#[tauri::command]
fn create_checklist_task(
    state: tauri::State<'_, db::AppState>,
    task: planner::TaskInput,
) -> Result<planner::ChecklistTask, AppError> {
    state.with_connection(|connection| {
        planner::create_task(connection, task, Local::now().date_naive())
    })
}

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

#[tauri::command]
fn complete_checklist_task(
    state: tauri::State<'_, db::AppState>,
    task_id: i64,
) -> Result<planner::ChecklistTask, AppError> {
    state.with_connection(|connection| {
        planner::complete_task(connection, task_id, Local::now().date_naive())
    })
}

#[tauri::command]
fn reopen_checklist_task(
    state: tauri::State<'_, db::AppState>,
    task_id: i64,
) -> Result<planner::ChecklistTask, AppError> {
    state.with_connection(|connection| {
        planner::reopen_task(connection, task_id, Local::now().date_naive())
    })
}

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

#[tauri::command]
fn delete_checklist_task(
    state: tauri::State<'_, db::AppState>,
    task_id: i64,
) -> Result<(), AppError> {
    state.with_connection(|connection| planner::delete_task(connection, task_id))
}

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

#[tauri::command]
fn get_agenda(state: tauri::State<'_, db::AppState>) -> Result<planner::Agenda, AppError> {
    state.with_connection(|connection| planner::agenda(connection, Local::now().date_naive()))
}

#[tauri::command]
fn create_planned_block(
    state: tauri::State<'_, db::AppState>,
    block: planner::PlannedBlockInput,
) -> Result<planner::PlannedBlock, AppError> {
    state.with_connection(|connection| planner::create_block(connection, block))
}

#[tauri::command]
fn update_planned_block(
    state: tauri::State<'_, db::AppState>,
    block_id: i64,
    block: planner::PlannedBlockInput,
) -> Result<planner::PlannedBlock, AppError> {
    state.with_connection(|connection| planner::update_block(connection, block_id, block))
}

#[tauri::command]
fn update_planned_block_status(
    state: tauri::State<'_, db::AppState>,
    block_id: i64,
    status: String,
) -> Result<planner::PlannedBlock, AppError> {
    state.with_connection(|connection| planner::update_block_status(connection, block_id, &status))
}

#[tauri::command]
fn link_planned_block_session(
    state: tauri::State<'_, db::AppState>,
    block_id: i64,
    session_id: Option<i64>,
) -> Result<planner::PlannedBlock, AppError> {
    state
        .with_connection(|connection| planner::link_block_session(connection, block_id, session_id))
}

#[tauri::command]
fn delete_planned_block(
    state: tauri::State<'_, db::AppState>,
    block_id: i64,
) -> Result<(), AppError> {
    state.with_connection(|connection| planner::delete_block(connection, block_id))
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
        .plugin(tauri_plugin_dialog::init())
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
            get_backup_overview,
            update_backup_settings,
            create_manual_backup,
            run_automatic_backup_if_due,
            preview_restore_backup,
            apply_restore_backup,
            export_csv_data,
            export_json_data,
            preview_json_import,
            apply_json_import,
            get_settings,
            update_settings,
            preview_exam_date_change,
            apply_exam_date_change,
            get_pyq_options,
            get_pyq_attempts,
            create_pyq_attempt,
            update_pyq_attempt,
            delete_pyq_attempt,
            save_pyq_csv_template,
            preview_pyq_import,
            apply_pyq_import,
            get_pyq_analytics,
            get_mock_tests,
            create_mock_test,
            update_mock_test,
            delete_mock_test,
            save_mock_csv_template,
            preview_mock_import,
            apply_mock_import,
            get_mock_analytics,
            get_study_performance,
            get_performance_analytics,
            get_preparation_dashboard,
            update_dashboard_layout,
            reset_dashboard_layout,
            get_study_analytics,
            get_topic_intelligence,
            get_preparation_timeline,
            get_revision_dashboard,
            update_revision_settings,
            get_revision_history,
            schedule_revision_topic,
            reschedule_revision,
            update_revision_notes,
            suspend_revision,
            reactivate_revision,
            complete_revision,
            get_revision_summaries,
            check_revision_reminder,
            get_planner_topics,
            get_checklist_tasks,
            create_checklist_task,
            update_checklist_task,
            complete_checklist_task,
            reopen_checklist_task,
            reschedule_checklist_task,
            delete_checklist_task,
            get_calendar,
            get_agenda,
            create_planned_block,
            update_planned_block,
            update_planned_block_status,
            link_planned_block_session,
            delete_planned_block,
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
