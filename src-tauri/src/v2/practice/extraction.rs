use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedKeyEntry {
    pub question_number: String,
    pub session_label: String,
    pub question_type_code: String,
    pub section_code: String,
    pub key_value: String,
    pub marks: f64,
    pub key_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedQuestionOption {
    pub key: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedQuestion {
    pub question_number: String,
    pub page_number: i64,
    pub raw_text: String,
    pub statement: String,
    pub options: Vec<ParsedQuestionOption>,
    pub figure_reference: bool,
    pub encoding_issue: bool,
    pub short_statement: bool,
    pub next_question_text: bool,
    pub option_issues: Vec<String>,
}

fn page_re() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"(?m)^===== PAGE (\d+) =====\s*$").expect("page regex"))
}

fn question_re() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"(?m)^\s*Q\.?\s*(\d{1,2})\b").expect("question regex"))
}

fn option_re() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"\(([A-D])\)").expect("option regex"))
}

fn figure_re() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| {
        Regex::new(
            r"(?i)(shown\s+(in|on|above|below|as|at)|(above|below|frequency\s+)?(figure|chart|graph|table|diagram|circuit|image|picture|plot|curve|grid|map|schematic)|the\s+((bar|line|pie|scatter|frequency)\s+)?(figure|chart|graph|table|diagram|circuit|image|picture|plot|curve|grid|map|schematic)|following\s+(figure|chart|graph|table|diagram|circuit|image|picture|plot|curve|grid|map|schematic)|(figure|chart|graph|table|diagram|circuit|image|picture|plot|curve|grid|map|schematic)\s+(below|above|shown)|as\s+shown|shown\s+on\s+the\s+(left|right)|bar\s+chart|line\s+graph|pie\s+chart|histogram|scatter\s+plot|frequency\s+chart)",
        )
        .expect("figure regex")
    })
}

fn key_header(line: &str) -> bool {
    let line = line.trim();
    line.is_empty()
        || (line.starts_with("Page ") && line.contains(" of "))
        || line.starts_with("Answer Key for Physics")
        || line == "Final Answer Key"
        || line == "Physics (PH)"
        || line == "Physics(PH)"
        || (line.starts_with("GATE ") && line.contains("Physics"))
        || line.starts_with("Q. No.")
        || line == "Q. No."
        || line == "Session"
        || line == "Q. Type"
        || line.starts_with("Question Type")
        || line == "Section"
        || line.starts_with("Subject Name")
        || line.starts_with("Key/Range")
        || line == "Marks"
        || line == "Mark"
        || line.starts_with("MCQ/MSQ/NAT")
        || line == "MTA = Marks to All"
}

fn paper_header(line: &str) -> bool {
    let line = line.trim();
    line.is_empty()
        || (line.starts_with("GATE ") && line.ends_with("Physics (PH)"))
        || line == "Physics (PH)"
        || (line.starts_with("Page ") && line.contains(" of "))
        || line.starts_with("Organizing Institute:")
        || line.starts_with("General Aptitude")
        || line.contains("Carry ONE mark")
        || line.contains("Carry TWO marks")
        || (line.starts_with("Q.") && line.contains(" – Q."))
        || (line.starts_with("Q.") && line.contains(" - Q."))
}

