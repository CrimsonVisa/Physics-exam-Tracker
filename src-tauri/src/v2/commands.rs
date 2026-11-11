use chrono::{Local, Utc};
use tauri::{AppHandle, Manager};

use crate::{
    error::AppError,
    v2::{
        V2DatabaseStatus, V2ExamSummary, V2IntegrityReport, V2State,
        admin::{
            V2AppSettings, V2ExamActivationInput, V2ExamDateChangeInput, V2ExamDateChangePreview,
            V2ExamProfile, V2UpdateAppSettingsInput,
        },
        analytics::{V2Analytics, V2AnalyticsInput},
        backup::{
            V2BackupOverview, V2BackupRecord, V2BackupSettings, V2BackupSettingsInput,
            V2RestoreInput, V2RestorePreview, V2RestoreRecovery, V2RestoreResult,
        },
        concepts::{
            V2ConceptDetail, V2ConceptDetailInput, V2ConceptMappingUpdateInput,
            V2ConceptSearchFilter, V2ConceptSearchResult,
        },
        dashboard::{V2DashboardLayoutInput, V2DashboardLayoutScope, V2DashboardSection},
        mastery::{
            V2MasteryReplayVerification, V2MasteryUpdate, V2ReplayConceptMasteryInput,
            V2UpdateConceptMasteryInput, V2VerifyMasteryRunInput,
        },
        planner::{
            V2BlockStatusInput, V2LinkBlockSessionInput, V2PlannerBlock, V2PlannerBlockInput,
            V2PlannerDay, V2PlannerDayInput, V2PlannerTask, V2PlannerTaskInput,
            V2RescheduleTaskInput, V2TaskStatusInput,
        },
        practice::{
            V2MockTest, V2MockTestInput, V2PracticeAnalytics, V2PracticeAnalyticsInput,
            V2PracticeExamInput, V2PracticeImportPreview, V2PracticeImportResult,
            V2PracticeOptions, V2PyqAttempt, V2PyqAttemptInput, V2PyqFilter, V2PyqQuestionDetail,
            V2PyqQuestionInput,
            question_mock::{
                V2QuestionMockCandidate, V2QuestionMockSaveResult, V2QuestionMockSession,
                V2SaveQuestionMockInput, V2StartQuestionMockInput, V2SubmitQuestionMockInput,
            },
            sources::{
                V2AcquireSourceInput, V2AnswerKeyConflict, V2AnswerKeyPreview,
                V2CandidateDecisionInput, V2ExtractionCandidate, V2ExtractionCandidateFilter,
                V2ExtractionCandidateUpdate, V2ExtractionInput, V2ExtractionResult, V2PyqAnswerKey,
                V2PyqAnswerKeyInput, V2PyqSource, V2PyqSourceAcquisition, V2PyqSourceFilter,
                V2PyqSourceInput, V2SourceCounts, V2SourceDetail, V2SourceStatusInput,
            },
        },
        resources::{
            V2Resource, V2ResourceArchiveInput, V2ResourceExamLinkCommandInput,
            V2ResourceFileInput, V2ResourceInput, V2ResourceProgressInput, V2ResourceSearchFilter,
        },
        revision::{
            V2CompleteRevisionInput, V2DeferRevisionInput, V2LinkRevisionBlockInput,
            V2MoveRevisionPlanItemInput, V2RevisionDailyPlan, V2RevisionDashboard,
            V2RevisionDashboardInput, V2RevisionPlanInput, V2RevisionRescheduleInput,
            V2RevisionSchedule, V2RevisionScheduleInput, V2RevisionSettings,
            V2RevisionSettingsInput, V2RevisionStatusInput,
        },
        study::{
            V2CreateManualSessionInput, V2SetSessionAllocationsInput, V2SetTimerAllocationsInput,
            V2StartTimerInput, V2StudySession, V2StudyTotals, V2TimerSettingsInput,
            V2TimerSnapshot, V2UpdateSessionInput,
        },
        syllabus::{V2ExamSyllabus, V2SyllabusExamInput, V2SyllabusProgressInput},
    },
};

async fn run_blocking<T, F>(app: AppHandle, operation: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&V2State) -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<V2State>();
        operation(state.inner())
    })
    .await
    .map_err(|_| {
        AppError::internal(
            "The V2 database operation ended unexpectedly. Restart the app and try again.",
        )
    })?
}

pub(crate) fn database_status(state: &V2State) -> V2DatabaseStatus {
    state.status()
}

pub(crate) fn initialize_database(state: &V2State) -> Result<V2DatabaseStatus, AppError> {
    state.retry_initialize()?;
    Ok(state.status())
}

pub(crate) fn exam_list(state: &V2State) -> Result<Vec<V2ExamSummary>, AppError> {
    state.list_exams()
}

