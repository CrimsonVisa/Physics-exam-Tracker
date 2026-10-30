use serde::Serialize;

/// Serializable, command-safe error returned to the frontend.
///
/// Internal SQLite details are logged in Rust and deliberately not exposed to
/// the webview. Validation errors identify the responsible field so the form
/// can attach the message to the correct control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<&'static str>,
}

impl AppError {
    pub fn validation(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            code: "validation_error",
            message: message.into(),
            field: Some(field),
        }
    }

    pub fn database(context: &str, source: impl std::fmt::Display) -> Self {
        eprintln!("{context}: {source}");
        Self {
            code: "database_error",
            message: format!("{context}. Please retry; if the problem continues, restart the app."),
            field: None,
        }
    }

    pub fn migration(context: &str, source: impl std::fmt::Display) -> Self {
        eprintln!("{context}: {source}");
        Self {
            code: "migration_error",
            message: format!(
                "{context}. This database may have been created by a newer app version."
            ),
            field: None,
        }
    }

    pub fn database_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "database_unavailable",
            message: message.into(),
            field: None,
        }
    }

    pub fn settings_not_found() -> Self {
        Self {
            code: "settings_not_found",
            message: "The settings record is missing from the local database. Restart the app to recreate it."
                .to_owned(),
            field: None,
        }
    }

    pub fn topic_not_found() -> Self {
        Self {
            code: "topic_not_found",
            message: "This syllabus topic is not in the active catalog. Refresh the syllabus and try again."
                .to_owned(),
            field: Some("topicId"),
        }
    }

    pub fn session_not_found() -> Self {
        Self {
            code: "session_not_found",
            message: "This study session is no longer available. Refresh the session history and try again."
                .to_owned(),
            field: None,
        }
    }

    pub fn task_not_found() -> Self {
        Self {
            code: "task_not_found",
            message:
                "This checklist task is no longer available. Refresh the checklist and try again."
                    .to_owned(),
            field: None,
        }
    }

    pub fn task_completed() -> Self {
        Self {
            code: "task_completed",
            message: "Reopen this task before rescheduling it.".to_owned(),
            field: None,
        }
    }

    pub fn block_not_found() -> Self {
        Self {
            code: "block_not_found",
            message: "This planned study block is no longer available. Refresh the calendar and try again."
                .to_owned(),
            field: None,
        }
    }

    pub fn block_not_completed() -> Self {
        Self {
            code: "block_not_completed",
            message: "Mark the planned block completed before linking a study session.".to_owned(),
            field: None,
        }
    }

    pub fn reschedule_confirmation_required() -> Self {
        Self {
            code: "reschedule_confirmation_required",
            message: "Preview the exam-date change before saving it because exam-anchored tasks need confirmation."
                .to_owned(),
            field: None,
        }
    }

    pub fn reschedule_stale() -> Self {
        Self {
            code: "reschedule_stale",
            message: "The exam date or an anchored task changed after the preview. Review the new preview and try again."
                .to_owned(),
            field: None,
        }
    }

    pub fn pyq_not_found() -> Self {
        Self {
            code: "pyq_not_found",
            message: "This PYQ attempt is no longer available. Refresh the PYQ list and try again."
                .to_owned(),
            field: None,
        }
    }

    pub fn mock_test_not_found() -> Self {
        Self {
            code: "mock_test_not_found",
            message: "This mock test is no longer available. Refresh the mock-test history and try again."
                .to_owned(),
            field: None,
        }
    }

    pub fn file_operation(context: &str, source: impl std::fmt::Display) -> Self {
        eprintln!("{context}: {source}");
        Self {
            code: "file_operation_error",
            message: format!(
                "{context}. Check that your Downloads folder is available, then try again."
            ),
            field: None,
        }
    }

    pub fn timer_active() -> Self {
        Self {
            code: "timer_active",
            message:
                "A study timer is already active. Finish or discard it before starting another."
                    .to_owned(),
            field: None,
        }
    }

    pub fn timer_not_running() -> Self {
        Self {
            code: "timer_not_running",
            message:
                "There is no running or paused timer to update. Refresh the timer and try again."
                    .to_owned(),
            field: None,
        }
    }

    pub fn timer_not_paused() -> Self {
        Self {
            code: "timer_not_paused",
            message: "The timer is not paused, so it cannot be resumed.".to_owned(),
            field: None,
        }
    }

    pub fn timer_empty() -> Self {
        Self {
            code: "timer_empty",
            message: "This timer has not accumulated any focus time yet. Discard it instead."
                .to_owned(),
            field: None,
        }
    }

    pub fn timer_unavailable() -> Self {
        Self {
            code: "timer_unavailable",
            message: "The saved timer state could not be read. Restart the app to recreate it."
                .to_owned(),
            field: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal_error",
            message: message.into(),
            field: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_errors_identify_their_field() {
        let error = AppError::validation("examDate", "Enter a valid date.");

        assert_eq!(error.code, "validation_error");
        assert_eq!(error.field, Some("examDate"));
    }

    #[test]
    fn database_errors_do_not_expose_internal_details() {
        let error = AppError::database("Settings could not be loaded", "disk I/O error");

        assert_eq!(error.code, "database_error");
        assert!(!error.message.contains("disk I/O error"));
        assert!(error.message.contains("retry"));
    }
}