fn parse_number(value: &str) -> Option<f64> {
    value
        .trim()
        .replace(',', "")
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn key_start(tokens: &[String], index: usize) -> Option<(i64, String, String, String)> {
    if index + 3 >= tokens.len() {
        return None;
    }
    let question_number = tokens[index].parse::<i64>().ok()?;
    if !(1..=65).contains(&question_number) {
        return None;
    }
    let session = tokens[index + 1].parse::<i64>().ok()?;
    if session < 0 {
        return None;
    }
    let question_type = tokens[index + 2].to_ascii_uppercase();
    if !matches!(question_type.as_str(), "MCQ" | "MSQ" | "NAT") {
        return None;
    }
    let section = tokens[index + 3].to_ascii_uppercase();
    if !matches!(section.as_str(), "GA" | "PH") {
        return None;
    }
    Some((question_number, session.to_string(), question_type, section))
}

fn key_value(tokens: &[String], index: usize, question_type: &str) -> Option<(String, usize)> {
    let token = tokens.get(index)?.trim();
    if token.eq_ignore_ascii_case("MTA") {
        return Some(("MTA".to_owned(), 1));
    }
    match question_type {
        "MCQ" => {
            if token.len() == 1 && token.chars().all(|ch| ('A'..='D').contains(&ch)) {
                Some((token.to_ascii_uppercase(), 1))
            } else {
                None
            }
        }
        "MSQ" => {
            let mut parts = Vec::new();
            let mut used = 0usize;
            while let Some(candidate) = tokens.get(index + used) {
                let candidate = candidate.trim();
                if !candidate.is_empty()
                    && candidate
                        .chars()
                        .all(|ch| ('A'..='D').contains(&ch) || ch == ',' || ch == ';')
                {
                    parts.push(candidate.to_owned());
                    used += 1;
                } else {
                    break;
                }
            }
            if parts.is_empty() {
                None
            } else {
                let mut values = parts
                    .join("")
                    .split([',', ';'])
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_ascii_uppercase())
                    .collect::<Vec<_>>();
                values.sort();
                values.dedup();
                if values.is_empty() {
                    None
                } else {
                    Some((values.join(","), used))
                }
            }
        }
        _ => {
            if index + 2 < tokens.len()
                && tokens[index + 1].eq_ignore_ascii_case("to")
                && parse_number(&tokens[index]).is_some()
                && parse_number(&tokens[index + 2]).is_some()
            {
                Some((
                    format!("{} to {}", tokens[index].trim(), tokens[index + 2].trim()),
                    3,
                ))
            } else if parse_number(token).is_some() {
                Some((token.to_owned(), 1))
            } else {
                None
            }
        }
    }
}

pub fn parse_answer_key(text: &str) -> Vec<ParsedKeyEntry> {
    let mut tokens = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("===== PAGE") || key_header(line) {
            continue;
        }
        tokens.extend(line.split_whitespace().map(str::to_owned));
    }
    let mut entries = BTreeMap::<i64, ParsedKeyEntry>::new();
    let mut index = 0usize;
    while index < tokens.len() {
        let Some((question_number, session_label, question_type, section)) =
            key_start(&tokens, index)
        else {
            index += 1;
            continue;
        };
        let Some((value, used)) = key_value(&tokens, index + 4, &question_type) else {
            index += 1;
            continue;
        };
        let marks_index = index + 4 + used;
        let Some(marks) = tokens
            .get(marks_index)
            .and_then(|token| parse_number(token))
        else {
            index += 1;
            continue;
        };
        if marks != 1.0 && marks != 2.0 {
            index += 1;
            continue;
        }
        let after = marks_index + 1;
        if after < tokens.len() && key_start(&tokens, after).is_none() {
            index += 1;
            continue;
        }
        entries.entry(question_number).or_insert(ParsedKeyEntry {
            question_number: question_number.to_string(),
            session_label,
            question_type_code: question_type,
            section_code: section,
            key_value: value.clone(),
            marks,
            key_status: if value.eq_ignore_ascii_case("MTA") {
                "mta".to_owned()
            } else {
                "answer".to_owned()
            },
        });
        index = after;
    }
    entries.into_values().collect()
}

fn unusual_char(ch: char) -> bool {
    let code = ch as u32;
    if code == 0x09 || code == 0x0A || code == 0x0D {
        return false;
    }
    if (0x20..=0x7E).contains(&code) {
        return false;
    }
    !matches!(
        code,
        0x00A0..=0x00FF
            | 0x0370..=0x03FF
            | 0x2010..=0x2027
            | 0x2030..=0x205E
            | 0x2070..=0x209F
            | 0x20A0..=0x20CF
            | 0x2100..=0x214F
            | 0x2190..=0x21FF
            | 0x2200..=0x22FF
            | 0x2300..=0x23FF
            | 0x25A0..=0x25FF
            | 0x2600..=0x26FF
            | 0x1D400..=0x1D7FF
            | 0xFB00..=0xFB06
    )
}

fn encoding_issue(text: &str) -> bool {
    if text.contains('\u{FFFD}')
        || text.contains("d?")
        || text.contains('Ã')
        || text.contains('Â')
        || text.contains("â€")
        || text.contains("ï¿½")
        || text.contains("??")
    {
        return true;
    }
    if text.chars().any(unusual_char) {
        return true;
    }
    let mut previous = '\0';
    for ch in text.chars() {
        if ch == '?' && (previous.is_ascii_digit() || previous == ' ') {
            return true;
        }
        if previous == '?' && ch.is_ascii_digit() {
            return true;
        }
        previous = ch;
    }
    false
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_page_text(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if paper_header(trimmed) {
            continue;
        }
        lines.push(trimmed.to_owned());
    }
    let mut result = lines.join("\n");
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }
    result.trim().to_owned()
}