pub(crate) fn database_integrity(
    state: &V2State,
    full: bool,
) -> Result<V2IntegrityReport, AppError> {
    state.check_integrity(full)
}

#[tauri::command]
pub async fn v2_get_database_status(app: AppHandle) -> Result<V2DatabaseStatus, AppError> {
    run_blocking(app, |state| Ok(database_status(state))).await
}

#[tauri::command]
pub async fn v2_initialize_database(app: AppHandle) -> Result<V2DatabaseStatus, AppError> {
    run_blocking(app, initialize_database).await
}

#[tauri::command]
pub async fn v2_list_exams(app: AppHandle) -> Result<Vec<V2ExamSummary>, AppError> {
    run_blocking(app, exam_list).await
}

#[tauri::command]
pub async fn v2_check_integrity(app: AppHandle, full: bool) -> Result<V2IntegrityReport, AppError> {
    run_blocking(app, move |state| database_integrity(state, full)).await
}

#[tauri::command]
pub async fn v2_get_timer(app: AppHandle) -> Result<V2TimerSnapshot, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::study::get_timer(connection, Utc::now()))
    })
    .await
}

#[tauri::command]
pub async fn v2_start_timer(
    app: AppHandle,
    input: V2StartTimerInput,
) -> Result<V2TimerSnapshot, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::study::start_timer(connection, Utc::now(), input))
    })
    .await
}

#[tauri::command]
pub async fn v2_set_timer_allocations(
    app: AppHandle,
    input: V2SetTimerAllocationsInput,
) -> Result<V2TimerSnapshot, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::study::set_timer_allocations(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_pause_timer(app: AppHandle) -> Result<V2TimerSnapshot, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::study::pause_timer(connection, Utc::now()))
    })
    .await
}

#[tauri::command]
pub async fn v2_resume_timer(app: AppHandle) -> Result<V2TimerSnapshot, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::study::resume_timer(connection, Utc::now()))
    })
    .await
}

#[tauri::command]
pub async fn v2_finish_timer(app: AppHandle) -> Result<V2TimerSnapshot, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::study::finish_timer(connection, Utc::now()))
    })
    .await
}

#[tauri::command]
pub async fn v2_discard_timer(app: AppHandle) -> Result<V2TimerSnapshot, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::study::discard_timer(connection, Utc::now()))
    })
    .await
}

#[tauri::command]
pub async fn v2_heartbeat_timer(app: AppHandle) -> Result<V2TimerSnapshot, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::study::heartbeat_timer(connection, Utc::now()))
    })
    .await
}

#[tauri::command]
pub async fn v2_update_timer_settings(
    app: AppHandle,
    settings: V2TimerSettingsInput,
) -> Result<V2TimerSnapshot, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::study::update_settings(connection, settings, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_list_sessions(app: AppHandle) -> Result<Vec<V2StudySession>, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::study::list_sessions(connection))
    })
    .await
}

#[tauri::command]
pub async fn v2_set_session_allocations(
    app: AppHandle,
    input: V2SetSessionAllocationsInput,
) -> Result<V2StudySession, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::study::set_session_allocations(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_study_totals(app: AppHandle) -> Result<V2StudyTotals, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::study::study_totals(connection))
    })
    .await
}

#[tauri::command]
pub async fn v2_search_concepts(
    app: AppHandle,
    filter: V2ConceptSearchFilter,
) -> Result<V2ConceptSearchResult, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::concepts::search_concepts(connection, filter))
    })
    .await
}

#[tauri::command]
pub async fn v2_get_concept_detail(
    app: AppHandle,
    input: V2ConceptDetailInput,
) -> Result<V2ConceptDetail, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::concepts::get_concept_detail(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_update_concept_mapping(
    app: AppHandle,
    input: V2ConceptMappingUpdateInput,
) -> Result<V2ConceptDetail, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::concepts::update_mapping(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_update_concept_mastery(
    app: AppHandle,
    input: V2UpdateConceptMasteryInput,
) -> Result<V2MasteryUpdate, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::mastery::update_manual_mastery(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_replay_concept_mastery(
    app: AppHandle,
    input: V2ReplayConceptMasteryInput,
) -> Result<V2MasteryUpdate, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::mastery::replay_current(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_verify_mastery_run(
    app: AppHandle,
    input: V2VerifyMasteryRunInput,
) -> Result<V2MasteryReplayVerification, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::mastery::verify_mastery_run(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_get_app_settings(app: AppHandle) -> Result<V2AppSettings, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::admin::load_settings(connection))
    })
    .await
}

#[tauri::command]
pub async fn v2_update_app_settings(
    app: AppHandle,
    input: V2UpdateAppSettingsInput,
) -> Result<V2AppSettings, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::admin::update_settings(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_list_exam_profiles(app: AppHandle) -> Result<Vec<V2ExamProfile>, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::admin::list_exam_profiles(connection))
    })
    .await
}

