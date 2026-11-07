pub mod analytics;
pub mod extraction;
pub mod mock;
pub mod patterns;
pub mod pyq;
pub mod question_mock;
pub mod scoring;
pub mod sources;

pub use analytics::{V2PracticeAnalytics, V2PracticeAnalyticsInput};
pub use mock::{V2MockTest, V2MockTestInput};
pub use pyq::{
    V2PracticeExamInput, V2PracticeImportPreview, V2PracticeImportResult, V2PracticeOptions,
    V2PyqAttempt, V2PyqAttemptInput, V2PyqFilter, V2PyqQuestionDetail, V2PyqQuestionInput,
};

pub fn prepare(connection: &mut rusqlite::Connection) -> Result<(), crate::error::AppError> {
    patterns::prepare(connection)?;
    sources::prepare(connection)?;
    question_mock::prepare(connection)?;
    question_mock::expire_due(connection, chrono::Utc::now())
}

#[cfg(test)]
mod tests {
    use super::pyq::V2PyqQuestionInput;
    use super::*;
    use crate::v2::V2State;

    fn state() -> (tempfile::TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    fn verified_pattern(options: &V2PracticeOptions, code: &str) -> (i64, i64, i64, i64) {
        let version = options
            .patterns
            .iter()
            .find(|pattern| pattern.verification_status == "verified")
            .expect("verified pattern");
        let section = version
            .sections
            .iter()
            .find(|section| section.code == code)
            .expect("pattern section");
        let question_type = options
            .question_types
            .iter()
            .find(|kind| kind.code == "MCQ" || kind.code == "NAT")
            .expect("question type");
        let topic = options
            .topics
            .iter()
            .find(|topic| topic.node_kind == "topic");
        (
            version.id,
            section.section_id,
            question_type.id,
            topic.map_or(0, |topic| topic.id),
        )
    }

    #[test]
    fn verified_and_pending_patterns_are_seeded_for_all_four_exams() {
        let (_root, state) = state();
        for exam_id in 1..=4 {
            let options = state
                .with_connection(|connection| {
                    super::pyq::options(connection, V2PracticeExamInput { exam_id })
                })
                .expect("practice options");
            assert!(
                options
                    .patterns
                    .iter()
                    .any(|pattern| pattern.verification_status == "verified"),
                "exam {exam_id} has no verified pattern"
            );
            assert!(!options.sections.is_empty());
            assert!(!options.question_types.is_empty());
        }
        let jest = state
            .with_connection(|connection| {
                super::pyq::options(connection, V2PracticeExamInput { exam_id: 4 })
            })
            .expect("JEST options");
        assert!(
            jest.patterns
                .iter()
                .any(|pattern| pattern.version_label.starts_with("JEST 2027")
                    && pattern.verification_status == "pending")
        );
        let tifr = state
            .with_connection(|connection| {
                super::pyq::options(connection, V2PracticeExamInput { exam_id: 3 })
            })
            .expect("TIFR options");
        assert!(
            tifr.patterns
                .iter()
                .any(|pattern| pattern.version_label == "GS 2024"
                    && pattern.verification_status == "verified")
        );
        assert!(
            tifr.patterns
                .iter()
                .any(|pattern| pattern.version_label.starts_with("GS 2027"))
        );
    }

    #[test]
    fn pyq_attempt_is_backend_scored_and_creates_one_evidence_row() {
        let (_root, state) = state();
        let (pattern_id, section_id, type_id, topic_id) = state
            .with_connection(|connection| {
                let options = super::pyq::options(connection, V2PracticeExamInput { exam_id: 1 })?;
                Ok(verified_pattern(&options, "csir-705-part-a"))
            })
            .expect("pattern ids");
        let concept_id = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT id FROM concepts WHERE slug = 'classical-mechanics'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| crate::error::AppError::database("concept", error))
            })
            .expect("concept");
        let question = state
            .with_connection(|connection| {
                super::pyq::save_question(
                    connection,
                    V2PyqQuestionInput {
                        id: None,
                        exam_id: 1,
                        year: 2026,
                        session_label: "Synthetic test-only fixture".to_owned(),
                        section_id,
                        question_type_id: type_id,
                        question_number: "T1".to_owned(),
                        statement: "Synthetic test question; not production content.".to_owned(),
                        correct_answer: "B".to_owned(),
                        correct_range: String::new(),
                        max_marks: 2.0,
                        negative_marks: -0.5,
                        unanswered_marks: 0.0,
                        difficulty: 3,
                        source_title: String::new(),
                        source_page: String::new(),
                        source_url: None,
                        answer_key_source: "Synthetic test key".to_owned(),
                        answer_key_version: "test-1".to_owned(),
                        answer_key_verified: true,
                        verification_status: "verified".to_owned(),
                        numeric_tolerance: 0.0,
                        options: Vec::new(),
                        pattern_version_id: Some(pattern_id),
                        syllabus_node_id: Some(topic_id),
                        topic_ids: Vec::new(),
                        concept_ids: vec![concept_id],
                        resource_id: None,
                        source_id: None,
                        answer_key_id: None,
                        dedupe_key: String::new(),
                        official_explanation: String::new(),
                        third_party_solution: String::new(),
                        user_note: String::new(),
                    },
                )
            })
            .expect("save question");
        assert_eq!(question.question.verification_status, "verified");
        let attempt = state
            .with_connection(|connection| {
                super::pyq::record_attempt(
                    connection,
                    super::pyq::V2PyqAttemptInput {
                        exam_id: 1,
                        question_id: question.question.id,
                        attempt_date: "2026-11-05".to_owned(),
                        selected_answer: "B".to_owned(),
                        time_spent_seconds: 42,
                        confidence: Some(3),
                        mistakes: String::new(),
                        review_status: "unreviewed".to_owned(),
                        notes: String::new(),
                    },
                )
            })
            .expect("attempt");
        assert_eq!(attempt.status, "correct");
        assert_eq!(attempt.marks_obtained, 2.0);
        let evidence: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM concept_evidence", [], |row| {
                        row.get(0)
                    })
                    .map_err(|error| crate::error::AppError::database("evidence", error))
            })
            .expect("evidence count");
        assert_eq!(evidence, 1);
        let analytics = state
            .with_connection(|connection| {
                super::analytics::analytics(
                    connection,
                    super::analytics::V2PracticeAnalyticsInput {
                        exam_id: 1,
                        from_date: None,
                        to_date: None,
                        year: None,
                        section_id: None,
                        syllabus_node_id: None,
                    },
                )
            })
            .expect("analytics");
        assert_eq!(analytics.pyq_attempts, 1);
        assert_eq!(analytics.correct, 1);
        assert_eq!(analytics.accuracy_percentage, Some(100.0));
        assert_eq!(analytics.verified_questions, 1);
    }

    #[test]
    fn mock_completion_is_immutable_and_recompletion_is_idempotent() {
        let (_root, state) = state();
        let (pattern_id, section_id, _, _) = state
            .with_connection(|connection| {
                let options = super::pyq::options(connection, V2PracticeExamInput { exam_id: 1 })?;
                Ok(verified_pattern(&options, "csir-705-part-a"))
            })
            .expect("pattern ids");
        let pattern_section_id = state
            .with_connection(|connection| {
                let options = super::pyq::options(connection, V2PracticeExamInput { exam_id: 1 })?;
                let version = options
                    .patterns
                    .iter()
                    .find(|pattern| pattern.id == pattern_id)
                    .expect("version");
                Ok(version
                    .sections
                    .iter()
                    .find(|section| section.section_id == section_id)
                    .expect("section")
                    .id)
            })
            .expect("pattern section");
        let mock = state
            .with_connection(|connection| {
                super::mock::save_mock(
                    connection,
                    V2MockTestInput {
                        id: None,
                        exam_id: 1,
                        pattern_version_id: pattern_id,
                        test_name: "Synthetic test-only mock".to_owned(),
                        test_date: "2026-11-05".to_owned(),
                        duration_minutes: 180,
                        test_mode: "section".to_owned(),
                        focus_node_id: None,
                        notes: String::new(),
                        review_observations: String::new(),
                        sections: vec![super::mock::V2MockSectionInput {
                            pattern_section_id,
                            question_count: 15,
                            correct: 10,
                            incorrect: 3,
                            unanswered: 2,
                            max_marks: None,
                            marks_obtained: None,
                            time_spent_minutes: Some(60),
                            notes: String::new(),
                        }],
                    },
                )
            })
            .expect("save mock");
        assert_eq!(mock.status, "planned");
        let completed = state
            .with_connection(|connection| {
                super::mock::set_mock_status(connection, 1, mock.id, "complete")
            })
            .expect("complete mock");
        assert_eq!(completed.status, "completed");
        assert_eq!(completed.score_history.len(), 1);
        assert_eq!(completed.marks_obtained, Some(18.5));
        let reopened = state
            .with_connection(|connection| {
                super::mock::set_mock_status(connection, 1, mock.id, "reopen")
            })
            .expect("reopen");
        assert_eq!(reopened.status, "in_progress");
        let reclosed = state
            .with_connection(|connection| {
                super::mock::set_mock_status(connection, 1, mock.id, "complete")
            })
            .expect("recomplete");
        assert_eq!(reclosed.score_history.len(), 1);
        let sessions: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM study_sessions", [], |row| row.get(0))
                    .map_err(|error| crate::error::AppError::database("sessions", error))
            })
            .expect("sessions");
        assert_eq!(sessions, 0);
    }

    #[test]
    fn full_length_mocks_require_the_whole_verified_pattern() {
        let (_root, state) = state();
        let (pattern_id, pattern_sections) = state
            .with_connection(|connection| {
                let options = super::pyq::options(connection, V2PracticeExamInput { exam_id: 1 })?;
                let version = options
                    .patterns
                    .iter()
                    .find(|pattern| pattern.verification_status == "verified")
                    .expect("verified pattern");
                Ok((
                    version.id,
                    version
                        .sections
                        .iter()
                        .map(|section| (section.id, section.question_count))
                        .collect::<Vec<_>>(),
                ))
            })
            .expect("pattern sections");
        let partial_error = state
            .with_connection(|connection| {
                super::mock::save_mock(
                    connection,
                    V2MockTestInput {
                        id: None,
                        exam_id: 1,
                        pattern_version_id: pattern_id,
                        test_name: "Synthetic partial full-length".to_owned(),
                        test_date: "2026-11-05".to_owned(),
                        duration_minutes: 180,
                        test_mode: "full_length".to_owned(),
                        focus_node_id: None,
                        notes: String::new(),
                        review_observations: String::new(),
                        sections: vec![super::mock::V2MockSectionInput {
                            pattern_section_id: pattern_sections[0].0,
                            question_count: pattern_sections[0].1,
                            correct: 0,
                            incorrect: 0,
                            unanswered: pattern_sections[0].1,
                            max_marks: None,
                            marks_obtained: None,
                            time_spent_minutes: None,
                            notes: String::new(),
                        }],
                    },
                )
            })
            .expect_err("partial full length must fail");
        assert_eq!(partial_error.code, "validation_error");
        let full = state
            .with_connection(|connection| {
                super::mock::save_mock(
                    connection,
                    V2MockTestInput {
                        id: None,
                        exam_id: 1,
                        pattern_version_id: pattern_id,
                        test_name: "Synthetic full-length".to_owned(),
                        test_date: "2026-11-05".to_owned(),
                        duration_minutes: 180,
                        test_mode: "full_length".to_owned(),
                        focus_node_id: None,
                        notes: String::new(),
                        review_observations: String::new(),
                        sections: pattern_sections
                            .iter()
                            .map(|(id, count)| super::mock::V2MockSectionInput {
                                pattern_section_id: *id,
                                question_count: *count,
                                correct: 0,
                                incorrect: 0,
                                unanswered: *count,
                                max_marks: None,
                                marks_obtained: None,
                                time_spent_minutes: None,
                                notes: String::new(),
                            })
                            .collect(),
                    },
                )
            })
            .expect("full length");
        assert_eq!(full.max_marks, 200.0);
        assert_eq!(full.available_questions, 75);
    }

    #[test]
    fn inactive_exam_is_read_only_and_history_remains_queryable() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE exam_profiles SET is_active = 0 WHERE exam_id = 1",
                        [],
                    )
                    .map_err(|error| crate::error::AppError::database("deactivate", error))
            })
            .expect("deactivate");
        let options = state
            .with_connection(|connection| {
                super::pyq::options(connection, V2PracticeExamInput { exam_id: 1 })
            })
            .expect("options");
        assert!(!options.exam.is_active);
        let error = state
            .with_connection(|connection| {
                super::mock::save_mock(
                    connection,
                    V2MockTestInput {
                        id: None,
                        exam_id: 1,
                        pattern_version_id: 1,
                        test_name: "Blocked inactive exam".to_owned(),
                        test_date: "2026-11-05".to_owned(),
                        duration_minutes: 180,
                        test_mode: "full_length".to_owned(),
                        focus_node_id: None,
                        notes: String::new(),
                        review_observations: String::new(),
                        sections: Vec::new(),
                    },
                )
            })
            .expect_err("inactive write should fail");
        assert_eq!(error.code, "forbidden");
        let attempts = state
            .with_connection(|connection| super::pyq::list_attempts(connection, 1, None))
            .expect("history remains readable");
        assert!(attempts.is_empty());
    }

    #[test]
    fn manual_duplicate_and_csv_import_validation_are_explicit() {
        let (_root, state) = state();
        let (pattern_id, section_id, type_id, topic_id) = state
            .with_connection(|connection| {
                let options = super::pyq::options(connection, V2PracticeExamInput { exam_id: 1 })?;
                Ok(verified_pattern(&options, "csir-705-part-a"))
            })
            .expect("pattern ids");
        let concept_id = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT id FROM concepts WHERE slug = 'classical-mechanics'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| crate::error::AppError::database("concept", error))
            })
            .expect("concept");
        let input = V2PyqQuestionInput {
            id: None,
            exam_id: 1,
            year: 2099,
            session_label: "Duplicate test".to_owned(),
            section_id,
            question_type_id: type_id,
            question_number: "D1".to_owned(),
            statement: "Synthetic duplicate test fixture.".to_owned(),
            correct_answer: "B".to_owned(),
            correct_range: String::new(),
            max_marks: 2.0,
            negative_marks: -0.5,
            unanswered_marks: 0.0,
            difficulty: 3,
            source_title: "Synthetic duplicate source".to_owned(),
            source_page: "1".to_owned(),
            source_url: None,
            answer_key_source: "Synthetic key".to_owned(),
            answer_key_version: "duplicate-1".to_owned(),
            answer_key_verified: true,
            verification_status: "verified".to_owned(),
            numeric_tolerance: 0.0,
            options: Vec::new(),
            pattern_version_id: Some(pattern_id),
            syllabus_node_id: Some(topic_id),
            topic_ids: vec![topic_id],
            concept_ids: vec![concept_id],
            resource_id: None,
            source_id: None,
            answer_key_id: None,
            dedupe_key: String::new(),
            official_explanation: String::new(),
            third_party_solution: String::new(),
            user_note: String::new(),
        };
        state
            .with_connection(|connection| super::pyq::save_question(connection, input.clone()))
            .expect("first manual question");
        let duplicate = state
            .with_connection(|connection| super::pyq::save_question(connection, input.clone()))
            .expect_err("manual duplicate must fail");
        assert_eq!(duplicate.code, "validation_error");
        assert!(duplicate.message.contains("already exists"));

        let header = "exam_code,year,session_label,section_code,question_type_code,question_number,statement,correct_answer,correct_range,max_marks,negative_marks,unanswered_marks,difficulty,source_title,source_page,source_url,answer_key_source,answer_key_version,answer_key_verified,verification_status,numeric_tolerance,syllabus_node_code,concept_slugs,dedupe_key,official_explanation,third_party_solution,user_note";
        let valid = "csir-net-physics,2098,CSV test,csir-705-part-a,MCQ,C1,Synthetic CSV fixture,B,,2,-0.5,0,3,Synthetic source,1,,Synthetic key,csv-1,true,verified,0,,classical-mechanics,,,,";
        let duplicate_row = "csir-net-physics,2098,CSV test,csir-705-part-a,MCQ,C1,Duplicate CSV fixture,B,,2,-0.5,0,3,Synthetic source,1,,Synthetic key,csv-2,true,verified,0,,classical-mechanics,,,,";
        let invalid = "bogus-exam,2098,CSV test,csir-705-part-a,MCQ,C2,Invalid CSV fixture,B,,2,-0.5,0,3,Synthetic source,1,,Synthetic key,csv-3,true,verified,0,,classical-mechanics,,,,";
        let csv = format!("{header}\n{valid}\n{duplicate_row}\n{invalid}\n");
        let preview = state
            .with_connection(|connection| super::pyq::preview_import(connection, &csv))
            .expect("CSV preview");
        assert_eq!(preview.preview.valid_rows, 1);
        assert_eq!(preview.preview.duplicate_rows, 1);
        assert_eq!(preview.preview.invalid_rows, 1);
        let imported = state
            .with_connection(|connection| super::pyq::apply_import(connection, &csv))
            .expect("CSV apply");
        assert_eq!(imported.result.imported, 1);
        assert_eq!(imported.result.skipped, 1);
        assert_eq!(imported.result.rejected, 1);
        let total: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM pyq_questions", [], |row| row.get(0))
                    .map_err(|error| crate::error::AppError::database("question count", error))
            })
            .expect("question count");
        assert_eq!(total, 2);
    }

    #[test]
    fn csv_v2_imports_answer_choices_without_breaking_v1_contract() {
        let (_root, state) = state();
        let headers = "exam_code,year,session_label,section_code,question_type_code,question_number,statement,correct_answer,correct_range,max_marks,negative_marks,unanswered_marks,difficulty,source_title,source_page,source_url,answer_key_source,answer_key_version,answer_key_verified,verification_status,numeric_tolerance,syllabus_node_code,concept_slugs,dedupe_key,official_explanation,third_party_solution,user_note,options_json,pattern_version_id,source_id,answer_key_id";
        let options = r#"[{""key"":""A"",""text"":""Alpha""},{""key"":""B"",""text"":""Beta""},{""key"":""C"",""text"":""Gamma""},{""key"":""D"",""text"":""Delta""}]"#;
        let row = format!(
            "gate-physics,2022,5,gate-ph,MCQ,1,\"Synthetic CSV v2 question with four choices.\",B,,1,-0.3333333333333333,0,3,\"GATE 2022 PH paper\",\"p. 1\",https://example.test/ph-2022.pdf,\"GATE 2022 final key\",final-1,true,verified,0,,,csv-v2-1,,,,\"{options}\",2,,"
        );
        let csv = format!("{headers}\n{row}\n");
        let preview = state
            .with_connection(|connection| super::pyq::preview_import(connection, &csv))
            .expect("v2 preview");
        assert_eq!(preview.preview.valid_rows, 1);
        assert!(
            preview
                .headers
                .iter()
                .any(|header| header == "options_json")
        );
        let imported = state
            .with_connection(|connection| super::pyq::apply_import(connection, &csv))
            .expect("v2 apply");
        assert_eq!(imported.result.imported, 1);
        let question = state
            .with_connection(|connection| {
                super::pyq::list_questions(
                    connection,
                    super::pyq::V2PyqFilter {
                        exam_id: 2,
                        question_id: None,
                        year: Some(2022),
                        section_id: None,
                        question_type_id: None,
                        syllabus_node_id: None,
                        verification_status: Some("verified".to_owned()),
                        query: None,
                        include_archived: Some(false),
                        limit: Some(10),
                        offset: Some(0),
                    },
                )
            })
            .expect("v2 question");
        assert_eq!(question.len(), 1);
        assert_eq!(question[0].options.len(), 4);
        assert_eq!(question[0].options[1].key, "B");
        assert_eq!(question[0].pattern_version_id, Some(2));
        let mock_candidates = state
            .with_connection(|connection| super::question_mock::candidates(connection, 2, 2))
            .expect("mock candidates");
        assert_eq!(mock_candidates.len(), 1);

        let invalid = format!(
            "{headers}\ngate-physics,2022,5,gate-ph,MCQ,2,\"Invalid choices row should roll back.\",B,,1,-0.3333333333333333,0,3,,,,,,,,true,verified,0,,,csv-v2-invalid,,,,not-json,2,,,\n"
        );
        let preview = state
            .with_connection(|connection| super::pyq::preview_import(connection, &invalid))
            .expect("invalid preview");
        assert_eq!(preview.preview.invalid_rows, 1);
        let rejected = state
            .with_connection(|connection| super::pyq::apply_import(connection, &invalid))
            .expect("invalid apply");
        assert_eq!(rejected.result.rejected, 1);
        let count: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM pyq_questions", [], |row| row.get(0))
                    .map_err(|error| crate::error::AppError::database("question count", error))
            })
            .expect("question count");
        assert_eq!(count, 1);
    }

    #[test]
    fn answer_key_changes_do_not_rewrite_attempt_history() {
        let (_root, state) = state();
        let (pattern_id, section_id, type_id, topic_id) = state
            .with_connection(|connection| {
                let options = super::pyq::options(connection, V2PracticeExamInput { exam_id: 1 })?;
                Ok(verified_pattern(&options, "csir-705-part-a"))
            })
            .expect("pattern ids");
        let concept_id = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT id FROM concepts WHERE slug = 'classical-mechanics'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| crate::error::AppError::database("concept", error))
            })
            .expect("concept");
        let input = V2PyqQuestionInput {
            id: None,
            exam_id: 1,
            year: 2097,
            session_label: "Key history test".to_owned(),
            section_id,
            question_type_id: type_id,
            question_number: "K1".to_owned(),
            statement: "Synthetic key-history fixture.".to_owned(),
            correct_answer: "B".to_owned(),
            correct_range: String::new(),
            max_marks: 2.0,
            negative_marks: -0.5,
            unanswered_marks: 0.0,
            difficulty: 3,
            source_title: "Synthetic source".to_owned(),
            source_page: "1".to_owned(),
            source_url: None,
            answer_key_source: "Synthetic key".to_owned(),
            answer_key_version: "key-1".to_owned(),
            answer_key_verified: true,
            verification_status: "verified".to_owned(),
            numeric_tolerance: 0.0,
            options: Vec::new(),
            pattern_version_id: Some(pattern_id),
            syllabus_node_id: Some(topic_id),
            topic_ids: vec![topic_id],
            concept_ids: vec![concept_id],
            resource_id: None,
            source_id: None,
            answer_key_id: None,
            dedupe_key: String::new(),
            official_explanation: String::new(),
            third_party_solution: String::new(),
            user_note: String::new(),
        };
        let saved = state
            .with_connection(|connection| super::pyq::save_question(connection, input.clone()))
            .expect("save question");
        let first = state
            .with_connection(|connection| {
                super::pyq::record_attempt(
                    connection,
                    super::pyq::V2PyqAttemptInput {
                        exam_id: 1,
                        question_id: saved.question.id,
                        attempt_date: "2026-11-05".to_owned(),
                        selected_answer: "B".to_owned(),
                        time_spent_seconds: 10,
                        confidence: None,
                        mistakes: String::new(),
                        review_status: "unreviewed".to_owned(),
                        notes: String::new(),
                    },
                )
            })
            .expect("first attempt");
        let mut update = input;
        update.id = Some(saved.question.id);
        update.correct_answer = "A".to_owned();
        update.answer_key_version = "key-2".to_owned();
        state
            .with_connection(|connection| super::pyq::save_question(connection, update))
            .expect("change answer key");
        let detail = state
            .with_connection(|connection| super::pyq::detail(connection, 1, saved.question.id))
            .expect("question detail");
        let preserved = detail
            .attempts
            .iter()
            .find(|attempt| attempt.id == first.id)
            .expect("historical attempt");
        assert_eq!(preserved.correct_answer_at_attempt, "B");
        assert_eq!(preserved.answer_key_version, "key-1");
        assert_eq!(preserved.marks_obtained, 2.0);
        let second = state
            .with_connection(|connection| {
                super::pyq::record_attempt(
                    connection,
                    super::pyq::V2PyqAttemptInput {
                        exam_id: 1,
                        question_id: saved.question.id,
                        attempt_date: "2026-11-05".to_owned(),
                        selected_answer: "B".to_owned(),
                        time_spent_seconds: 10,
                        confidence: None,
                        mistakes: String::new(),
                        review_status: "unreviewed".to_owned(),
                        notes: String::new(),
                    },
                )
            })
            .expect("second attempt");
        assert_eq!(second.status, "incorrect");
        assert_eq!(second.marks_obtained, -0.5);
        assert_eq!(second.correct_answer_at_attempt, "A");
        assert_eq!(second.answer_key_version, "key-2");
    }

    #[test]
    fn numeric_tolerance_is_engine_enforced() {
        let (_root, state) = state();
        let (pattern_id, section_id, type_id) = state
            .with_connection(|connection| {
                let options = super::pyq::options(connection, V2PracticeExamInput { exam_id: 4 })?;
                let version = options
                    .patterns
                    .iter()
                    .find(|pattern| {
                        pattern.verification_status == "verified"
                            && pattern.version_label == "JEST 2026"
                    })
                    .expect("verified JEST pattern");
                let section = version
                    .sections
                    .iter()
                    .find(|section| section.code == "jest-part-c")
                    .expect("JEST Part C");
                let nat = options
                    .question_types
                    .iter()
                    .find(|question_type| question_type.code == "NAT")
                    .expect("NAT");
                Ok((version.id, section.section_id, nat.id))
            })
            .expect("JEST pattern ids");
        let saved = state
            .with_connection(|connection| {
                super::pyq::save_question(
                    connection,
                    V2PyqQuestionInput {
                        id: None,
                        exam_id: 4,
                        year: 2097,
                        session_label: "Tolerance test".to_owned(),
                        section_id,
                        question_type_id: type_id,
                        question_number: "N1".to_owned(),
                        statement: "Synthetic tolerance fixture.".to_owned(),
                        correct_answer: "1.0".to_owned(),
                        correct_range: String::new(),
                        max_marks: 3.0,
                        negative_marks: 0.0,
                        unanswered_marks: 0.0,
                        difficulty: 3,
                        source_title: "Synthetic source".to_owned(),
                        source_page: "1".to_owned(),
                        source_url: None,
                        answer_key_source: "Synthetic key".to_owned(),
                        answer_key_version: "tolerance-1".to_owned(),
                        answer_key_verified: true,
                        verification_status: "verified".to_owned(),
                        numeric_tolerance: 0.01,
                        options: Vec::new(),
                        pattern_version_id: Some(pattern_id),
                        syllabus_node_id: None,
                        topic_ids: Vec::new(),
                        concept_ids: Vec::new(),
                        resource_id: None,
                        source_id: None,
                        answer_key_id: None,
                        dedupe_key: String::new(),
                        official_explanation: String::new(),
                        third_party_solution: String::new(),
                        user_note: String::new(),
                    },
                )
            })
            .expect("save numeric question");
        let within = state
            .with_connection(|connection| {
                super::pyq::record_attempt(
                    connection,
                    super::pyq::V2PyqAttemptInput {
                        exam_id: 4,
                        question_id: saved.question.id,
                        attempt_date: "2026-11-05".to_owned(),
                        selected_answer: "1.005".to_owned(),
                        time_spent_seconds: 10,
                        confidence: None,
                        mistakes: String::new(),
                        review_status: "unreviewed".to_owned(),
                        notes: String::new(),
                    },
                )
            })
            .expect("within tolerance");
        assert_eq!(within.status, "correct");
        assert_eq!(within.marks_obtained, 3.0);
        let outside = state
            .with_connection(|connection| {
                super::pyq::record_attempt(
                    connection,
                    super::pyq::V2PyqAttemptInput {
                        exam_id: 4,
                        question_id: saved.question.id,
                        attempt_date: "2026-11-05".to_owned(),
                        selected_answer: "1.02".to_owned(),
                        time_spent_seconds: 10,
                        confidence: None,
                        mistakes: String::new(),
                        review_status: "unreviewed".to_owned(),
                        notes: String::new(),
                    },
                )
            })
            .expect("outside tolerance");
        assert_eq!(outside.status, "incorrect");
        assert_eq!(outside.marks_obtained, 0.0);
    }
}
