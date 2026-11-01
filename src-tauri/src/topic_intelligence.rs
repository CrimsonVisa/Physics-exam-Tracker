use std::collections::HashMap;

use chrono::NaiveDate;
use rusqlite::Connection;
use serde::Serialize;

use crate::{
    analytics::{
        self, EVIDENCE_WINDOW_DAYS, MINIMUM_EVIDENCE_QUESTIONS, STRONG_ACCURACY_THRESHOLD,
        WEAK_ACCURACY_THRESHOLD,
    },
    error::AppError,
    revision, syllabus,
};

pub const HIGH_CONFIDENCE_MIN: i64 = 4;
pub const LOW_CONFIDENCE_MAX: i64 = 2;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicIntelligenceThresholds {
    pub minimum_scored_questions: i64,
    pub evidence_window_days: i64,
    pub weak_accuracy_threshold: f64,
    pub strong_accuracy_threshold: f64,
    pub high_confidence_minimum: i64,
    pub low_confidence_maximum: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicEvidence {
    pub topic_id: i64,
    pub code: String,
    pub title: String,
    pub section_code: String,
    pub section_title: String,
    pub syllabus_status: String,
    pub confidence: i64,
    pub revision_status: Option<String>,
    pub due_state: String,
    pub next_revision_date: Option<String>,
    pub days_overdue: i64,
    pub days_until_due: Option<i64>,
    pub completed_revisions: i64,
    pub previous_outcome: Option<String>,
    pub mastery_estimate: Option<f64>,
    pub pyq_correct: i64,
    pub pyq_incorrect: i64,
    pub pyq_skipped: i64,
    pub pyq_attempts: i64,
    pub pyq_total_attempts: i64,
    pub pyq_total_questions: i64,
    pub pyq_questions: i64,
    pub pyq_accuracy: Option<f64>,
    pub pyq_evidence_accuracy: Option<f64>,
    pub pyq_last_attempt_date: Option<String>,
    pub skipped_only: bool,
    pub sufficient_data: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicIntelligence {
    pub thresholds: TopicIntelligenceThresholds,
    pub total_topics: i64,
    pub scored_topics: i64,
    pub no_data_topics: i64,
    pub skipped_only_topics: i64,
    pub strong: Vec<TopicEvidence>,
    pub weak: Vec<TopicEvidence>,
    pub needs_revision: Vec<TopicEvidence>,
    pub insufficient: Vec<TopicEvidence>,
    pub overdue_revision: Vec<TopicEvidence>,
    pub high_confidence_poor_pyq: Vec<TopicEvidence>,
    pub low_confidence_strong_pyq: Vec<TopicEvidence>,
    pub notes: Vec<String>,
}

pub fn topic_intelligence(
    connection: &Connection,
    today: NaiveDate,
) -> Result<TopicIntelligence, AppError> {
    let pyq = analytics::pyq(connection, today)?;
    let revision_dashboard = revision::dashboard(connection, today)?;
    let syllabus_topics = syllabus::search_topics(connection, syllabus::SyllabusFilter::default())?;

    let pyq_by_topic = pyq
        .by_topic
        .iter()
        .map(|topic| (topic.topic_id, topic))
        .collect::<HashMap<_, _>>();
    let revision_by_topic = revision_dashboard
        .topics
        .iter()
        .map(|state| (state.topic.id, state))
        .collect::<HashMap<_, _>>();

    let mut strong = Vec::new();
    let mut weak = Vec::new();
    let mut needs_revision = Vec::new();
    let mut insufficient = Vec::new();
    let mut overdue_revision = Vec::new();
    let mut high_confidence_poor_pyq = Vec::new();
    let mut low_confidence_strong_pyq = Vec::new();
    let mut scored_topics = 0_i64;
    let mut no_data_topics = 0_i64;
    let mut skipped_only_topics = 0_i64;

    for topic in &syllabus_topics {
        let stats = pyq_by_topic.get(&topic.id);
        let state = revision_by_topic.get(&topic.id);
        let attempts = stats.map_or(0, |entry| entry.attempts);
        let total_attempts = stats.map_or(0, |entry| entry.total_attempts);
        let total_questions = stats.map_or(0, |entry| entry.distinct_questions);
        let questions = stats.map_or(0, |entry| entry.evidence_questions);
        let evidence_accuracy = stats.and_then(|entry| entry.evidence_accuracy);
        let skipped_only = stats.is_some_and(|entry| entry.skipped_only);
        if attempts == 0 {
            no_data_topics += 1;
        } else {
            scored_topics += 1;
        }
        if skipped_only {
            skipped_only_topics += 1;
        }

        let base = TopicEvidence {
            topic_id: topic.id,
            code: topic.code.clone(),
            title: topic.title.clone(),
            section_code: topic.section_code.clone(),
            section_title: topic.section_title.clone(),
            syllabus_status: topic.progress.status.clone(),
            confidence: topic.progress.confidence,
            revision_status: state.and_then(|entry| {
                entry
                    .schedule
                    .as_ref()
                    .map(|schedule| schedule.status.clone())
            }),
            due_state: state.map_or_else(
                || "not_scheduled".to_owned(),
                |entry| entry.due_state.clone(),
            ),
            next_revision_date: state.and_then(|entry| {
                entry
                    .schedule
                    .as_ref()
                    .map(|schedule| schedule.next_revision_date.clone())
            }),
            days_overdue: state.map_or(0, |entry| entry.days_overdue),
            days_until_due: state.and_then(|entry| entry.days_until_due),
            completed_revisions: state
                .and_then(|entry| entry.schedule.as_ref())
                .map_or(0, |schedule| schedule.completed_revisions),
            previous_outcome: state.and_then(|entry| {
                entry
                    .schedule
                    .as_ref()
                    .and_then(|schedule| schedule.previous_outcome.clone())
            }),
            mastery_estimate: state.and_then(|entry| {
                entry
                    .schedule
                    .as_ref()
                    .map(|schedule| schedule.mastery_estimate)
            }),
            pyq_correct: stats.map_or(0, |entry| entry.correct),
            pyq_incorrect: stats.map_or(0, |entry| entry.incorrect),
            pyq_skipped: stats.map_or(0, |entry| entry.skipped),
            pyq_attempts: attempts,
            pyq_total_attempts: total_attempts,
            pyq_total_questions: total_questions,
            pyq_questions: questions,
            pyq_accuracy: stats.and_then(|entry| entry.accuracy),
            pyq_evidence_accuracy: evidence_accuracy,
            pyq_last_attempt_date: stats
                .and_then(|entry| entry.last_scored_attempt_date.as_deref().map(str::to_owned)),
            skipped_only,
            sufficient_data: questions >= MINIMUM_EVIDENCE_QUESTIONS,
            reasons: Vec::new(),
        };

        let strong_evidence = questions >= MINIMUM_EVIDENCE_QUESTIONS
            && base
                .pyq_evidence_accuracy
                .is_some_and(|accuracy| accuracy >= STRONG_ACCURACY_THRESHOLD);
        let weak_evidence = questions >= MINIMUM_EVIDENCE_QUESTIONS
            && base
                .pyq_evidence_accuracy
                .is_some_and(|accuracy| accuracy < WEAK_ACCURACY_THRESHOLD);

        if strong_evidence {
            let mut item = base.clone();
            item.reasons.push(format!(
                "PYQ accuracy {:.1}% across {} distinct questions in the last {} days using each question's latest scored outcome (strong threshold: {:.0}% with at least {} questions).",
                base.pyq_evidence_accuracy.unwrap_or(0.0),
                questions,
                EVIDENCE_WINDOW_DAYS,
                STRONG_ACCURACY_THRESHOLD,
                MINIMUM_EVIDENCE_QUESTIONS
            ));
            strong.push(item);
        }

        if weak_evidence {
            let mut item = base.clone();
            item.reasons.push(format!(
                "PYQ accuracy {:.1}% across {} distinct questions in the last {} days using each question's latest scored outcome (weak threshold: below {:.0}% with at least {} questions).",
                base.pyq_evidence_accuracy.unwrap_or(0.0),
                questions,
                EVIDENCE_WINDOW_DAYS,
                WEAK_ACCURACY_THRESHOLD,
                MINIMUM_EVIDENCE_QUESTIONS
            ));
            weak.push(item);
        }

        if total_attempts > 0 && questions < MINIMUM_EVIDENCE_QUESTIONS {
            let mut item = base.clone();
            item.reasons.push(if skipped_only {
                "Only skipped PYQ attempts are recorded for this topic; no scored outcome is available yet.".to_owned()
            } else if questions == 0 && total_questions > 0 {
                format!(
                    "Scored PYQ outcomes exist, but none are within the last {} days. Older outcomes remain in history but do not support a current weak or strong claim.",
                    EVIDENCE_WINDOW_DAYS
                )
            } else if questions == 0 {
                "PYQ attempts are recorded without a scored correct or incorrect outcome; no accuracy claim is available yet.".to_owned()
            } else {
                format!(
                    "Only {} distinct question{} with a scored outcome in the last {} days; at least {} are required before a weak or strong claim.",
                    questions,
                    if questions == 1 { "" } else { "s" },
                    EVIDENCE_WINDOW_DAYS,
                    MINIMUM_EVIDENCE_QUESTIONS
                )
            });
            insufficient.push(item);
        }

        let mut revision_reasons = revision_reasons(&base);
        if weak_evidence {
            revision_reasons.push(format!(
                "Sufficient PYQ evidence is below the weak-topic threshold ({:.1}% across {} distinct questions in the last {} days, latest outcome per question).",
                base.pyq_evidence_accuracy.unwrap_or(0.0),
                questions,
                EVIDENCE_WINDOW_DAYS
            ));
        }
        if !revision_reasons.is_empty() {
            let mut item = base.clone();
            item.reasons.extend(revision_reasons);
            needs_revision.push(item);
        }

        if base.due_state == "overdue" {
            let mut item = base.clone();
            item.reasons.push(format!(
                "Revision is {} day{} overdue.",
                base.days_overdue,
                if base.days_overdue == 1 { "" } else { "s" }
            ));
            overdue_revision.push(item);
        }

        if base.confidence >= HIGH_CONFIDENCE_MIN && weak_evidence {
            let mut item = base.clone();
            item.reasons.push(format!(
                "Confidence is {}/5 but PYQ evidence accuracy is {:.1}% across {} distinct questions. The confidence rating and saved outcomes disagree.",
                base.confidence,
                base.pyq_evidence_accuracy.unwrap_or(0.0),
                questions
            ));
            high_confidence_poor_pyq.push(item);
        }

        if base.confidence <= LOW_CONFIDENCE_MAX && strong_evidence {
            let mut item = base.clone();
            item.reasons.push(format!(
                "Confidence is {}/5 but PYQ evidence accuracy is {:.1}% across {} distinct questions. Saved outcomes are stronger than the confidence rating.",
                base.confidence,
                base.pyq_evidence_accuracy.unwrap_or(0.0),
                questions
            ));
            low_confidence_strong_pyq.push(item);
        }
    }

    sort_by_accuracy_desc(&mut strong);
    sort_by_accuracy_asc(&mut weak);
    needs_revision.sort_by(|left, right| {
        revision_urgency(left)
            .cmp(&revision_urgency(right))
            .then_with(|| right.days_overdue.cmp(&left.days_overdue))
            .then_with(|| left.code.cmp(&right.code))
    });
    overdue_revision.sort_by(|left, right| {
        right
            .days_overdue
            .cmp(&left.days_overdue)
            .then_with(|| left.code.cmp(&right.code))
    });
    insufficient.sort_by(|left, right| {
        right
            .pyq_questions
            .cmp(&left.pyq_questions)
            .then_with(|| right.pyq_total_attempts.cmp(&left.pyq_total_attempts))
            .then_with(|| left.code.cmp(&right.code))
    });
    high_confidence_poor_pyq.sort_by(|left, right| {
        right.confidence.cmp(&left.confidence).then_with(|| {
            left.pyq_evidence_accuracy
                .partial_cmp(&right.pyq_evidence_accuracy)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    low_confidence_strong_pyq.sort_by(|left, right| {
        right
            .pyq_evidence_accuracy
            .partial_cmp(&left.pyq_evidence_accuracy)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(TopicIntelligence {
        thresholds: TopicIntelligenceThresholds {
            minimum_scored_questions: MINIMUM_EVIDENCE_QUESTIONS,
            evidence_window_days: EVIDENCE_WINDOW_DAYS,
            weak_accuracy_threshold: WEAK_ACCURACY_THRESHOLD,
            strong_accuracy_threshold: STRONG_ACCURACY_THRESHOLD,
            high_confidence_minimum: HIGH_CONFIDENCE_MIN,
            low_confidence_maximum: LOW_CONFIDENCE_MAX,
        },
        total_topics: syllabus_topics.len() as i64,
        scored_topics,
        no_data_topics,
        skipped_only_topics,
        strong,
        weak,
        needs_revision,
        insufficient,
        overdue_revision,
        high_confidence_poor_pyq,
        low_confidence_strong_pyq,
        notes: vec![
            "Topic intelligence uses saved PYQ evidence and revision records only. Syllabus status and confidence are shown as user-entered context, not proof of mastery.".to_owned(),
            format!(
                "A topic needs scored outcomes for at least five distinct question references within the last {} days before a weak or strong claim. Each question's latest scored outcome in that window is used, so repeated attempts cannot inflate the evidence count. Older outcomes remain in history and skipped-only topics are reported separately.",
                EVIDENCE_WINDOW_DAYS
            ),
        ],
    })
}

fn revision_reasons(evidence: &TopicEvidence) -> Vec<String> {
    let mut reasons = Vec::new();
    match evidence.due_state.as_str() {
        "overdue" => reasons.push(format!(
            "Revision is {} day{} overdue.",
            evidence.days_overdue,
            if evidence.days_overdue == 1 { "" } else { "s" }
        )),
        "due_today" => reasons.push("Revision is due today.".to_owned()),
        "upcoming"
            if evidence
                .mastery_estimate
                .is_some_and(|mastery| mastery < 40.0)
                && evidence.completed_revisions > 0 =>
        {
            reasons.push(format!(
                "Revision mastery estimate is {:.1}%, below 40%.",
                evidence.mastery_estimate.unwrap_or(0.0)
            ));
        }
        _ => {}
    }

    match evidence.previous_outcome.as_deref() {
        Some("again") => reasons.push("Previous revision outcome was Again.".to_owned()),
        Some("difficult") => reasons.push("Previous revision outcome was Difficult.".to_owned()),
        _ => {}
    }

    reasons
}

fn revision_urgency(evidence: &TopicEvidence) -> i64 {
    match evidence.due_state.as_str() {
        "overdue" => 0,
        "due_today" => 1,
        "upcoming" => 2,
        _ => 3,
    }
}

fn sort_by_accuracy_desc(items: &mut [TopicEvidence]) {
    items.sort_by(|left, right| {
        right
            .pyq_evidence_accuracy
            .partial_cmp(&left.pyq_evidence_accuracy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.pyq_questions.cmp(&left.pyq_questions))
            .then_with(|| left.code.cmp(&right.code))
    });
}

fn sort_by_accuracy_asc(items: &mut [TopicEvidence]) {
    items.sort_by(|left, right| {
        left.pyq_evidence_accuracy
            .partial_cmp(&right.pyq_evidence_accuracy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.pyq_questions.cmp(&left.pyq_questions))
            .then_with(|| left.code.cmp(&right.code))
    });
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rusqlite::Connection;

    use super::*;
    use crate::{db, pyq, revision, syllabus};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn connection() -> Connection {
        let mut connection = db::test_connection_with_defaults(date(2026, 10, 30));
        syllabus::seed(&mut connection).expect("syllabus should seed");
        connection
    }

    fn topic_id(connection: &Connection, code: &str) -> i64 {
        connection
            .query_row(
                "SELECT id FROM syllabus_topics WHERE code = ?1",
                [code],
                |row| row.get(0),
            )
            .expect("topic should exist")
    }

    fn progress(connection: &mut Connection, topic_id: i64, status: &str, confidence: i64) {
        syllabus::update_progress(
            connection,
            topic_id,
            syllabus::TopicProgressInput {
                status: status.to_owned(),
                priority: "High".to_owned(),
                confidence,
                notes: String::new(),
            },
        )
        .expect("progress should save");
    }

    fn pyq_attempt(connection: &mut Connection, topic_id: i64, status: &str, reference: &str) {
        pyq_attempt_on(connection, topic_id, status, reference, "2026-10-30");
    }

    fn pyq_attempt_on(
        connection: &mut Connection,
        topic_id: i64,
        status: &str,
        reference: &str,
        attempt_date: &str,
    ) {
        pyq::create(
            connection,
            pyq::PyqInput {
                question_year: 2024,
                exam_session: String::new(),
                part: "B".to_owned(),
                topic_id,
                question_reference: reference.to_owned(),
                attempt_date: attempt_date.to_owned(),
                status: status.to_owned(),
                marks_obtained: None,
                max_marks: None,
                time_spent_seconds: 60,
                notes: String::new(),
                difficulty: None,
            },
        )
        .expect("PYQ should save");
    }

    #[test]
    fn evidence_thresholds_group_topics_and_preserve_insufficient_data() {
        let mut connection = connection();
        let weak = topic_id(&connection, "MMP-C01");
        let strong = topic_id(&connection, "GA-01");
        let insufficient = topic_id(&connection, "CM-C01");
        let skipped_only = topic_id(&connection, "MMP-C02");
        let overdue = topic_id(&connection, "EMT-C01");

        progress(&mut connection, weak, "Confident", 5);
        progress(&mut connection, strong, "Learning", 2);
        for index in 0..6 {
            pyq_attempt(&mut connection, weak, "incorrect", &format!("WEAK-{index}"));
        }
        for index in 0..5 {
            pyq_attempt(
                &mut connection,
                strong,
                "correct",
                &format!("STRONG-{index}"),
            );
        }
        for index in 0..2 {
            pyq_attempt(
                &mut connection,
                insufficient,
                "incorrect",
                &format!("LOW-{index}"),
            );
        }
        for index in 0..2 {
            pyq_attempt(
                &mut connection,
                skipped_only,
                "skipped",
                &format!("SKIPPED-{index}"),
            );
        }
        revision::schedule_topic(&mut connection, overdue, "2026-10-27", date(2026, 10, 30))
            .expect("schedule should save");

        let intelligence =
            topic_intelligence(&connection, date(2026, 10, 30)).expect("intelligence should load");
        assert_eq!(intelligence.total_topics, 182);
        assert_eq!(intelligence.thresholds.minimum_scored_questions, 5);
        assert_eq!(
            intelligence.thresholds.weak_accuracy_threshold,
            WEAK_ACCURACY_THRESHOLD
        );
        assert_eq!(
            intelligence.thresholds.strong_accuracy_threshold,
            STRONG_ACCURACY_THRESHOLD
        );

        let weak_item = intelligence
            .weak
            .iter()
            .find(|item| item.code == "MMP-C01")
            .expect("weak topic should be present");
        assert_eq!(weak_item.pyq_attempts, 6);
        assert_eq!(weak_item.pyq_questions, 6);
        assert_eq!(weak_item.pyq_accuracy, Some(0.0));
        assert_eq!(weak_item.pyq_evidence_accuracy, Some(0.0));
        assert!(
            weak_item
                .reasons
                .iter()
                .any(|reason| reason.contains("Weak threshold"))
                || weak_item
                    .reasons
                    .iter()
                    .any(|reason| reason.contains("weak threshold"))
        );

        let strong_item = intelligence
            .strong
            .iter()
            .find(|item| item.code == "GA-01")
            .expect("strong topic should be present");
        assert_eq!(strong_item.pyq_accuracy, Some(100.0));
        assert_eq!(strong_item.pyq_questions, 5);

        let insufficient_item = intelligence
            .insufficient
            .iter()
            .find(|item| item.code == "CM-C01")
            .expect("insufficient topic should be present");
        assert!(
            insufficient_item
                .reasons
                .iter()
                .any(|reason| reason.contains("Only 2 distinct questions"))
        );
        assert!(!intelligence.weak.iter().any(|item| item.code == "CM-C01"));

        let skipped_item = intelligence
            .insufficient
            .iter()
            .find(|item| item.code == "MMP-C02")
            .expect("skipped-only topic should be reported");
        assert!(skipped_item.skipped_only);
        assert_eq!(skipped_item.pyq_questions, 0);
        assert_eq!(skipped_item.pyq_total_attempts, 2);
        assert!(
            skipped_item
                .reasons
                .iter()
                .any(|reason| reason.contains("Only skipped PYQ attempts"))
        );
        assert_eq!(intelligence.skipped_only_topics, 1);

        assert!(
            intelligence
                .high_confidence_poor_pyq
                .iter()
                .any(|item| item.code == "MMP-C01")
        );
        assert!(
            intelligence
                .low_confidence_strong_pyq
                .iter()
                .any(|item| item.code == "GA-01")
        );
        assert!(
            intelligence
                .overdue_revision
                .iter()
                .any(|item| item.code == "EMT-C01")
        );
        assert!(
            intelligence
                .needs_revision
                .iter()
                .any(|item| item.code == "EMT-C01")
        );
        assert_eq!(intelligence.no_data_topics, 179);
        assert!(
            intelligence
                .notes
                .iter()
                .any(|note| note.contains("not proof of mastery"))
        );
    }

    #[test]
    fn stale_pyq_evidence_does_not_qualify_for_weak_or_strong_claims() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        for index in 0..6 {
            pyq_attempt_on(
                &mut connection,
                topic,
                "incorrect",
                &format!("STALE-{index}"),
                "2019-06-01",
            );
        }

        let intelligence =
            topic_intelligence(&connection, date(2026, 10, 30)).expect("intelligence should load");
        let evidence = intelligence
            .insufficient
            .iter()
            .find(|item| item.code == "MMP-C01")
            .expect("stale topic should be reported as insufficient");
        assert_eq!(evidence.pyq_questions, 0);
        assert_eq!(evidence.pyq_total_questions, 6);
        assert!(!evidence.sufficient_data);
        assert!(
            evidence
                .reasons
                .iter()
                .any(|reason| { reason.contains("none are within the last 365 days") })
        );
        assert!(!intelligence.weak.iter().any(|item| item.code == "MMP-C01"));
        assert!(
            !intelligence
                .strong
                .iter()
                .any(|item| item.code == "MMP-C01")
        );
    }
}
