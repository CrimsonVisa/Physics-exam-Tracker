use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredResponse {
    pub status: String,
    pub marks: f64,
    pub normalized_answer: String,
    pub engine: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerKind {
    SingleChoice,
    MultiChoice,
    Numeric,
}

impl AnswerKind {
    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_ascii_uppercase().as_str() {
            "MCQ" | "SINGLE" | "SINGLE_CHOICE" => Some(Self::SingleChoice),
            "MSQ" | "MULTI" | "MULTIPLE_CHOICE" => Some(Self::MultiChoice),
            "NAT" | "NUMERICAL" | "NUMERIC" => Some(Self::Numeric),
            _ => None,
        }
    }
}

pub fn normalize_choice(value: &str) -> String {
    let mut values = value
        .split([',', ';', '|', '/'])
        .map(|part| part.trim().to_ascii_uppercase())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values.join(",")
}

pub fn normalize_numeric(value: &str) -> Option<f64> {
    value.trim().replace(',', "").parse::<f64>().ok()
}

fn numeric_match(correct: &str, range: &str, selected: &str, tolerance: f64) -> bool {
    let Some(selected) = normalize_numeric(selected) else {
        return false;
    };
    if !range.trim().is_empty() {
        let parts = range
            .split([',', ';', ' '])
            .filter(|part| !part.trim().is_empty())
            .filter_map(normalize_numeric)
            .collect::<Vec<_>>();
        if parts.len() == 2 {
            return selected >= parts[0] - tolerance && selected <= parts[1] + tolerance;
        }
    }
    normalize_numeric(correct)
        .map(|value| (selected - value).abs() <= tolerance.max(1e-9))
        .unwrap_or(false)
}

fn choice_match(correct: &str, selected: &str, kind: AnswerKind) -> bool {
    let correct = normalize_choice(correct);
    let selected = normalize_choice(selected);
    if correct.is_empty() || selected.is_empty() {
        return false;
    }
    match kind {
        AnswerKind::SingleChoice => {
            !selected.contains(',') && !correct.contains(',') && selected == correct
        }
        AnswerKind::MultiChoice => selected == correct,
        AnswerKind::Numeric => unreachable!(),
    }
}

pub fn score_csir(
    kind: AnswerKind,
    correct_answer: &str,
    correct_range: &str,
    selected: &str,
    correct_marks: f64,
    incorrect_marks: f64,
    tolerance: f64,
) -> ScoredResponse {
    if selected.trim().is_empty() {
        return ScoredResponse {
            status: "skipped".to_owned(),
            marks: 0.0,
            normalized_answer: String::new(),
            engine: "csir-net-physics".to_owned(),
        };
    }
    let correct = match kind {
        AnswerKind::Numeric => numeric_match(correct_answer, correct_range, selected, tolerance),
        _ => choice_match(correct_answer, selected, kind),
    };
    ScoredResponse {
        status: if correct { "correct" } else { "incorrect" }.to_owned(),
        marks: if correct {
            correct_marks
        } else {
            incorrect_marks
        },
        normalized_answer: if kind == AnswerKind::Numeric {
            normalize_numeric(selected)
                .map(|value| value.to_string())
                .unwrap_or_else(|| selected.trim().to_owned())
        } else {
            normalize_choice(selected)
        },
        engine: "csir-net-physics".to_owned(),
    }
}

pub fn score_gate(
    kind: AnswerKind,
    question_marks: f64,
    correct_answer: &str,
    correct_range: &str,
    selected: &str,
    tolerance: f64,
) -> ScoredResponse {
    if selected.trim().is_empty() {
        return ScoredResponse {
            status: "skipped".to_owned(),
            marks: 0.0,
            normalized_answer: String::new(),
            engine: "gate-physics".to_owned(),
        };
    }
    let correct = match kind {
        AnswerKind::Numeric => numeric_match(correct_answer, correct_range, selected, tolerance),
        _ => choice_match(correct_answer, selected, kind),
    };
    let incorrect_marks = match kind {
        AnswerKind::SingleChoice => {
            if question_marks >= 2.0 {
                -2.0 / 3.0
            } else {
                -1.0 / 3.0
            }
        }
        AnswerKind::MultiChoice | AnswerKind::Numeric => 0.0,
    };
    ScoredResponse {
        status: if correct { "correct" } else { "incorrect" }.to_owned(),
        marks: if correct {
            question_marks
        } else {
            incorrect_marks
        },
        normalized_answer: if kind == AnswerKind::Numeric {
            normalize_numeric(selected)
                .map(|value| value.to_string())
                .unwrap_or_else(|| selected.trim().to_owned())
        } else {
            normalize_choice(selected)
        },
        engine: "gate-physics".to_owned(),
    }
}

