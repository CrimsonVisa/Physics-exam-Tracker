use std::time::Instant;

use chrono::NaiveDate;
use rusqlite::params;

use crate::{
    analytics, backup, dashboard, db, planner, portability, pyq, revision, sessions,
    study_analytics, syllabus,
};

/// Synthetic-data audit. Run explicitly with:
/// `cargo test --manifest-path src-tauri/Cargo.toml performance_audit -- --ignored --nocapture`
#[test]
#[ignore = "synthetic performance audit"]
fn performance_audit_with_larger_local_dataset() {
    let directory = tempfile::tempdir().expect("temporary directory should be created");
    let path = directory.path().join(db::DATABASE_FILE_NAME);
    let today = NaiveDate::from_ymd_opt(2026, 10, 30).expect("date");
    let mut connection = db::open_database(&path, today).expect("database should open");
    syllabus::seed(&mut connection).expect("syllabus should seed");
    let mut topic_statement = connection
        .prepare("SELECT id FROM syllabus_topics ORDER BY id")
        .expect("topic query");
    let topic_ids = topic_statement
        .query_map([], |row| row.get::<_, i64>(0))
        .expect("topic rows")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("topic ids");
    drop(topic_statement);

    let transaction = connection.transaction().expect("transaction");
    {
        let mut session = transaction
            .prepare(
                "INSERT INTO study_sessions (
                    topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                 ) VALUES (?1, date('2026-01-01', '+' || ?2 || ' days'), ?3, 'manual', ?4,
                    '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            )
            .expect("session statement");
        for index in 0..2_000_i64 {
            session
                .execute(params![
                    topic_ids[(index as usize) % topic_ids.len()],
                    index % 300,
                    600 + (index % 7_200),
                    format!("synthetic session {index}")
                ])
                .expect("session insert");
        }
    }
    {
        let mut pyq = transaction
            .prepare(
                "INSERT INTO pyq_attempts (
                    question_year, exam_session, part, topic_id, question_reference,
                    attempt_date, status, time_spent_seconds, notes, created_at, updated_at
                 ) VALUES (
                    2024, 'June 2024', ?1, ?2, ?3, date('2026-01-01', '+' || ?4 || ' days'),
                    ?5, 60, 'synthetic PYQ', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
                 )",
            )
            .expect("PYQ statement");
        for index in 0..2_000_i64 {
            pyq.execute(params![
                match index % 3 {
                    0 => "A",
                    1 => "B",
                    _ => "C",
                },
                topic_ids[(index as usize) % topic_ids.len()],
                format!("Q-{index}"),
                index % 300,
                if index % 3 == 0 {
                    "incorrect"
                } else {
                    "correct"
                }
            ])
            .expect("PYQ insert");
        }
    }
    {
        let mut task = transaction
            .prepare(
                "INSERT INTO checklist_tasks (
                    title, scheduled_date, deadline_date, topic_id, priority, status,
                    schedule_anchor, completed_at, created_at, updated_at
                 ) VALUES (?1, date('2026-10-01', '+' || ?2 || ' days'),
                    date('2026-10-01', '+' || ?3 || ' days'), ?4, 'Normal', 'open',
                    'manual', NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z')",
            )
            .expect("task statement");
        for index in 0..500_i64 {
            task.execute(params![
                format!("Synthetic task {index}"),
                index % 30,
                (index % 30) + 2,
                topic_ids[(index as usize) % topic_ids.len()]
            ])
            .expect("task insert");
        }
    }
    {
        let mut block = transaction
            .prepare(
                "INSERT INTO planned_study_blocks (
                    title, topic_id, calendar_date, start_time, end_time, status, notes,
                    created_at, updated_at
                 ) VALUES (?1, ?2, date('2026-10-01', '+' || ?3 || ' days'),
                    '10:00', '11:00', 'planned', '', '2026-10-01T00:00:00Z',
                    '2026-10-01T00:00:00Z')",
            )
            .expect("block statement");
        for index in 0..200_i64 {
            block
                .execute(params![
                    format!("Synthetic block {index}"),
                    topic_ids[(index as usize) % topic_ids.len()],
                    index % 30
                ])
                .expect("block insert");
        }
    }
    {
        let mut mock = transaction
            .prepare(
                "INSERT INTO mock_tests (
                    test_name, test_date, duration_minutes, max_marks,
                    part_a_questions, part_a_correct, part_a_incorrect, part_a_unanswered,
                    part_a_correct_marks, part_a_incorrect_marks, part_a_unanswered_marks,
                    part_b_questions, part_b_correct, part_b_incorrect, part_b_unanswered,
                    part_b_correct_marks, part_b_incorrect_marks, part_b_unanswered_marks,
                    part_c_questions, part_c_correct, part_c_incorrect, part_c_unanswered,
                    part_c_correct_marks, part_c_incorrect_marks, part_c_unanswered_marks,
                    notes, review_observations, created_at, updated_at
                 ) VALUES (
                    ?1, date('2026-01-01', '+' || ?2 || ' days'), 180, 200,
                    20, ?3, ?4, 0, 2, -0.5, 0,
                    40, 25, 15, 0, 3, -1, 0,
                    20, 10, 10, 0, 4, -2, 0,
                    '', '', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
                 )",
            )
            .expect("mock statement");
        for index in 0..100_i64 {
            let correct = 10 + (index % 9);
            mock.execute(params![
                format!("Synthetic mock {index}"),
                index % 280,
                correct,
                20 - correct
            ])
            .expect("mock insert");
        }
    }
    {
        let mut schedule = transaction
            .prepare(
                "INSERT INTO revision_schedules (
                    topic_id, status, next_revision_date, current_interval_days,
                    completed_revisions, mastery_estimate, notes, created_at, updated_at
                 ) VALUES (?1, 'active', date('2026-10-01', '+' || ?2 || ' days'),
                    5, 0, 0, '', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z')",
            )
            .expect("schedule statement");
        for (index, topic_id) in topic_ids.iter().take(100).enumerate() {
            schedule
                .execute(params![topic_id, index as i64 % 45])
                .expect("schedule insert");
        }
    }
    transaction.commit().expect("seed commit");

    let started = Instant::now();
    let dashboard = dashboard::preparation_dashboard(&connection, today).expect("dashboard");
    println!(
        "dashboard: {:?} ({} insights)",
        started.elapsed(),
        dashboard.insights.len()
    );

    let started = Instant::now();
    let study = study_analytics::study_analytics(&connection, "2026-01-01", "2026-10-30", today)
        .expect("study analytics");
    println!(
        "study analytics: {:?} ({} days)",
        started.elapsed(),
        study.daily.len()
    );

    let started = Instant::now();
    let performance = analytics::performance(&connection, "2026-01-01", "2026-10-30", today)
        .expect("performance analytics");
    println!(
        "performance analytics: {:?} ({} PYQ topics)",
        started.elapsed(),
        performance.pyq.by_topic.len()
    );

    let started = Instant::now();
    let filtered = pyq::list(
        &connection,
        pyq::PyqFilter {
            query: Some("Q-1".to_owned()),
            ..pyq::PyqFilter::default()
        },
    )
    .expect("PYQ search");
    println!(
        "PYQ filtered search: {:?} ({} rows)",
        started.elapsed(),
        filtered.len()
    );

    let started = Instant::now();
    let calendar =
        planner::calendar(&connection, "2026-10-01", "2026-10-30", today).expect("calendar");
    println!(
        "calendar month: {:?} ({} tasks, {} blocks)",
        started.elapsed(),
        calendar.tasks.len(),
        calendar.blocks.len()
    );

    let started = Instant::now();
    let revisions = revision::dashboard(&connection, today).expect("revision dashboard");
    println!(
        "revision dashboard: {:?} ({} due, {} overdue)",
        started.elapsed(),
        revisions.stats.due_today_count,
        revisions.stats.overdue_count
    );

    let export_dir = directory.path().join("export");
    let started = Instant::now();
    let exported = portability::export_json(&connection, &export_dir).expect("JSON export");
    println!(
        "JSON export: {:?} ({} bytes)",
        started.elapsed(),
        exported.bytes
    );

    let backup_dir = directory.path().join("backups");
    let started = Instant::now();
    let backup = backup::create_backup(&connection, &backup_dir, "manual", None).expect("backup");
    println!(
        "SQLite backup: {:?} ({} bytes)",
        started.elapsed(),
        backup.manifest.backup_size_bytes
    );

    let started = Instant::now();
    let session_count = sessions::list(&connection, sessions::StudySessionFilter::default())
        .expect("sessions")
        .len();
    println!(
        "session history load: {:?} ({} rows)",
        started.elapsed(),
        session_count
    );
}