fn page_texts(text: &str) -> Vec<(i64, String)> {
    let matches = page_re().find_iter(text).collect::<Vec<_>>();
    if matches.is_empty() {
        return vec![(1, text.to_owned())];
    }
    let mut pages = Vec::new();
    for (index, matched) in matches.iter().enumerate() {
        let start = matched.end();
        let end = matches
            .get(index + 1)
            .map(|next| next.start())
            .unwrap_or(text.len());
        let number = page_re()
            .captures(matched.as_str())
            .and_then(|captures| captures.get(1))
            .and_then(|value| value.as_str().parse::<i64>().ok())
            .unwrap_or(index as i64 + 1);
        let mut page = text[start..end].to_owned();
        if let Some(key_start) = page.find("Q. No. Session") {
            page.truncate(key_start);
        }
        pages.push((number, page));
    }
    pages
}

pub fn parse_question_paper(text: &str) -> Vec<ParsedQuestion> {
    let pages = page_texts(text);
    let mut flat = String::new();
    let mut page_ranges = Vec::new();
    for (number, page) in &pages {
        let start = flat.len();
        flat.push_str(page);
        page_ranges.push((start, flat.len(), *number));
    }
    let mut starts = BTreeMap::<i64, (usize, i64)>::new();
    for matched in question_re().find_iter(&flat) {
        let Some(captures) = question_re().captures(matched.as_str()) else {
            continue;
        };
        let number = captures
            .get(1)
            .and_then(|value| value.as_str().parse::<i64>().ok())
            .unwrap_or(0);
        if !(1..=65).contains(&number) {
            continue;
        }
        let line_start = flat[..matched.start()]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let line_end = flat[matched.end()..]
            .find('\n')
            .map(|index| matched.end() + index)
            .unwrap_or(flat.len());
        let line = flat[line_start..line_end].trim();
        if line.to_ascii_lowercase().contains("carry one mark")
            || line.to_ascii_lowercase().contains("carry two mark")
            || line.contains(" – Q.")
            || line.contains(" - Q.")
        {
            continue;
        }
        let Some((_, _, page_number)) = page_ranges
            .iter()
            .find(|(start, end, _)| *start <= matched.start() && matched.start() < *end)
        else {
            continue;
        };
        starts
            .entry(number)
            .or_insert((matched.start(), *page_number));
    }
    let ordered = starts
        .into_iter()
        .map(|(number, (start, page))| (number, start, page))
        .collect::<Vec<_>>();
    let mut questions = Vec::new();
    for (index, (number, start, page)) in ordered.iter().enumerate() {
        let end = ordered
            .get(index + 1)
            .map(|(_, next, _)| *next)
            .unwrap_or(flat.len());
        if *start >= end || *start >= flat.len() {
            continue;
        }
        let raw = clean_page_text(&flat[*start..end]);
        if raw.is_empty() {
            continue;
        }
        let options = option_re().find_iter(&raw).collect::<Vec<_>>();
        let (statement, parsed_options) = if options.is_empty() {
            (collapse_whitespace(&raw), Vec::new())
        } else {
            let statement = collapse_whitespace(&raw[..options[0].start()]);
            let mut parsed = Vec::new();
            let mut seen = HashSet::new();
            for (option_index, option) in options.iter().enumerate() {
                let key = option
                    .as_str()
                    .trim_matches(|ch| ch == '(' || ch == ')')
                    .to_ascii_uppercase();
                if !seen.insert(key.clone()) {
                    continue;
                }
                let option_end = options
                    .get(option_index + 1)
                    .map(|next| next.start())
                    .unwrap_or(raw.len());
                let text = collapse_whitespace(&raw[option.end()..option_end]);
                parsed.push(ParsedQuestionOption { key, text });
                if parsed.len() == 4 {
                    break;
                }
            }
            (statement, parsed)
        };
        let options_encoding = parsed_options
            .iter()
            .any(|option| encoding_issue(&option.text));
        let option_issues = parsed_options
            .iter()
            .flat_map(|option| {
                let mut issues = Vec::new();
                if option.text.is_empty() {
                    issues.push(format!("Option {} has no text.", option.key));
                }
                if option.text.chars().count() > 2_000 {
                    issues.push(format!("Option {} is too long.", option.key));
                }
                if encoding_issue(&option.text) {
                    issues.push(format!(
                        "Option {} contains garbled characters.",
                        option.key
                    ));
                }
                issues
            })
            .collect::<Vec<_>>();
        let statement_after_label = statement.chars().skip(20).collect::<String>();
        questions.push(ParsedQuestion {
            question_number: number.to_string(),
            page_number: *page,
            raw_text: raw.chars().take(50_000).collect(),
            figure_reference: figure_re().is_match(&statement),
            encoding_issue: encoding_issue(&statement) || options_encoding,
            short_statement: statement.chars().count() < 40,
            next_question_text: question_re().is_match(&statement_after_label),
            options: parsed_options,
            statement,
            option_issues,
        });
    }
    questions
}