#[tauri::command]
pub async fn v2_set_exam_active(
    app: AppHandle,
    input: V2ExamActivationInput,
) -> Result<V2ExamProfile, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::admin::set_exam_active(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_preview_exam_date_change(
    app: AppHandle,
    input: V2ExamDateChangeInput,
) -> Result<V2ExamDateChangePreview, AppError> {
    run_blocking(app, move |state| {
        state
            .with_connection(|connection| super::admin::preview_exam_date_change(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_apply_exam_date_change(
    app: AppHandle,
    input: V2ExamDateChangeInput,
) -> Result<V2ExamProfile, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::admin::apply_exam_date_change(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_create_manual_session(
    app: AppHandle,
    input: V2CreateManualSessionInput,
) -> Result<V2StudySession, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::study::create_manual_session(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_update_study_session(
    app: AppHandle,
    input: V2UpdateSessionInput,
) -> Result<V2StudySession, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::study::update_study_session(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_delete_study_session(app: AppHandle, session_id: i64) -> Result<(), AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::study::delete_study_session(connection, session_id)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_planner_day(
    app: AppHandle,
    input: V2PlannerDayInput,
) -> Result<V2PlannerDay, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::planner::get_planner_day(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_get_agenda(
    app: AppHandle,
    input: V2PlannerDayInput,
) -> Result<V2PlannerDay, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::planner::get_agenda(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_save_task(
    app: AppHandle,
    input: V2PlannerTaskInput,
) -> Result<V2PlannerTask, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::planner::save_task(connection, input, Utc::now()))
    })
    .await
}

#[tauri::command]
pub async fn v2_set_task_status(
    app: AppHandle,
    input: V2TaskStatusInput,
) -> Result<V2PlannerTask, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::planner::set_task_status(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_reschedule_task(
    app: AppHandle,
    input: V2RescheduleTaskInput,
) -> Result<V2PlannerTask, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::planner::reschedule_task(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_delete_task(app: AppHandle, task_id: i64) -> Result<(), AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::planner::delete_task(connection, task_id))
    })
    .await
}

#[tauri::command]
pub async fn v2_save_block(
    app: AppHandle,
    input: V2PlannerBlockInput,
) -> Result<V2PlannerBlock, AppError> {
    run_blocking(app, move |state| {
        state
            .with_connection(|connection| super::planner::save_block(connection, input, Utc::now()))
    })
    .await
}

#[tauri::command]
pub async fn v2_set_block_status(
    app: AppHandle,
    input: V2BlockStatusInput,
) -> Result<V2PlannerBlock, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::planner::set_block_status(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_link_block_session(
    app: AppHandle,
    input: V2LinkBlockSessionInput,
) -> Result<V2PlannerBlock, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::planner::link_block_session(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_delete_block(app: AppHandle, block_id: i64) -> Result<(), AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::planner::delete_block(connection, block_id))
    })
    .await
}

#[tauri::command]
pub async fn v2_list_resources(
    app: AppHandle,
    filter: V2ResourceSearchFilter,
) -> Result<Vec<V2Resource>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::resources::list_resources(connection, filter))
    })
    .await
}