pub fn score_tifr(
    section_code: &str,
    kind: AnswerKind,
    correct_answer: &str,
    correct_range: &str,
    selected: &str,
    tolerance: f64,
) -> ScoredResponse {
    if selected.trim().is_empty() {
        return ScoredResponse {
            status: "skipped".to_owned(),
            marks: 0.0,
            normalized_answer: String::new(),
            engine: "tifr-gs-physics".to_owned(),
        };
    }
    let correct = match kind {
        AnswerKind::Numeric => numeric_match(correct_answer, correct_range, selected, tolerance),
        _ => choice_match(correct_answer, selected, kind),
    };
    let (award, penalty) = if section_code.eq_ignore_ascii_case("tifr-gs-section-b") {
        (5.0, 0.0)
    } else {
        (3.0, -1.0)
    };
    ScoredResponse {
        status: if correct { "correct" } else { "incorrect" }.to_owned(),
        marks: if correct { award } else { penalty },
        normalized_answer: normalize_choice(selected),
        engine: "tifr-gs-physics".to_owned(),
    }
}

pub fn score_jest(
    section_code: &str,
    kind: AnswerKind,
    correct_answer: &str,
    correct_range: &str,
    selected: &str,
    tolerance: f64,
) -> ScoredResponse {
    if selected.trim().is_empty() {
        return ScoredResponse {
            status: "skipped".to_owned(),
            marks: 0.0,
            normalized_answer: String::new(),
            engine: "jest-physics".to_owned(),
        };
    }
    let correct = match kind {
        AnswerKind::Numeric => numeric_match(correct_answer, correct_range, selected, tolerance),
        _ => choice_match(correct_answer, selected, kind),
    };
    let (award, penalty) = if section_code.eq_ignore_ascii_case("jest-part-a") {
        (1.0, -1.0 / 3.0)
    } else if section_code.eq_ignore_ascii_case("jest-part-b") {
        (3.0, -1.0)
    } else {
        (3.0, 0.0)
    };
    ScoredResponse {
        status: if correct { "correct" } else { "incorrect" }.to_owned(),
        marks: if correct { award } else { penalty },
        normalized_answer: if kind == AnswerKind::Numeric {
            normalize_numeric(selected)
                .map(|value| value.to_string())
                .unwrap_or_else(|| selected.trim().to_owned())
        } else {
            normalize_choice(selected)
        },
        engine: "jest-physics".to_owned(),
    }
}

#[allow(dead_code)]
/// CSIR evaluates only the first N answered questions in a part. Ordered
/// responses make this rule explicit and testable; aggregate mocks do not
/// invent an order when only totals are available.
pub fn score_csir_first_n(
    responses: &[Option<ScoredResponse>],
    limit: usize,
    correct_marks: f64,
    incorrect_marks: f64,
) -> (f64, usize) {
    let mut seen = 0usize;
    let mut total = 0.0;
    for response in responses {
        let Some(response) = response else {
            continue;
        };
        if response.status == "skipped" {
            continue;
        }
        if seen >= limit {
            break;
        }
        seen += 1;
        total += if response.status == "correct" {
            correct_marks
        } else {
            incorrect_marks
        };
    }
    (total, seen)
}