pub fn review_reasons(question: &ParsedQuestion, key: Option<&ParsedKeyEntry>) -> Vec<String> {
    let mut reasons = Vec::new();
    if question.statement.trim().is_empty() {
        reasons.push("No question statement was extracted.".to_owned());
    }
    if question.short_statement {
        reasons.push("The extracted statement is too short.".to_owned());
    }
    if question.encoding_issue {
        reasons.push("The PDF text layer contains unsupported or garbled characters.".to_owned());
    }
    if question.figure_reference {
        reasons.push(
            "The question appears to depend on a figure, chart, table or diagram.".to_owned(),
        );
    }
    if question.next_question_text {
        reasons.push("The extraction may include text from the next question.".to_owned());
    }
    if !question.option_issues.is_empty() {
        reasons.extend(question.option_issues.iter().cloned());
    }
    let Some(key) = key else {
        reasons.push("No matching final answer-key entry was found.".to_owned());
        return reasons;
    };
    if key.key_status == "mta" {
        reasons.push("The official key is MTA (marks to all).".to_owned());
    }
    if key.marks != 1.0 && key.marks != 2.0 {
        reasons.push(format!("Unexpected official marks: {}.", key.marks));
    }
    if !matches!(key.section_code.as_str(), "GA" | "PH") {
        reasons.push(format!(
            "Unexpected official section: {}.",
            key.section_code
        ));
    }
    match key.question_type_code.as_str() {
        "MCQ" | "MSQ" => {
            if question.options.len() != 4 {
                reasons.push(format!(
                    "Expected four answer choices; found {}.",
                    question.options.len()
                ));
            }
            if key.key_status != "mta" {
                if key.question_type_code == "MCQ" && key.key_value.contains(',') {
                    reasons.push("An MCQ key contains more than one option.".to_owned());
                }
                let options = question
                    .options
                    .iter()
                    .map(|option| option.key.as_str())
                    .collect::<HashSet<_>>();
                for value in key.key_value.split(',').filter(|value| !value.is_empty()) {
                    if !options.contains(value) {
                        reasons.push(format!(
                            "Official key option {value} is not among the extracted choices."
                        ));
                    }
                }
            }
        }
        "NAT" => {
            if !question.options.is_empty() {
                reasons.push("A numerical question has extracted answer choices.".to_owned());
            }
            if key.key_status != "mta" {
                let parsed = if key.key_value.contains(" to ") {
                    let mut parts = key.key_value.split(" to ");
                    parse_number(parts.next().unwrap_or_default()).is_some()
                        && parse_number(parts.next().unwrap_or_default()).is_some()
                } else {
                    parse_number(&key.key_value).is_some()
                };
                if !parsed {
                    reasons.push("The numerical official key is not a number or range.".to_owned());
                }
            }
        }
        _ => {}
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_row_based_and_vertical_answer_keys() {
        let row = r#"
        Q. No. Session Question Type Section Key/Range Mark
        1 5 MCQ GA D 1
        2 5 MSQ PH A, C, D 1
        3 5 NAT PH 2.4 to 2.6 1
        4 5 NAT PH MTA 2
        "#;
        let entries = parse_answer_key(row);
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[1].key_value, "A,C,D");
        assert_eq!(entries[2].key_value, "2.4 to 2.6");
        assert_eq!(entries[3].key_status, "mta");

        let vertical = r#"
        Answer Key for Physics (PH)
        Q. No.
        Session
        Q. Type
        Section
        Key/Range
        Marks
        1
        7
        MCQ
        GA
        D
        1
        2
        7
        MSQ
        PH
        A;B;C
        2
        "#;
        let entries = parse_answer_key(vertical);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key_value, "D");
        assert_eq!(entries[1].key_value, "A,B,C");
    }

    #[test]
    fn parses_questions_with_pages_options_and_nat() {
        let text = r#"
===== PAGE 1 =====
GATE 2022 Physics (PH)
Q.1 Choose the correct option.
(A) Alpha
(B) Beta
(C) Gamma
(D) Delta
===== PAGE 2 =====
Q.2 A numerical question.
The value is ______.
"#;
        let questions = parse_question_paper(text);
        assert_eq!(questions.len(), 2);
        assert_eq!(questions[0].page_number, 1);
        assert_eq!(questions[0].options.len(), 4);
        assert_eq!(questions[0].options[1].key, "B");
        assert_eq!(questions[0].options[1].text, "Beta");
        assert_eq!(questions[1].page_number, 2);
        assert!(questions[1].options.is_empty());
        assert!(questions[1].statement.contains("numerical question"));
    }

    #[test]
    fn flags_figures_and_encoding_damage() {
        let question = ParsedQuestion {
            question_number: "1".to_owned(),
            page_number: 1,
            raw_text: String::new(),
            statement: "The above frequency chart shows 1.4 x 10?? m.".to_owned(),
            options: Vec::new(),
            figure_reference: true,
            encoding_issue: true,
            short_statement: false,
            next_question_text: false,
            option_issues: Vec::new(),
        };
        let key = ParsedKeyEntry {
            question_number: "1".to_owned(),
            session_label: "5".to_owned(),
            question_type_code: "MCQ".to_owned(),
            section_code: "PH".to_owned(),
            key_value: "A".to_owned(),
            marks: 1.0,
            key_status: "answer".to_owned(),
        };
        let reasons = review_reasons(&question, Some(&key));
        assert!(reasons.iter().any(|reason| reason.contains("figure")));
        assert!(reasons.iter().any(|reason| reason.contains("garbled")));
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains("four answer choices"))
        );
    }

    #[test]
    #[ignore = "manual verification against downloaded GATE PH text"]
    fn inspect_real_gate_ph_texts() {
        let directory = std::env::var("M12_TEXT_DIR").expect("M12_TEXT_DIR");
        for (paper, key, year) in [
            ("ph_2022.txt", "ph_2022_key.txt", 2022),
            ("ph_2023.txt", "ph_2023_key.txt", 2023),
            ("ph_2024_iisc_qp4.txt", "ph_2024_iisc_key.txt", 2024),
            ("ph_2025_a.txt", "ph_2025_key_a.txt", 2025),
            ("ph_2026.txt", "ph_2026_key.txt", 2026),
        ] {
            let paper_text = std::fs::read_to_string(std::path::Path::new(&directory).join(paper))
                .expect("paper text");
            let key_text = std::fs::read_to_string(std::path::Path::new(&directory).join(key))
                .expect("key text");
            let keys = parse_answer_key(&key_text);
            let questions = parse_question_paper(&paper_text);
            let key_map = keys
                .iter()
                .map(|entry| (entry.question_number.clone(), entry))
                .collect::<BTreeMap<_, _>>();
            let mut clean = 0;
            let mut reasons = BTreeMap::<String, i64>::new();
            for question in &questions {
                let key = key_map.get(&question.question_number).copied();
                let review = review_reasons(question, key);
                if review.is_empty() {
                    clean += 1;
                }
                for reason in review {
                    *reasons.entry(reason).or_default() += 1;
                }
            }
            println!(
                "{year}: questions={} keys={} clean={} reasons={reasons:?}",
                questions.len(),
                keys.len(),
                clean
            );
            let found = questions
                .iter()
                .map(|question| question.question_number.parse::<i64>().unwrap_or(0))
                .collect::<HashSet<_>>();
            let missing = (1..=65)
                .filter(|number| !found.contains(number))
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                println!("  missing question numbers: {missing:?}");
            }
        }
    }
}