#[tauri::command]
pub async fn v2_save_resource(
    app: AppHandle,
    input: V2ResourceInput,
) -> Result<V2Resource, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::resources::save_resource(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_archive_resource(
    app: AppHandle,
    input: V2ResourceArchiveInput,
) -> Result<V2Resource, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::resources::archive_resource(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_update_resource_progress(
    app: AppHandle,
    input: V2ResourceProgressInput,
) -> Result<V2Resource, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::resources::update_resource_progress(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_link_resource_exam(
    app: AppHandle,
    input: V2ResourceExamLinkCommandInput,
) -> Result<V2Resource, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::resources::link_resource_exam(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_import_resource_file(
    app: AppHandle,
    input: V2ResourceFileInput,
) -> Result<V2Resource, AppError> {
    run_blocking(app, move |state| {
        let data_dir = state.data_directory().ok_or_else(|| {
            AppError::database_unavailable("The V2 data directory is unavailable.")
        })?;
        state.with_connection(|connection| {
            super::resources::import_resource_file(connection, &data_dir, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_remove_resource_file(
    app: AppHandle,
    resource_id: i64,
) -> Result<V2Resource, AppError> {
    run_blocking(app, move |state| {
        let data_dir = state.data_directory().ok_or_else(|| {
            AppError::database_unavailable("The V2 data directory is unavailable.")
        })?;
        state.with_connection(|connection| {
            super::resources::remove_resource_file(connection, &data_dir, resource_id, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_backup_overview(app: AppHandle) -> Result<V2BackupOverview, AppError> {
    run_blocking(app, |state| {
        let data_dir = state.data_directory().ok_or_else(|| {
            AppError::database_unavailable("The V2 data directory is unavailable.")
        })?;
        let database_path = state.database_path().ok_or_else(|| {
            AppError::database_unavailable("The V2 database path is unavailable.")
        })?;
        state.with_connection(|connection| {
            super::backup::get_overview(connection, &data_dir, &database_path)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_update_backup_settings(
    app: AppHandle,
    input: V2BackupSettingsInput,
) -> Result<V2BackupSettings, AppError> {
    run_blocking(app, move |state| {
        let data_dir = state.data_directory().ok_or_else(|| {
            AppError::database_unavailable("The V2 data directory is unavailable.")
        })?;
        state.with_connection(|connection| {
            super::backup::update_settings(connection, &data_dir, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_create_manual_backup(app: AppHandle) -> Result<V2BackupRecord, AppError> {
    run_blocking(app, |state| {
        let data_dir = state.data_directory().ok_or_else(|| {
            AppError::database_unavailable("The V2 data directory is unavailable.")
        })?;
        state.with_connection(|connection| {
            super::backup::create_manual_backup(connection, &data_dir, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_run_automatic_backup_if_due(
    app: AppHandle,
) -> Result<Option<V2BackupRecord>, AppError> {
    run_blocking(app, |state| {
        let data_dir = state.data_directory().ok_or_else(|| {
            AppError::database_unavailable("The V2 data directory is unavailable.")
        })?;
        state.with_connection(|connection| {
            super::backup::run_automatic_backup_if_due(connection, &data_dir, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_preview_restore_backup(path: String) -> Result<V2RestorePreview, AppError> {
    tauri::async_runtime::spawn_blocking(move || super::backup::preview_restore(&path))
        .await
        .map_err(|_| AppError::internal("The restore preview ended unexpectedly."))?
}

#[tauri::command]
pub async fn v2_apply_restore_backup(
    app: AppHandle,
    input: V2RestoreInput,
) -> Result<V2RestoreResult, AppError> {
    run_blocking(app, move |state| state.restore_backup(input, Utc::now())).await
}

#[tauri::command]
pub async fn v2_get_restore_recovery(app: AppHandle) -> Result<V2RestoreRecovery, AppError> {
    let data_dir = app
        .state::<V2State>()
        .data_directory()
        .ok_or_else(|| AppError::database_unavailable("The V2 data directory is unavailable."))?;
    tauri::async_runtime::spawn_blocking(move || super::backup::get_restore_recovery(&data_dir))
        .await
        .map_err(|_| AppError::internal("The restore recovery check ended unexpectedly."))?
}

#[tauri::command]
pub async fn v2_restore_recovery_safety(
    app: AppHandle,
    input: V2RestoreInput,
) -> Result<V2RestoreResult, AppError> {
    run_blocking(app, move |state| {
        state.restore_recovery_safety(input, Utc::now())
    })
    .await
}

#[tauri::command]
pub async fn v2_get_exam_syllabus(
    app: AppHandle,
    input: V2SyllabusExamInput,
) -> Result<V2ExamSyllabus, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::syllabus::get_exam_syllabus(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_update_syllabus_node_progress(
    app: AppHandle,
    input: V2SyllabusProgressInput,
) -> Result<V2ExamSyllabus, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::syllabus::update_node_progress(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_revision_dashboard(
    app: AppHandle,
    input: V2RevisionDashboardInput,
) -> Result<V2RevisionDashboard, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::revision::dashboard(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_update_revision_settings(
    app: AppHandle,
    input: V2RevisionSettingsInput,
) -> Result<V2RevisionSettings, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::revision::update_settings(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_schedule_revision_concept(
    app: AppHandle,
    input: V2RevisionScheduleInput,
) -> Result<V2RevisionSchedule, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::revision::schedule_concept(
                connection,
                input,
                Local::now().date_naive(),
                Utc::now(),
            )
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_reschedule_revision(
    app: AppHandle,
    input: V2RevisionRescheduleInput,
) -> Result<V2RevisionSchedule, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::revision::reschedule(connection, input, Local::now().date_naive(), Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_set_revision_status(
    app: AppHandle,
    input: V2RevisionStatusInput,
) -> Result<V2RevisionSchedule, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::revision::set_status(connection, input, Local::now().date_naive(), Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_complete_revision(
    app: AppHandle,
    input: V2CompleteRevisionInput,
) -> Result<V2RevisionDashboard, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::revision::complete_revision(
                connection,
                input,
                Local::now().date_naive(),
                Utc::now(),
            )
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_defer_revision(
    app: AppHandle,
    input: V2DeferRevisionInput,
) -> Result<V2RevisionDashboard, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::revision::defer_revision(
                connection,
                input,
                Local::now().date_naive(),
                Utc::now(),
            )
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_revision_daily_plan(
    app: AppHandle,
    input: V2RevisionPlanInput,
) -> Result<Option<V2RevisionDailyPlan>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::revision::get_daily_plan(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_generate_revision_daily_plan(
    app: AppHandle,
    input: V2RevisionPlanInput,
) -> Result<V2RevisionDailyPlan, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::revision::generate_daily_plan(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_rebalance_revision_daily_plan(
    app: AppHandle,
    input: V2RevisionPlanInput,
) -> Result<V2RevisionDailyPlan, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::revision::rebalance_daily_plan(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_move_revision_plan_item(
    app: AppHandle,
    input: V2MoveRevisionPlanItemInput,
) -> Result<Option<V2RevisionDailyPlan>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::revision::move_plan_item(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_link_revision_plan_block(
    app: AppHandle,
    input: V2LinkRevisionBlockInput,
) -> Result<Option<V2RevisionDailyPlan>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::revision::link_plan_block(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_analytics(
    app: AppHandle,
    input: V2AnalyticsInput,
) -> Result<V2Analytics, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::analytics::get_analytics(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_get_dashboard_sections(
    app: AppHandle,
    input: V2DashboardLayoutScope,
) -> Result<Vec<V2DashboardSection>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::dashboard::load_layout(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_save_dashboard_sections(
    app: AppHandle,
    input: V2DashboardLayoutInput,
) -> Result<Vec<V2DashboardSection>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::dashboard::save_layout(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_reset_dashboard_sections(
    app: AppHandle,
    input: V2DashboardLayoutScope,
) -> Result<Vec<V2DashboardSection>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::dashboard::reset_layout(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_list_quick_start_configs(
    app: AppHandle,
) -> Result<Vec<super::quick_start::V2QuickStartConfig>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::quick_start::list_quick_start_configs(connection))
    })
    .await
}

#[tauri::command]
pub async fn v2_save_quick_start_config(
    app: AppHandle,
    input: super::quick_start::V2QuickStartConfigInput,
) -> Result<Vec<super::quick_start::V2QuickStartConfig>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::quick_start::save_quick_start_config(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_delete_quick_start_config(
    app: AppHandle,
    input: super::quick_start::V2QuickStartConfigDeleteInput,
) -> Result<Vec<super::quick_start::V2QuickStartConfig>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::quick_start::delete_quick_start_config(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_global_search(
    app: AppHandle,
    input: super::search::V2GlobalSearchInput,
) -> Result<super::search::V2GlobalSearchResult, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::search::global_search(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_get_practice_options(
    app: AppHandle,
    input: V2PracticeExamInput,
) -> Result<V2PracticeOptions, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::practice::pyq::options(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_list_pyq_questions(
    app: AppHandle,
    filter: V2PyqFilter,
) -> Result<Vec<super::practice::pyq::V2PyqQuestionSummary>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::practice::pyq::list_questions(connection, filter))
    })
    .await
}

#[tauri::command]
pub async fn v2_get_pyq_question_detail(
    app: AppHandle,
    exam_id: i64,
    question_id: i64,
) -> Result<V2PyqQuestionDetail, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::pyq::detail(connection, exam_id, question_id)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_save_pyq_question(
    app: AppHandle,
    input: V2PyqQuestionInput,
) -> Result<V2PyqQuestionDetail, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::practice::pyq::save_question(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_archive_pyq_question(
    app: AppHandle,
    exam_id: i64,
    question_id: i64,
    archived: bool,
) -> Result<(), AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::pyq::archive_question(connection, exam_id, question_id, archived)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_record_pyq_attempt(
    app: AppHandle,
    input: V2PyqAttemptInput,
) -> Result<V2PyqAttempt, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::practice::pyq::record_attempt(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_list_pyq_attempts(
    app: AppHandle,
    exam_id: i64,
    question_id: Option<i64>,
) -> Result<Vec<V2PyqAttempt>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::pyq::list_attempts(connection, exam_id, question_id)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_preview_pyq_import(
    app: AppHandle,
    csv: String,
) -> Result<V2PracticeImportPreview, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::practice::pyq::preview_import(connection, &csv))
    })
    .await
}

#[tauri::command]
pub async fn v2_apply_pyq_import(
    app: AppHandle,
    csv: String,
) -> Result<V2PracticeImportResult, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::practice::pyq::apply_import(connection, &csv))
    })
    .await
}

#[tauri::command]
pub async fn v2_list_mock_tests(
    app: AppHandle,
    exam_id: i64,
    include_archived: bool,
) -> Result<Vec<V2MockTest>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::mock::list_mocks(connection, exam_id, include_archived)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_save_mock_test(
    app: AppHandle,
    input: V2MockTestInput,
) -> Result<V2MockTest, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::practice::mock::save_mock(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_set_mock_test_status(
    app: AppHandle,
    exam_id: i64,
    mock_id: i64,
    action: String,
) -> Result<V2MockTest, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::mock::set_mock_status(connection, exam_id, mock_id, &action)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_archive_mock_test(
    app: AppHandle,
    exam_id: i64,
    mock_id: i64,
    archived: bool,
) -> Result<(), AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::mock::archive_mock(connection, exam_id, mock_id, archived)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_practice_analytics(
    app: AppHandle,
    input: V2PracticeAnalyticsInput,
) -> Result<V2PracticeAnalytics, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::practice::analytics::analytics(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_list_question_mock_candidates(
    app: AppHandle,
    exam_id: i64,
    pattern_version_id: i64,
) -> Result<Vec<V2QuestionMockCandidate>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::question_mock::candidates(connection, exam_id, pattern_version_id)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_start_question_mock(
    app: AppHandle,
    input: V2StartQuestionMockInput,
) -> Result<V2QuestionMockSession, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::practice::question_mock::start(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_get_question_mock_session(
    app: AppHandle,
    exam_id: i64,
    mock_id: i64,
) -> Result<V2QuestionMockSession, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::question_mock::get_session(connection, exam_id, mock_id)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_save_question_mock_state(
    app: AppHandle,
    input: V2SaveQuestionMockInput,
) -> Result<V2QuestionMockSaveResult, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::question_mock::save_state(connection, input)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_submit_question_mock(
    app: AppHandle,
    input: V2SubmitQuestionMockInput,
) -> Result<V2QuestionMockSession, AppError> {
    run_blocking(app, move |state| {
        state
            .with_connection(|connection| super::practice::question_mock::submit(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_list_pyq_sources(
    app: AppHandle,
    filter: V2PyqSourceFilter,
) -> Result<Vec<V2PyqSource>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::list_sources(connection, filter)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_pyq_source_detail(
    app: AppHandle,
    source_id: i64,
) -> Result<V2SourceDetail, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::source_detail(connection, source_id)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_save_pyq_source(
    app: AppHandle,
    input: V2PyqSourceInput,
) -> Result<V2PyqSource, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::save_source(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_acquire_pyq_source(
    app: AppHandle,
    input: V2AcquireSourceInput,
) -> Result<V2PyqSourceAcquisition, AppError> {
    run_blocking(app, move |state| {
        let data_dir = state.data_directory().ok_or_else(|| {
            AppError::database_unavailable("The V2 data directory is unavailable.")
        })?;
        state.with_connection(|connection| {
            super::practice::sources::acquire_source(connection, &data_dir, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_set_pyq_source_status(
    app: AppHandle,
    input: V2SourceStatusInput,
) -> Result<V2PyqSource, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::set_source_status(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_preview_pyq_answer_key(
    app: AppHandle,
    text: String,
) -> Result<V2AnswerKeyPreview, AppError> {
    run_blocking(app, move |state| {
        let _ = state;
        Ok(super::practice::sources::preview_answer_key_text(&text))
    })
    .await
}

#[tauri::command]
pub async fn v2_list_pyq_answer_keys(
    app: AppHandle,
    exam_id: Option<i64>,
    year: Option<i64>,
) -> Result<Vec<V2PyqAnswerKey>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::list_answer_keys(connection, exam_id, year)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_pyq_answer_key(
    app: AppHandle,
    answer_key_id: i64,
) -> Result<V2PyqAnswerKey, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::get_answer_key(connection, answer_key_id)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_save_pyq_answer_key(
    app: AppHandle,
    input: V2PyqAnswerKeyInput,
) -> Result<V2PyqAnswerKey, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::save_answer_key(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_list_pyq_answer_key_conflicts(
    app: AppHandle,
    exam_id: Option<i64>,
) -> Result<Vec<V2AnswerKeyConflict>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::list_answer_key_conflicts(connection, exam_id)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_extract_pyq_source(
    app: AppHandle,
    input: V2ExtractionInput,
) -> Result<V2ExtractionResult, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::extract_source(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_reconcile_pyq_source(
    app: AppHandle,
    source_id: i64,
    answer_key_id: i64,
) -> Result<i64, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::reconcile_source_candidates(
                connection,
                source_id,
                answer_key_id,
                Utc::now(),
            )
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_list_pyq_extraction_candidates(
    app: AppHandle,
    filter: V2ExtractionCandidateFilter,
) -> Result<Vec<V2ExtractionCandidate>, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::list_candidates(connection, filter)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_pyq_extraction_candidate(
    app: AppHandle,
    candidate_id: i64,
) -> Result<V2ExtractionCandidate, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::get_candidate(connection, candidate_id)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_update_pyq_extraction_candidate(
    app: AppHandle,
    input: V2ExtractionCandidateUpdate,
) -> Result<V2ExtractionCandidate, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::update_candidate(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_verify_pyq_extraction_candidate(
    app: AppHandle,
    input: V2CandidateDecisionInput,
) -> Result<V2ExtractionCandidate, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::verify_candidate(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_reject_pyq_extraction_candidate(
    app: AppHandle,
    input: V2CandidateDecisionInput,
) -> Result<V2ExtractionCandidate, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::practice::sources::reject_candidate(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_pyq_source_counts(app: AppHandle) -> Result<V2SourceCounts, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::practice::sources::counts(connection))
    })
    .await
}

#[tauri::command]
pub async fn v2_list_flashcards(
    app: AppHandle,
    filter: super::flashcards::V2FlashcardFilter,
) -> Result<super::flashcards::V2FlashcardLibrary, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::flashcards::list_flashcards(connection, filter))
    })
    .await
}

#[tauri::command]
pub async fn v2_get_flashcard_detail(
    app: AppHandle,
    flashcard_id: i64,
) -> Result<super::flashcards::V2FlashcardDetail, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::flashcards::get_flashcard_detail(connection, flashcard_id)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_save_flashcard(
    app: AppHandle,
    input: super::flashcards::V2FlashcardInput,
) -> Result<super::flashcards::V2FlashcardDetail, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::flashcards::save_flashcard(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_set_flashcard_archived(
    app: AppHandle,
    input: super::flashcards::V2FlashcardArchiveInput,
) -> Result<super::flashcards::V2FlashcardDetail, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::flashcards::set_flashcard_archived(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_flashcard_review_queue(
    app: AppHandle,
    input: super::flashcards::V2FlashcardQueueInput,
) -> Result<super::flashcards::V2FlashcardQueue, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::flashcards::review_queue(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_review_flashcard(
    app: AppHandle,
    input: super::flashcards::V2FlashcardReviewInput,
) -> Result<super::flashcards::V2FlashcardReviewResult, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::flashcards::review_flashcard(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_flashcard_stats(
    app: AppHandle,
    input: super::flashcards::V2FlashcardStatsInput,
) -> Result<super::flashcards::V2FlashcardStats, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::flashcards::flashcard_stats(connection, input))
    })
    .await
}

#[tauri::command]
pub async fn v2_list_custom_formulas(
    app: AppHandle,
) -> Result<Vec<super::reference::V2CustomFormula>, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::reference::list_custom_formulas(connection))
    })
    .await
}

#[tauri::command]
pub async fn v2_save_custom_formula(
    app: AppHandle,
    input: super::reference::V2CustomFormulaInput,
) -> Result<super::reference::V2CustomFormula, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::reference::save_custom_formula(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_delete_custom_formula(app: AppHandle, id: i64) -> Result<(), AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::reference::delete_custom_formula(connection, id))
    })
    .await
}

#[tauri::command]
pub async fn v2_list_custom_units(
    app: AppHandle,
) -> Result<Vec<super::reference::V2CustomUnit>, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::reference::list_custom_units(connection))
    })
    .await
}

#[tauri::command]
pub async fn v2_save_custom_unit(
    app: AppHandle,
    input: super::reference::V2CustomUnitInput,
) -> Result<super::reference::V2CustomUnit, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::reference::save_custom_unit(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_delete_custom_unit(app: AppHandle, id: i64) -> Result<(), AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::reference::delete_custom_unit(connection, id))
    })
    .await
}

#[tauri::command]
pub async fn v2_list_custom_constants(
    app: AppHandle,
) -> Result<Vec<super::reference::V2CustomConstant>, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::reference::list_custom_constants(connection))
    })
    .await
}

#[tauri::command]
pub async fn v2_save_custom_constant(
    app: AppHandle,
    input: super::reference::V2CustomConstantInput,
) -> Result<super::reference::V2CustomConstant, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::reference::save_custom_constant(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_delete_custom_constant(app: AppHandle, id: i64) -> Result<(), AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::reference::delete_custom_constant(connection, id))
    })
    .await
}

#[tauri::command]
pub async fn v2_get_analytics_charts(
    app: AppHandle,
    input: super::analytics_charts::V2AnalyticsChartsInput,
) -> Result<super::analytics_charts::V2AnalyticsCharts, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::analytics_charts::get_analytics_charts(connection, input)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_get_builtin_constants(
    app: AppHandle,
) -> Result<super::builtin_constants::V2BuiltinConstantCatalog, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| {
            super::builtin_constants::list_builtin_constants(connection)
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_list_sticky_notes(
    app: AppHandle,
) -> Result<Vec<super::sticky_notes::V2StickyNote>, AppError> {
    run_blocking(app, |state| {
        state.with_connection(|connection| super::sticky_notes::list_sticky_notes(connection))
    })
    .await
}

#[tauri::command]
pub async fn v2_save_sticky_note(
    app: AppHandle,
    input: super::sticky_notes::V2StickyNoteInput,
) -> Result<super::sticky_notes::V2StickyNote, AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| {
            super::sticky_notes::save_sticky_note(connection, input, Utc::now())
        })
    })
    .await
}

#[tauri::command]
pub async fn v2_delete_sticky_note(app: AppHandle, id: i64) -> Result<(), AppError> {
    run_blocking(app, move |state| {
        state.with_connection(|connection| super::sticky_notes::delete_sticky_note(connection, id))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> (tempfile::TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    #[test]
    fn status_command_reports_initialized_database() {
        let (_root, state) = state();
        let status = database_status(&state);
        assert!(status.initialized);
        assert_eq!(status.exam_count, 4);
    }

    #[test]
    fn initialize_command_is_idempotent() {
        let (_root, state) = state();
        let status = initialize_database(&state).expect("initialize command");
        assert!(status.initialized);
        assert_eq!(status.exam_count, 4);
    }

    #[test]
    fn list_exams_command_returns_four_exams() {
        let (_root, state) = state();
        let exams = exam_list(&state).expect("exam command");
        assert_eq!(exams.len(), 4);
        assert!(exams.iter().all(|exam| exam.is_active));
    }

    #[test]
    fn integrity_command_reports_clean_database() {
        let (_root, state) = state();
        let report = database_integrity(&state, true).expect("integrity command");
        assert!(report.ok);
        assert_eq!(report.foreign_key_violations, 0);
    }

    #[test]
    fn timer_and_session_commands_have_a_usable_idle_state() {
        let (_root, state) = state();
        let timer = state
            .with_connection(|connection| super::super::study::get_timer(connection, Utc::now()))
            .expect("timer command state");
        assert_eq!(timer.state.status, super::super::study::TimerStatus::Idle);
        assert_eq!(timer.settings.focus_minutes, 25);

        let sessions = state
            .with_connection(|connection| super::super::study::list_sessions(connection))
            .expect("session command state");
        assert!(sessions.is_empty());

        let totals = state
            .with_connection(|connection| super::super::study::study_totals(connection))
            .expect("totals command state");
        assert_eq!(totals.total_seconds, 0);
        assert_eq!(totals.session_count, 0);
    }

    #[test]
    fn v2_command_layer_never_references_v1_state() {
        let source = include_str!("commands.rs");
        let v1_state = concat!("db", "::", "AppState");
        let v1_generic = concat!("state", "::<", "db", "::");
        let v1_crate = concat!("crate", "::", "db");
        assert!(!source.contains(v1_state));
        assert!(!source.contains(v1_generic));
        assert!(!source.contains(v1_crate));
        assert!(source.contains("app.state::<V2State>"));
    }

    #[test]
    fn concept_commands_seed_the_library_and_replay_manual_mastery() {
        let (_root, state) = state();
        let search = state
            .with_connection(|connection| {
                super::super::concepts::search_concepts(
                    connection,
                    super::super::concepts::V2ConceptSearchFilter::default(),
                )
            })
            .expect("concept search command state");
        assert!(search.total >= 60);
        assert_eq!(search.exam_coverage.len(), 4);
        let concept = search
            .concepts
            .iter()
            .find(|concept| concept.slug == "kinematics")
            .expect("curated concept");

        let update = state
            .with_connection(|connection| {
                super::super::mastery::update_manual_mastery(
                    connection,
                    super::super::mastery::V2UpdateConceptMasteryInput {
                        concept_id: concept.id,
                        target_score: 70.0,
                        local_date: "2026-11-04".to_owned(),
                        notes: String::new(),
                    },
                    Utc::now(),
                )
            })
            .expect("manual mastery command state");
        assert!((update.mastery.mastery_score - 70.0).abs() < 1e-9);

        let detail = state
            .with_connection(|connection| {
                super::super::concepts::get_concept_detail(
                    connection,
                    super::super::concepts::V2ConceptDetailInput {
                        slug: "kinematics".to_owned(),
                        evidence_limit: None,
                        history_limit: None,
                    },
                )
            })
            .expect("concept detail command state");
        assert_eq!(detail.evidence.len(), 1);
        assert_eq!(detail.mastery_history.len(), 1);
        assert_eq!(detail.concept.mastery.expect("mastery").evidence_count, 1);
    }
}