#[allow(clippy::too_many_arguments)]
pub fn score_question(
    exam_code: &str,
    section_code: &str,
    type_code: &str,
    question_marks: f64,
    correct_answer: &str,
    correct_range: &str,
    selected: &str,
    tolerance: f64,
) -> Result<ScoredResponse, String> {
    let kind = AnswerKind::from_code(type_code)
        .ok_or_else(|| format!("Unsupported question type '{type_code}'."))?;
    Ok(match exam_code {
        "csir-net-physics" => score_csir(
            kind,
            correct_answer,
            correct_range,
            selected,
            question_marks,
            if question_marks >= 5.0 {
                -1.25
            } else if question_marks >= 3.0 {
                -0.875
            } else {
                -0.5
            },
            tolerance,
        ),
        "gate-physics" => score_gate(
            kind,
            question_marks,
            correct_answer,
            correct_range,
            selected,
            tolerance,
        ),
        "tifr-gs-physics" => score_tifr(
            section_code,
            kind,
            correct_answer,
            correct_range,
            selected,
            tolerance,
        ),
        "jest-physics" => score_jest(
            section_code,
            kind,
            correct_answer,
            correct_range,
            selected,
            tolerance,
        ),
        other => return Err(format!("No scoring engine is configured for '{other}'.")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csir_marks_and_first_n_rule_are_explicit() {
        let correct = score_csir(AnswerKind::SingleChoice, "B", "", "b", 2.0, -0.5, 0.0);
        assert_eq!(correct.status, "correct");
        assert_eq!(correct.marks, 2.0);
        let wrong = score_csir(AnswerKind::SingleChoice, "B", "", "A", 2.0, -0.5, 0.0);
        assert_eq!(wrong.status, "incorrect");
        assert_eq!(wrong.marks, -0.5);
        let responses = vec![
            Some(correct.clone()),
            Some(wrong.clone()),
            Some(correct.clone()),
            Some(wrong.clone()),
        ];
        let (marks, counted) = score_csir_first_n(&responses, 2, 2.0, -0.5);
        assert_eq!(counted, 2);
        assert_eq!(marks, 1.5);
    }

    #[test]
    fn gate_distinguishes_mcq_msq_and_nat() {
        let mcq1 = score_gate(AnswerKind::SingleChoice, 1.0, "A", "", "B", 0.0);
        assert!((mcq1.marks + 1.0 / 3.0).abs() < 1e-9);
        let mcq2 = score_gate(AnswerKind::SingleChoice, 2.0, "A", "", "B", 0.0);
        assert!((mcq2.marks + 2.0 / 3.0).abs() < 1e-9);
        let msq_wrong = score_gate(AnswerKind::MultiChoice, 2.0, "A,C", "", "A", 0.0);
        assert_eq!(msq_wrong.status, "incorrect");
        assert_eq!(msq_wrong.marks, 0.0);
        let msq_correct = score_gate(AnswerKind::MultiChoice, 2.0, "C,A", "", "A,C", 0.0);
        assert_eq!(msq_correct.marks, 2.0);
        let nat_wrong = score_gate(AnswerKind::Numeric, 2.0, "1.0", "", "1.2", 0.01);
        assert_eq!(nat_wrong.marks, 0.0);
        let nat_correct = score_gate(AnswerKind::Numeric, 2.0, "1.0", "", "1.01", 0.02);
        assert_eq!(nat_correct.marks, 2.0);
    }

    #[test]
    fn tifr_section_and_jest_part_rules_differ() {
        let tifr_a = score_tifr(
            "tifr-gs-section-a",
            AnswerKind::SingleChoice,
            "B",
            "",
            "A",
            0.0,
        );
        assert_eq!(tifr_a.marks, -1.0);
        let tifr_b = score_tifr(
            "tifr-gs-section-b",
            AnswerKind::SingleChoice,
            "B",
            "",
            "A",
            0.0,
        );
        assert_eq!(tifr_b.marks, 0.0);
        let jest_a = score_jest("jest-part-a", AnswerKind::SingleChoice, "B", "", "A", 0.0);
        assert!((jest_a.marks + 1.0 / 3.0).abs() < 1e-9);
        let jest_b = score_jest("jest-part-b", AnswerKind::SingleChoice, "B", "", "A", 0.0);
        assert_eq!(jest_b.marks, -1.0);
        let jest_c = score_jest("jest-part-c", AnswerKind::Numeric, "2", "", "3", 0.0);
        assert_eq!(jest_c.marks, 0.0);
    }

    #[test]
    fn unanswered_responses_never_receive_negative_marks() {
        for engine in [
            "csir-net-physics",
            "gate-physics",
            "tifr-gs-physics",
            "jest-physics",
        ] {
            let result = score_question(engine, "jest-part-a", "MCQ", 1.0, "A", "", "", 0.0)
                .expect("engine");
            assert_eq!(result.status, "skipped");
            assert_eq!(result.marks, 0.0);
        }
    }
}
