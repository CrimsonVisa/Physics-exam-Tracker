use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use chrono::{SecondsFormat, Utc};
use regex::Regex;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value};
use scraper::Html;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::error::AppError;

pub const RESOURCE_CATEGORIES: [&str; 10] = [
    "handwritten_notes",
    "classroom_notes",
    "assignments",
    "formula_sheets",
    "previous_year_papers",
    "important_topics",
    "recommended_books",
    "nptel_courses",
    "youtube_playlists",
    "other",
];

pub const RESOURCE_UNITS: [&str; 12] = [
    "General Aptitude",
    "Mathematical Methods of Physics",
    "Classical Mechanics",
    "Electromagnetic Theory",
    "Quantum Mechanics",
    "Thermodynamic and Statistical Physics",
    "Electronics and Experimental Methods",
    "Atomic & Molecular Physics",
    "Condensed Matter Physics",
    "Nuclear and Particle Physics",
    "All Units",
    "Unmapped",
];

const MAX_LOCAL_PDF_BYTES: u64 = 200 * 1024 * 1024;
const MAX_REMOTE_BYTES: usize = 5 * 1024 * 1024;
const REQUEST_INTERVAL: Duration = Duration::from_millis(1_250);

const NOTES_PAGE: &str = "https://www.csirnetphysics.in/p/hand-written-notes_28.html";
const QUESTIONS_PAGE: &str = "https://www.csirnetphysics.in/p/previous-questions-paper.html";
const IMPORTANT_TOPICS_PAGE: &str = "https://www.csirnetphysics.in/p/important-topics.html";
const BOOKS_PAGE: &str = "https://www.csirnetphysics.in/p/book.html";
const SYLLABUS_PAGE: &str = "https://www.csirnetphysics.in/p/syllabus.html";
const NPTEL_PAGE: &str = "https://www.csirnetphysics.in/p/nptel-video-link.html";

const SYLLABUS_LABELS: [&str; 6] = [
    "CSIR-NET/JRF and SET Exam 2nd Paper Syllabus Physics",
    "JEST Syllabus Physics",
    "GATE Syllabus Physics",
    "IIT-JAM Syllabus Physics",
    "TIFR Paper",
    "SET Exam 1st Paper Syllabus",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    Notes,
    Questions,
    Books,
    ImportantTopics,
    Syllabus,
    Nptel,
}

struct SourcePage {
    kind: SourceKind,
    url: &'static str,
}

const SOURCE_PAGES: &[SourcePage] = &[
    SourcePage {
        kind: SourceKind::Notes,
        url: NOTES_PAGE,
    },
    SourcePage {
        kind: SourceKind::Questions,
        url: QUESTIONS_PAGE,
    },
    SourcePage {
        kind: SourceKind::ImportantTopics,
        url: IMPORTANT_TOPICS_PAGE,
    },
    SourcePage {
        kind: SourceKind::Books,
        url: BOOKS_PAGE,
    },
    SourcePage {
        kind: SourceKind::Syllabus,
        url: SYLLABUS_PAGE,
    },
    SourcePage {
        kind: SourceKind::Nptel,
        url: NPTEL_PAGE,
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub id: i64,
    pub resource_key: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub unit: String,
    pub topic_code: Option<String>,
    pub source_page_url: Option<String>,
    pub original_url: String,
    pub external_id: Option<String>,
    pub external_kind: String,
    pub mime_type: Option<String>,
    pub availability: String,
    pub discovered_at: String,
    pub last_seen_at: Option<String>,
    pub last_verified_at: Option<String>,
    pub removed_at: Option<String>,
    pub study_status: String,
    pub personal_notes: String,
    pub last_opened_at: Option<String>,
    pub local_file: Option<LocalResourceFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalResourceFile {
    pub file_name: String,
    pub file_path: String,
    pub size_bytes: i64,
    pub sha256: String,
    pub imported_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryRunInfo {
    pub status: String,
    pub finished_at: Option<String>,
    pub pages_ok: i64,
    pub pages_failed: i64,
    pub resources_seen: i64,
    pub resources_added: i64,
    pub resources_updated: i64,
    pub resources_removed: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceOverview {
    pub total: i64,
    pub local_count: i64,
    pub online_count: i64,
    pub removed_count: i64,
    pub studying_count: i64,
    pub completed_count: i64,
    pub last_discovery: Option<DiscoveryRunInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverySummary {
    pub status: String,
    pub pages_ok: i64,
    pub pages_failed: i64,
    pub resources_seen: i64,
    pub resources_added: i64,
    pub resources_updated: i64,
    pub resources_removed: i64,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualResourceInput {
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub unit: String,
    pub topic_code: Option<String>,
    pub original_url: String,
    pub source_page_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceProgressInput {
    pub status: String,
    pub notes: String,
}

#[derive(Debug, Clone)]
struct DiscoveredResource {
    title: String,
    description: String,
    category: &'static str,
    unit: &'static str,
    source_page_url: &'static str,
    original_url: String,
    external_id: Option<String>,
    external_kind: &'static str,
}

#[derive(Debug, Clone)]
struct SourcePayload {
    source_page_url: &'static str,
    resources: Vec<DiscoveredResource>,
}

#[derive(Debug, Default)]
pub(crate) struct FetchedDiscovery {
    payloads: Vec<SourcePayload>,
    failures: Vec<String>,
    blocked: bool,
    pages_ok: i64,
    pages_failed: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FetchError {
    RateLimited,
    Http(u16),
    Network(String),
    TooLarge,
}

pub(crate) trait ResourceFetcher {
    fn get(&mut self, url: &str) -> Result<String, FetchError>;
}

pub(crate) struct HttpFetcher {
    client: reqwest::blocking::Client,
    last_request: Option<Instant>,
}

impl HttpFetcher {
    pub(crate) fn new() -> Result<Self, AppError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(25))
            .user_agent("CSIR-NET-Physics-Tracker/1.1 resource-refresh")
            .build()
            .map_err(|error| {
                AppError::resource_discovery_failed(format!(
                    "The resource downloader could not start: {error}"
                ))
            })?;
        Ok(Self {
            client,
            last_request: None,
        })
    }

    fn wait_for_rate_limit(&mut self) {
        if let Some(last) = self.last_request {
            let elapsed = last.elapsed();
            if elapsed < REQUEST_INTERVAL {
                std::thread::sleep(REQUEST_INTERVAL - elapsed);
            }
        }
        self.last_request = Some(Instant::now());
    }
}

impl ResourceFetcher for HttpFetcher {
    fn get(&mut self, url: &str) -> Result<String, FetchError> {
        self.wait_for_rate_limit();
        let response = self
            .client
            .get(url)
            .header(
                reqwest::header::ACCEPT,
                "text/html,application/json;q=0.9,*/*;q=0.8",
            )
            .send()
            .map_err(|error| FetchError::Network(error.to_string()))?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(FetchError::RateLimited);
        }
        if !response.status().is_success() {
            return Err(FetchError::Http(response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_REMOTE_BYTES as u64)
        {
            return Err(FetchError::TooLarge);
        }
        let body = response
            .text()
            .map_err(|error| FetchError::Network(error.to_string()))?;
        if body.len() > MAX_REMOTE_BYTES {
            return Err(FetchError::TooLarge);
        }
        Ok(body)
    }
}

#[cfg(test)]
pub(crate) fn refresh_catalogue<F: ResourceFetcher>(
    connection: &mut Connection,
    fetcher: &mut F,
    now: &str,
) -> Result<DiscoverySummary, AppError> {
    let fetched = fetch_sources(fetcher);
    apply_discovery(connection, fetched, now)
}

/// Performs network work without holding the SQLite connection. The caller
/// applies the result with `apply_fetched_discovery`.
pub(crate) fn fetch_catalogue<F: ResourceFetcher>(fetcher: &mut F) -> FetchedDiscovery {
    fetch_sources(fetcher)
}

pub(crate) fn apply_fetched_discovery(
    connection: &mut Connection,
    fetched: FetchedDiscovery,
    now: &str,
) -> Result<DiscoverySummary, AppError> {
    apply_discovery(connection, fetched, now)
}

fn fetch_sources<F: ResourceFetcher>(fetcher: &mut F) -> FetchedDiscovery {
    let mut result = FetchedDiscovery::default();

    for source in SOURCE_PAGES {
        if result.blocked {
            break;
        }

        let html = match fetcher.get(source.url) {
            Ok(html) => html,
            Err(FetchError::RateLimited) => {
                result.blocked = true;
                result.pages_failed += 1;
                result.failures.push(format!(
                    "{} returned HTTP 429. Refresh stopped without changing saved resources.",
                    source.url
                ));
                break;
            }
            Err(error) => {
                result.pages_failed += 1;
                result.failures.push(format!(
                    "{} could not be read: {}",
                    source.url,
                    fetch_error_label(&error)
                ));
                continue;
            }
        };

        let parsed = match source.kind {
            SourceKind::Notes | SourceKind::Questions | SourceKind::Books => {
                discover_json_page(fetcher, source, &html)
            }
            SourceKind::ImportantTopics => discover_important_topics(source, &html),
            SourceKind::Syllabus => discover_syllabus(source, &html),
            SourceKind::Nptel => discover_nptel(source, &html),
        };

        match parsed {
            Ok(Some(payload)) => {
                result.pages_ok += 1;
                result.payloads.push(payload);
            }
            Ok(None) => {
                result.pages_ok += 1;
            }
            Err(FetchError::RateLimited) => {
                result.blocked = true;
                result.pages_failed += 1;
                result.failures.push(format!(
                    "A resource feed for {} returned HTTP 429. Refresh stopped without changing saved resources.",
                    source.url
                ));
                break;
            }
            Err(error) => {
                result.pages_failed += 1;
                result.failures.push(format!(
                    "{} did not expose readable resource links: {}",
                    source.url,
                    fetch_error_label(&error)
                ));
            }
        }
    }

    result
}

fn fetch_error_label(error: &FetchError) -> String {
    match error {
        FetchError::RateLimited => "the site asked the app to slow down (HTTP 429)".to_owned(),
        FetchError::Http(code) => format!("HTTP {code}"),
        FetchError::Network(message) => message.clone(),
        FetchError::TooLarge => "the response exceeded the 5 MB safety limit".to_owned(),
    }
}

fn discover_json_page<F: ResourceFetcher>(
    fetcher: &mut F,
    source: &SourcePage,
    html: &str,
) -> Result<Option<SourcePayload>, FetchError> {
    let Some(endpoint) = extract_catalogue_endpoint(html) else {
        if let Some(payload) = discover_drive_anchors(source, html)? {
            return Ok(Some(payload));
        }
        return Err(FetchError::Network(
            "no embedded catalogue feed or Drive links were found".to_owned(),
        ));
    };

    let body = fetcher.get(&endpoint)?;
    let catalogue: AppsScriptCatalogue = serde_json::from_str(body.trim_start_matches('\u{feff}'))
        .map_err(|error| FetchError::Network(format!("invalid catalogue JSON: {error}")))?;
    let resources = parse_catalogue_json(source.kind, source.url, &catalogue);
    if resources.is_empty() {
        return Err(FetchError::Network(
            "the embedded catalogue contained no usable resources".to_owned(),
        ));
    }
    Ok(Some(SourcePayload {
        source_page_url: source.url,
        resources,
    }))
}

fn extract_catalogue_endpoint(html: &str) -> Option<String> {
    let pattern = Regex::new(r#"fetch\(\s*["']([^"']+)["']"#).ok()?;
    pattern
        .captures_iter(html)
        .filter_map(|capture| capture.get(1).map(|value| value.as_str().to_owned()))
        .find(|url| url.starts_with("https://script.google.com/"))
}

fn discover_drive_anchors(
    source: &SourcePage,
    html: &str,
) -> Result<Option<SourcePayload>, FetchError> {
    let document = Html::parse_document(html);
    let anchor_pattern = Regex::new(r#"(?is)<a\b[^>]*href\s*=\s*["']([^"']+)["'][^>]*>(.*?)</a>"#)
        .map_err(|error| FetchError::Network(error.to_string()))?;
    let tag_pattern = Regex::new(r"(?is)<[^>]+>").ok();
    let mut resources = Vec::new();
    let mut seen = HashSet::new();

    for capture in anchor_pattern.captures_iter(html) {
        let Some(href) = capture.get(1).map(|value| value.as_str().trim()) else {
            continue;
        };
        if !(href.contains("drive.google.com") || href.contains("docs.google.com")) {
            continue;
        }
        let title_html = capture.get(2).map(|value| value.as_str()).unwrap_or("");
        let title = tag_pattern
            .as_ref()
            .map(|pattern| pattern.replace_all(title_html, " ").to_string())
            .unwrap_or_else(|| title_html.to_owned());
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        if title.is_empty() || !seen.insert(href.to_owned()) {
            continue;
        }
        let (external_kind, external_id) = classify_url(href);
        resources.push(DiscoveredResource {
            title,
            description: String::new(),
            category: category_for_source(source.kind, ""),
            unit: "Unmapped",
            source_page_url: source.url,
            original_url: href.to_owned(),
            external_id,
            external_kind,
        });
    }

    let _ = document;
    if resources.is_empty() {
        Ok(None)
    } else {
        Ok(Some(SourcePayload {
            source_page_url: source.url,
            resources,
        }))
    }
}

type AppsScriptCatalogue = BTreeMap<String, BTreeMap<String, Vec<RawCatalogueItem>>>;

#[derive(Debug, Clone, Deserialize)]
struct RawCatalogueItem {
    #[serde(rename = "examName", default)]
    exam_name: String,
    #[serde(default)]
    subject: String,
    #[serde(rename = "qpUrl", default)]
    qp_url: String,
    #[serde(rename = "ansUrl", default)]
    ans_url: String,
}

fn parse_catalogue_json(
    kind: SourceKind,
    source_page_url: &'static str,
    catalogue: &AppsScriptCatalogue,
) -> Vec<DiscoveredResource> {
    let mut resources = Vec::new();
    let mut seen = HashSet::new();

    for (group, subgroups) in catalogue {
        for (subgroup, items) in subgroups {
            for item in items {
                let title = item.exam_name.trim();
                if title.is_empty() {
                    continue;
                }
                let description = if subgroup.trim().is_empty() {
                    group.clone()
                } else {
                    format!("{group} — {subgroup}")
                };

                if !item.qp_url.trim().is_empty() {
                    push_discovered(
                        &mut resources,
                        &mut seen,
                        kind,
                        source_page_url,
                        title.to_owned(),
                        description.clone(),
                        group,
                        &item.subject,
                        item.qp_url.trim(),
                    );
                }
                if !item.ans_url.trim().is_empty() {
                    push_discovered(
                        &mut resources,
                        &mut seen,
                        kind,
                        source_page_url,
                        format!("{title} — solutions"),
                        description.clone(),
                        group,
                        &item.subject,
                        item.ans_url.trim(),
                    );
                }
            }
        }
    }

    resources
}

#[allow(clippy::too_many_arguments)]
fn push_discovered(
    resources: &mut Vec<DiscoveredResource>,
    seen: &mut HashSet<(String, String)>,
    kind: SourceKind,
    source_page_url: &'static str,
    title: String,
    description: String,
    group: &str,
    subject: &str,
    original_url: &str,
) {
    let category = category_for_source(kind, group);
    if !seen.insert((category.to_owned(), original_url.to_owned())) {
        return;
    }
    let (external_kind, external_id) = classify_url(original_url);
    let mapping_text = format!("{group} {subject} {title}");
    let unit = unit_for_source(kind, group, &mapping_text);
    resources.push(DiscoveredResource {
        title,
        description,
        category,
        unit,
        source_page_url,
        original_url: original_url.to_owned(),
        external_id,
        external_kind,
    });
}

fn category_for_source(kind: SourceKind, group: &str) -> &'static str {
    let group = group.to_lowercase();
    match kind {
        SourceKind::Notes => {
            if group.contains("hand written") || group.contains("handwritten") {
                "handwritten_notes"
            } else if group.contains("formula") {
                "formula_sheets"
            } else if group.contains("assignment") {
                "assignments"
            } else {
                "classroom_notes"
            }
        }
        SourceKind::Questions => {
            if group.contains("cut-off") || group.contains("cut off") {
                "other"
            } else {
                "previous_year_papers"
            }
        }
        SourceKind::Books => "recommended_books",
        SourceKind::Nptel => "nptel_courses",
        SourceKind::ImportantTopics => "important_topics",
        SourceKind::Syllabus => "other",
    }
}

fn unit_for_source(kind: SourceKind, group: &str, text: &str) -> &'static str {
    if kind == SourceKind::Questions {
        let lower_group = group.to_lowercase();
        if lower_group.contains("set paper i") {
            return "General Aptitude";
        }
        let mapped = unit_for_text(text);
        return if mapped == "Unmapped" {
            "All Units"
        } else {
            mapped
        };
    }
    if kind == SourceKind::Books {
        return unit_for_text(group);
    }
    unit_for_text(text)
}

fn unit_for_text(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    let has = |needle: &str| lower.contains(needle);
    if has("general aptitude") || has("paper i") || has("part a") || has("reasoning") {
        "General Aptitude"
    } else if has("mathematical method") || has("mathematical physics") || has("mathematics") {
        "Mathematical Methods of Physics"
    } else if has("classical mechanics") || has("theoretical mechanics") || has("relativity") {
        "Classical Mechanics"
    } else if has("electromagnetic")
        || has("electrodynamics")
        || has("electromagnetism")
        || has("electrostatic")
        || has("optics")
    {
        "Electromagnetic Theory"
    } else if has("quantum") || has("harmonic oscillator") {
        "Quantum Mechanics"
    } else if has("thermodynamic")
        || has("statistical physics")
        || has("statistical mechanics")
        || has("statistics")
    {
        "Thermodynamic and Statistical Physics"
    } else if has("electronics") || has("experimental method") || has("numerical method") {
        "Electronics and Experimental Methods"
    } else if has("atomic") || has("molecular") || has("spectroscopy") {
        "Atomic & Molecular Physics"
    } else if has("condensed matter") || has("solid state") || has("solid-state") {
        "Condensed Matter Physics"
    } else if has("nuclear") || has("particle physics") || has("particle") {
        "Nuclear and Particle Physics"
    } else {
        "Unmapped"
    }
}

fn discover_important_topics(
    source: &SourcePage,
    html: &str,
) -> Result<Option<SourcePayload>, FetchError> {
    let document = Html::parse_document(html);
    let body_selector = scraper::Selector::parse(".post-body").map_err(|_| FetchError::TooLarge)?;
    let body_text = document
        .select(&body_selector)
        .next()
        .map(|body| collapse_whitespace(&body.text().collect::<Vec<_>>().join(" ")))
        .unwrap_or_default();
    let description = body_text
        .split(" For part A ")
        .next()
        .unwrap_or(&body_text)
        .chars()
        .take(500)
        .collect::<String>();
    let (external_kind, external_id) = classify_url(source.url);
    Ok(Some(SourcePayload {
        source_page_url: source.url,
        resources: vec![DiscoveredResource {
            title: "Important Topics for CSIR-NET / GATE / JEST / SET Physics".to_owned(),
            description,
            category: "important_topics",
            unit: "All Units",
            source_page_url: source.url,
            original_url: source.url.to_owned(),
            external_id,
            external_kind,
        }],
    }))
}

fn discover_syllabus(source: &SourcePage, html: &str) -> Result<Option<SourcePayload>, FetchError> {
    let document = Html::parse_document(html);
    let selector = scraper::Selector::parse("a[href]").map_err(|_| FetchError::TooLarge)?;
    let mut resources = Vec::new();
    let mut seen = HashSet::new();
    let mut index = 0usize;

    for anchor in document.select(&selector) {
        let Some(href) = anchor.value().attr("href") else {
            continue;
        };
        if !(href.contains("drive.google.com") || href.contains("docs.google.com")) {
            continue;
        }
        if !seen.insert(href.to_owned()) {
            continue;
        }
        let label = SYLLABUS_LABELS
            .get(index)
            .copied()
            .unwrap_or("Syllabus download");
        index += 1;
        let (external_kind, external_id) = classify_url(href);
        resources.push(DiscoveredResource {
            title: label.to_owned(),
            description: "Syllabus document linked from the CSIR-NET Physics syllabus page."
                .to_owned(),
            category: "other",
            unit: if label.contains("1st Paper") {
                "General Aptitude"
            } else {
                "All Units"
            },
            source_page_url: source.url,
            original_url: href.to_owned(),
            external_id,
            external_kind,
        });
    }

    if resources.is_empty() {
        Ok(None)
    } else {
        Ok(Some(SourcePayload {
            source_page_url: source.url,
            resources,
        }))
    }
}

fn discover_nptel(source: &SourcePage, html: &str) -> Result<Option<SourcePayload>, FetchError> {
    let document = Html::parse_document(html);
    let row_selector = scraper::Selector::parse("table tr").map_err(|_| FetchError::TooLarge)?;
    let cell_selector = scraper::Selector::parse("td").map_err(|_| FetchError::TooLarge)?;
    let anchor_selector = scraper::Selector::parse("a[href]").map_err(|_| FetchError::TooLarge)?;
    let mut resources = Vec::new();
    let mut seen = HashSet::new();

    for row in document.select(&row_selector) {
        let cells = row.select(&cell_selector).collect::<Vec<_>>();
        if cells.len() < 2 {
            continue;
        }
        let title = collapse_whitespace(&cells[0].text().collect::<Vec<_>>().join(" "));
        if title.is_empty() {
            continue;
        }
        let links = row
            .select(&anchor_selector)
            .filter_map(|anchor| anchor.value().attr("href"))
            .map(str::trim)
            .collect::<Vec<_>>();
        let nptel_url = links
            .iter()
            .copied()
            .find(|url| url.contains("nptel.ac.in"));
        let youtube_url = links
            .iter()
            .copied()
            .find(|url| url.contains("youtube.com") || url.contains("youtu.be"));

        if let Some(url) = nptel_url
            && seen.insert(("nptel_courses", url.to_owned()))
        {
            push_media_resource(
                source,
                &mut resources,
                &title,
                "nptel_courses",
                url,
                "NPTEL course linked from the CSIR-NET Physics video page.",
            );
        }
        if let Some(url) = youtube_url
            && seen.insert(("youtube_playlists", url.to_owned()))
        {
            push_media_resource(
                source,
                &mut resources,
                &title,
                "youtube_playlists",
                url,
                "YouTube playlist linked from the CSIR-NET Physics video page.",
            );
        }
    }

    if resources.is_empty() {
        Ok(None)
    } else {
        Ok(Some(SourcePayload {
            source_page_url: source.url,
            resources,
        }))
    }
}

fn push_media_resource(
    source: &SourcePage,
    resources: &mut Vec<DiscoveredResource>,
    title: &str,
    category: &'static str,
    original_url: &str,
    description: &str,
) {
    let (external_kind, external_id) = classify_url(original_url);
    resources.push(DiscoveredResource {
        title: title.to_owned(),
        description: description.to_owned(),
        category,
        unit: unit_for_text(title),
        source_page_url: source.url,
        original_url: original_url.to_owned(),
        external_id,
        external_kind,
    });
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn apply_discovery(
    connection: &mut Connection,
    fetched: FetchedDiscovery,
    now: &str,
) -> Result<DiscoverySummary, AppError> {
    let started_at = now.to_owned();
    let mut summary = DiscoverySummary {
        status: "failed".to_owned(),
        pages_ok: fetched.pages_ok,
        pages_failed: fetched.pages_failed,
        resources_seen: 0,
        resources_added: 0,
        resources_updated: 0,
        resources_removed: 0,
        message: String::new(),
    };

    for payload in fetched.payloads {
        if payload.resources.is_empty() {
            continue;
        }
        let transaction = connection.transaction().map_err(|error| {
            AppError::database("Resource discovery could not start a transaction", error)
        })?;
        let mut source_keys = Vec::new();
        for discovered in &payload.resources {
            let key = resource_key(discovered.category, &discovered.original_url);
            source_keys.push(key.clone());
            let topic = lookup_topic_code(&transaction, &discovered.title, discovered.unit)?;
            let topic_code = topic.as_ref().map(|(code, _)| code.clone());
            let unit = if matches!(discovered.unit, "All Units" | "Unmapped") {
                topic
                    .as_ref()
                    .map(|(_, section)| section.clone())
                    .unwrap_or_else(|| discovered.unit.to_owned())
            } else {
                discovered.unit.to_owned()
            };
            let existed: bool = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM study_resources WHERE resource_key = ?1)",
                    [&key],
                    |row| row.get(0),
                )
                .map_err(|error| {
                    AppError::database("Resource identity could not be checked", error)
                })?;
            transaction
                .execute(
                    "INSERT INTO study_resources (
                        resource_key, title, description, category, unit, topic_code,
                        source_page_url, original_url, external_id, external_kind,
                        mime_type, availability, discovered_at, last_seen_at,
                        last_verified_at, removed_at, created_at, updated_at
                     ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                        NULL, 'online', ?11, ?11, ?11, NULL, ?11, ?11
                     )
                     ON CONFLICT(resource_key) DO UPDATE SET
                        title = excluded.title,
                        description = excluded.description,
                        category = excluded.category,
                        unit = excluded.unit,
                        topic_code = excluded.topic_code,
                        source_page_url = excluded.source_page_url,
                        original_url = excluded.original_url,
                        external_id = excluded.external_id,
                        external_kind = excluded.external_kind,
                        availability = 'online',
                        last_seen_at = excluded.last_seen_at,
                        last_verified_at = excluded.last_verified_at,
                        removed_at = NULL,
                        updated_at = excluded.updated_at",
                    params![
                        key,
                        discovered.title,
                        discovered.description,
                        discovered.category,
                        unit,
                        topic_code,
                        discovered.source_page_url,
                        discovered.original_url,
                        discovered.external_id,
                        discovered.external_kind,
                        now,
                    ],
                )
                .map_err(|error| {
                    AppError::database("A discovered resource could not be saved", error)
                })?;
            if existed {
                summary.resources_updated += 1;
            } else {
                summary.resources_added += 1;
            }
            summary.resources_seen += 1;
        }

        // Mark rows that disappeared from a successfully parsed source, but
        // retain every row and its study history.
        let placeholders = (0..source_keys.len())
            .map(|index| format!("?{}", index + 3))
            .collect::<Vec<_>>()
            .join(", ");
        let mut values = Vec::with_capacity(source_keys.len() + 2);
        values.push(Value::Text(now.to_owned()));
        values.push(Value::Text(payload.source_page_url.to_owned()));
        values.extend(source_keys.into_iter().map(Value::Text));
        let removed = transaction
            .execute(
                &format!(
                    "UPDATE study_resources
                     SET availability = 'removed', removed_at = ?1, updated_at = ?1
                     WHERE source_page_url = ?2
                       AND availability != 'removed'
                       AND resource_key NOT IN ({placeholders})"
                ),
                params_from_iter(values),
            )
            .map_err(|error| AppError::database("Removed resources could not be marked", error))?;
        summary.resources_removed += removed as i64;
        transaction.commit().map_err(|error| {
            AppError::database("Resource discovery could not be committed", error)
        })?;
    }

    summary.status = if fetched.blocked {
        "blocked".to_owned()
    } else if fetched.pages_failed > 0 || summary.resources_seen == 0 {
        "partial".to_owned()
    } else {
        "success".to_owned()
    };
    if summary.status == "partial" && summary.resources_seen == 0 {
        summary.status = "failed".to_owned();
    }
    summary.message = if fetched.failures.is_empty() {
        format!(
            "Catalogue refreshed. {} resources seen, {} added, {} updated, {} marked removed.",
            summary.resources_seen,
            summary.resources_added,
            summary.resources_updated,
            summary.resources_removed
        )
    } else {
        format!(
            "{} resources seen; {} added; {} updated; {} marked removed. {} source issue(s): {}",
            summary.resources_seen,
            summary.resources_added,
            summary.resources_updated,
            summary.resources_removed,
            fetched.failures.len(),
            fetched.failures.join(" ")
        )
    };

    connection
        .execute(
            "INSERT INTO resource_discovery_runs (
                started_at, finished_at, status, pages_ok, pages_failed,
                resources_seen, resources_added, resources_updated,
                resources_removed, message
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                started_at,
                now,
                summary.status,
                summary.pages_ok,
                summary.pages_failed,
                summary.resources_seen,
                summary.resources_added,
                summary.resources_updated,
                summary.resources_removed,
                summary.message,
            ],
        )
        .map_err(|error| AppError::database("The discovery run could not be recorded", error))?;

    Ok(summary)
}

fn lookup_topic_code(
    connection: &Connection,
    title: &str,
    unit: &str,
) -> Result<Option<(String, String)>, AppError> {
    let normalized = normalize_match_text(title);
    if normalized.len() < 8 {
        return Ok(None);
    }
    let mut statement = connection
        .prepare(
            "SELECT t.code, t.title, s.title
             FROM syllabus_topics t
             JOIN syllabus_sections s ON s.id = t.section_id
             WHERE t.is_archived = 0",
        )
        .map_err(|error| AppError::database("Syllabus topics could not be read", error))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| AppError::database("Syllabus topics could not be read", error))?;
    for row in rows {
        let (code, topic_title, section_title) =
            row.map_err(|error| AppError::database("Syllabus topics could not be read", error))?;
        let normalized_topic = normalize_match_text(&topic_title);
        let exact = normalized_topic == normalized;
        let topic_phrase_in_title =
            normalized_topic.len() >= 10 && normalized.contains(&normalized_topic);
        let title_phrase_in_topic =
            normalized.len() >= 24 && normalized_topic.contains(&normalized);
        if exact || (section_title == unit && (topic_phrase_in_title || title_phrase_in_topic)) {
            return Ok(Some((code, section_title)));
        }
    }
    Ok(None)
}

fn normalize_match_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn resource_key(category: &str, original_url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(category.as_bytes());
    hasher.update(b"\0");
    hasher.update(original_url.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn classify_url(value: &str) -> (&'static str, Option<String>) {
    let Ok(url) = Url::parse(value) else {
        return ("web_page", None);
    };
    let host = url.host_str().unwrap_or_default().to_lowercase();
    if host.ends_with("drive.google.com") || host.ends_with("docs.google.com") {
        let path = url.path();
        if let Some(id) = extract_after(path, "/file/d/") {
            return ("drive_file", Some(id));
        }
        if let Some(id) = extract_after(path, "/drive/folders/") {
            return ("drive_folder", Some(id));
        }
        if let Some(id) = url
            .query_pairs()
            .find(|(key, _)| key == "id")
            .map(|(_, value)| value.into_owned())
        {
            return ("drive_file", Some(id));
        }
        return ("drive_file", None);
    }
    if host.ends_with("nptel.ac.in") {
        return ("nptel_course", None);
    }
    if host.ends_with("youtube.com") || host.ends_with("youtu.be") {
        let id = url
            .query_pairs()
            .find(|(key, _)| key == "list")
            .map(|(_, value)| value.into_owned())
            .or_else(|| {
                url.query_pairs()
                    .find(|(key, _)| key == "v")
                    .map(|(_, value)| value.into_owned())
            });
        return ("youtube_playlist", id);
    }
    ("web_page", None)
}

fn extract_after(path: &str, marker: &str) -> Option<String> {
    let start = path.find(marker)? + marker.len();
    let value = path[start..]
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn validate_http_url(field: &'static str, value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    let parsed = Url::parse(trimmed)
        .map_err(|_| AppError::validation(field, "Enter a complete http:// or https:// URL."))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(AppError::validation(
            field,
            "Enter a complete http:// or https:// URL.",
        ));
    }
    Ok(trimmed.to_owned())
}

fn is_valid_category(value: &str) -> bool {
    RESOURCE_CATEGORIES.contains(&value)
}

fn is_valid_unit(value: &str) -> bool {
    RESOURCE_UNITS.contains(&value)
}

fn is_valid_study_status(value: &str) -> bool {
    matches!(value, "not_started" | "studying" | "completed" | "skipped")
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn list_resources(connection: &Connection) -> Result<Vec<Resource>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                r.id, r.resource_key, r.title, r.description, r.category, r.unit,
                r.topic_code, r.source_page_url, r.original_url, r.external_id,
                r.external_kind, r.mime_type, r.availability, r.discovered_at,
                r.last_seen_at, r.last_verified_at, r.removed_at,
                COALESCE(p.study_status, 'not_started'),
                COALESCE(p.personal_notes, ''),
                p.last_opened_at,
                l.file_name, l.file_path, l.size_bytes, l.sha256, l.imported_at
             FROM study_resources r
             LEFT JOIN resource_progress p ON p.resource_id = r.id
             LEFT JOIN resource_local_files l ON l.resource_id = r.id
             ORDER BY r.category, r.unit, r.title COLLATE NOCASE",
        )
        .map_err(|error| AppError::database("Resources could not be prepared", error))?;
    let rows = statement
        .query_map([], |row| {
            let local_file = row
                .get::<_, Option<String>>(20)?
                .map(|file_name| LocalResourceFile {
                    file_name,
                    file_path: row.get::<_, String>(21).unwrap_or_default(),
                    size_bytes: row.get::<_, i64>(22).unwrap_or(0),
                    sha256: row.get::<_, String>(23).unwrap_or_default(),
                    imported_at: row.get::<_, String>(24).unwrap_or_default(),
                });
            Ok(Resource {
                id: row.get(0)?,
                resource_key: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                category: row.get(4)?,
                unit: row.get(5)?,
                topic_code: row.get(6)?,
                source_page_url: row.get(7)?,
                original_url: row.get(8)?,
                external_id: row.get(9)?,
                external_kind: row.get(10)?,
                mime_type: row.get(11)?,
                availability: row.get(12)?,
                discovered_at: row.get(13)?,
                last_seen_at: row.get(14)?,
                last_verified_at: row.get(15)?,
                removed_at: row.get(16)?,
                study_status: row.get(17)?,
                personal_notes: row.get(18)?,
                last_opened_at: row.get(19)?,
                local_file,
            })
        })
        .map_err(|error| AppError::database("Resources could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Resources could not be read", error))
}

pub fn overview(connection: &Connection) -> Result<ResourceOverview, AppError> {
    let (total, local_count, online_count, removed_count, studying_count, completed_count) =
        connection
            .query_row(
                "SELECT
                    (SELECT count(*) FROM study_resources),
                    (SELECT count(*) FROM resource_local_files),
                    (SELECT count(*) FROM study_resources WHERE availability = 'online'),
                    (SELECT count(*) FROM study_resources WHERE availability = 'removed'),
                    (SELECT count(*) FROM resource_progress WHERE study_status = 'studying'),
                    (SELECT count(*) FROM resource_progress WHERE study_status = 'completed')",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .map_err(|error| AppError::database("Resource summary could not be read", error))?;
    let last_discovery = connection
        .query_row(
            "SELECT status, finished_at, pages_ok, pages_failed, resources_seen,
                    resources_added, resources_updated, resources_removed, message
             FROM resource_discovery_runs
             ORDER BY id DESC LIMIT 1",
            [],
            |row| {
                Ok(DiscoveryRunInfo {
                    status: row.get(0)?,
                    finished_at: row.get(1)?,
                    pages_ok: row.get(2)?,
                    pages_failed: row.get(3)?,
                    resources_seen: row.get(4)?,
                    resources_added: row.get(5)?,
                    resources_updated: row.get(6)?,
                    resources_removed: row.get(7)?,
                    message: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Discovery history could not be read", error))?;
    Ok(ResourceOverview {
        total,
        local_count,
        online_count,
        removed_count,
        studying_count,
        completed_count,
        last_discovery,
    })
}

pub fn create_manual(
    connection: &Connection,
    input: ManualResourceInput,
) -> Result<Resource, AppError> {
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 300 {
        return Err(AppError::validation(
            "title",
            "Enter a resource title of 300 characters or fewer.",
        ));
    }
    let description = input.description.unwrap_or_default();
    if description.chars().count() > 2000 {
        return Err(AppError::validation(
            "description",
            "Description must be 2000 characters or fewer.",
        ));
    }
    if !is_valid_category(&input.category) {
        return Err(AppError::validation(
            "category",
            "Choose a valid resource type.",
        ));
    }
    if !is_valid_unit(&input.unit) {
        return Err(AppError::validation("unit", "Choose a syllabus unit."));
    }
    let original_url = validate_http_url("originalUrl", &input.original_url)?;
    let source_page_url = input
        .source_page_url
        .as_deref()
        .map(|value| validate_http_url("sourcePageUrl", value))
        .transpose()?;
    let topic_code = input
        .topic_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if let Some(code) = &topic_code {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM syllabus_topics WHERE code = ?1 AND is_archived = 0)",
                [code],
                |row| row.get(0),
            )
            .map_err(|error| {
                AppError::database("The syllabus topic could not be checked", error)
            })?;
        if !exists {
            return Err(AppError::validation(
                "topicCode",
                "Choose a topic code from the active syllabus.",
            ));
        }
    }
    let key = resource_key(&input.category, &original_url);
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM study_resources WHERE resource_key = ?1)",
            [&key],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The resource link could not be checked", error))?;
    if exists {
        return Err(AppError::validation(
            "originalUrl",
            "This link is already in the resource catalogue.",
        ));
    }
    let (external_kind, external_id) = classify_url(&original_url);
    let now = now_rfc3339();
    connection
        .execute(
            "INSERT INTO study_resources (
                resource_key, title, description, category, unit, topic_code,
                source_page_url, original_url, external_id, external_kind, mime_type,
                availability, discovered_at, last_seen_at, last_verified_at,
                removed_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL,
                'unverified', ?11, NULL, NULL, NULL, ?11, ?11)",
            params![
                key,
                title,
                description.trim(),
                input.category,
                input.unit,
                topic_code,
                source_page_url,
                original_url,
                external_id,
                external_kind,
                now,
            ],
        )
        .map_err(|error| AppError::database("The manual resource could not be saved", error))?;
    let id = connection.last_insert_rowid();
    get_resource(connection, id)?.ok_or_else(AppError::resource_not_found)
}

pub fn get_resource(connection: &Connection, id: i64) -> Result<Option<Resource>, AppError> {
    Ok(list_resources(connection)?
        .into_iter()
        .find(|resource| resource.id == id))
}

pub fn update_progress(
    connection: &Connection,
    resource_id: i64,
    input: ResourceProgressInput,
    now: &str,
) -> Result<Resource, AppError> {
    if !is_valid_study_status(&input.status) {
        return Err(AppError::validation(
            "status",
            "Choose Not started, Studying, Completed, or Skipped.",
        ));
    }
    if input.notes.chars().count() > 4000 {
        return Err(AppError::validation(
            "notes",
            "Personal notes must be 4000 characters or fewer.",
        ));
    }
    ensure_resource_exists(connection, resource_id)?;
    connection
        .execute(
            "INSERT INTO resource_progress (
                resource_id, study_status, personal_notes, last_opened_at, updated_at
             ) VALUES (?1, ?2, ?3, NULL, ?4)
             ON CONFLICT(resource_id) DO UPDATE SET
                study_status = excluded.study_status,
                personal_notes = excluded.personal_notes,
                updated_at = excluded.updated_at",
            params![resource_id, input.status, input.notes.trim(), now],
        )
        .map_err(|error| AppError::database("Resource study status could not be saved", error))?;
    get_resource(connection, resource_id)?.ok_or_else(AppError::resource_not_found)
}

pub fn mark_opened(connection: &Connection, resource_id: i64, now: &str) -> Result<(), AppError> {
    ensure_resource_exists(connection, resource_id)?;
    connection
        .execute(
            "INSERT INTO resource_progress (
                resource_id, study_status, personal_notes, last_opened_at, updated_at
             ) VALUES (?1, 'not_started', '', ?2, ?2)
             ON CONFLICT(resource_id) DO UPDATE SET
                last_opened_at = excluded.last_opened_at,
                updated_at = excluded.updated_at",
            params![resource_id, now],
        )
        .map_err(|error| AppError::database("The last-opened time could not be saved", error))?;
    Ok(())
}

fn ensure_resource_exists(connection: &Connection, resource_id: i64) -> Result<(), AppError> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM study_resources WHERE id = ?1)",
            [resource_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The resource could not be checked", error))?;
    if exists {
        Ok(())
    } else {
        Err(AppError::resource_not_found())
    }
}

pub fn import_local_pdf(
    connection: &mut Connection,
    resource_id: i64,
    source_path: &Path,
    destination_dir: &Path,
    now: &str,
) -> Result<LocalResourceFile, AppError> {
    ensure_resource_exists(connection, resource_id)?;
    let existing: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM resource_local_files WHERE resource_id = ?1)",
            [resource_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The local copy could not be checked", error))?;
    if existing {
        return Err(AppError::validation(
            "localFile",
            "Remove the existing local copy before importing another PDF.",
        ));
    }
    if !source_path.is_file() {
        return Err(AppError::validation(
            "sourcePath",
            "Choose an existing PDF file.",
        ));
    }
    if !destination_dir.is_dir() {
        return Err(AppError::validation(
            "destinationDir",
            "Choose an existing folder for the local PDF library.",
        ));
    }
    let metadata = fs::metadata(source_path).map_err(|error| {
        AppError::file_operation("The selected PDF could not be inspected", error)
    })?;
    if metadata.len() == 0 || metadata.len() > MAX_LOCAL_PDF_BYTES {
        return Err(AppError::validation(
            "sourcePath",
            "Choose a PDF between 1 byte and 200 MB.",
        ));
    }
    let mut source = File::open(source_path)
        .map_err(|error| AppError::file_operation("The selected PDF could not be opened", error))?;
    let mut header = [0_u8; 5];
    source
        .read_exact(&mut header)
        .map_err(|_| AppError::validation("sourcePath", "This file is not a PDF."))?;
    if &header != b"%PDF-" {
        return Err(AppError::validation(
            "sourcePath",
            "This file is not a PDF.",
        ));
    }
    source
        .rewind()
        .map_err(|error| AppError::file_operation("The PDF could not be read", error))?;

    let resource =
        get_resource(connection, resource_id)?.ok_or_else(AppError::resource_not_found)?;
    let stem = sanitize_file_stem(&resource.title);
    let mut candidate = destination_dir.join(format!("{stem}.pdf"));
    let mut sequence = 2_u32;
    while candidate.exists() {
        candidate = destination_dir.join(format!("{stem}-{sequence}.pdf"));
        sequence += 1;
        if sequence > 9999 {
            return Err(AppError::file_operation(
                "No unique local PDF filename could be created",
                "too many filename collisions",
            ));
        }
    }

    let mut output = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&candidate)
    {
        Ok(file) => file,
        Err(error) => {
            return Err(AppError::file_operation(
                "The local PDF could not be created",
                error,
            ));
        }
    };
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut copied = 0_u64;
    loop {
        let read = match source.read(&mut buffer) {
            Ok(read) => read,
            Err(error) => {
                drop(output);
                let _ = fs::remove_file(&candidate);
                return Err(AppError::file_operation(
                    "The selected PDF could not be read",
                    error,
                ));
            }
        };
        if read == 0 {
            break;
        }
        copied += read as u64;
        if copied > MAX_LOCAL_PDF_BYTES {
            drop(output);
            let _ = fs::remove_file(&candidate);
            return Err(AppError::validation(
                "sourcePath",
                "Choose a PDF no larger than 200 MB.",
            ));
        }
        hasher.update(&buffer[..read]);
        if let Err(error) = output.write_all(&buffer[..read]) {
            drop(output);
            let _ = fs::remove_file(&candidate);
            return Err(AppError::file_operation(
                "The local PDF could not be written",
                error,
            ));
        }
    }
    if let Err(error) = output.flush() {
        drop(output);
        let _ = fs::remove_file(&candidate);
        return Err(AppError::file_operation(
            "The local PDF could not be flushed",
            error,
        ));
    }
    drop(output);

    let sha256 = format!("{:x}", hasher.finalize());
    let size_bytes = copied as i64;
    let file_name = candidate
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("resource.pdf")
        .to_owned();
    let file_path = candidate.to_string_lossy().to_string();

    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The local PDF record could not be started", error))?;
    if let Err(error) = transaction.execute(
        "INSERT INTO resource_local_files (
            resource_id, file_name, file_path, sha256, size_bytes, imported_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![resource_id, file_name, file_path, sha256, size_bytes, now],
    ) {
        let _ = fs::remove_file(&candidate);
        return Err(AppError::database(
            "The local PDF could not be recorded",
            error,
        ));
    }
    if let Err(error) = transaction.commit() {
        let _ = fs::remove_file(&candidate);
        return Err(AppError::database(
            "The local PDF could not be recorded",
            error,
        ));
    }

    Ok(LocalResourceFile {
        file_name,
        file_path,
        size_bytes,
        sha256,
        imported_at: now.to_owned(),
    })
}

pub fn remove_local_pdf(
    connection: &mut Connection,
    resource_id: i64,
    _now: &str,
) -> Result<(), AppError> {
    ensure_resource_exists(connection, resource_id)?;
    let path: Option<String> = connection
        .query_row(
            "SELECT file_path FROM resource_local_files WHERE resource_id = ?1",
            [resource_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("The local PDF could not be read", error))?;
    if let Some(path) = path {
        let file_path = PathBuf::from(&path);
        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|error| {
                AppError::file_operation("The local PDF could not be removed", error)
            })?;
        }
    }
    connection
        .execute(
            "DELETE FROM resource_local_files WHERE resource_id = ?1",
            [resource_id],
        )
        .map_err(|error| AppError::database("The local PDF record could not be removed", error))?;
    Ok(())
}

fn sanitize_file_stem(title: &str) -> String {
    let mut value = title
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while value.contains("--") {
        value = value.replace("--", "-");
    }
    let value = value.trim_matches('-').to_owned();
    let value = if value.is_empty() {
        "resource".to_owned()
    } else {
        value
    };
    value.chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, syllabus};
    use chrono::NaiveDate;
    use std::collections::HashMap;

    const NOW: &str = "2026-10-30T12:00:00.000Z";

    struct FakeFetcher {
        responses: HashMap<String, Result<String, FetchError>>,
        calls: Vec<String>,
    }

    impl FakeFetcher {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
                calls: Vec::new(),
            }
        }

        fn with(mut self, url: &str, body: &str) -> Self {
            self.responses.insert(url.to_owned(), Ok(body.to_owned()));
            self
        }

        fn failing(mut self, url: &str, error: FetchError) -> Self {
            self.responses.insert(url.to_owned(), Err(error));
            self
        }
    }

    impl ResourceFetcher for FakeFetcher {
        fn get(&mut self, url: &str) -> Result<String, FetchError> {
            self.calls.push(url.to_owned());
            self.responses.get(url).cloned().unwrap_or_else(|| {
                Err(FetchError::Network(format!(
                    "unexpected URL in test: {url}"
                )))
            })
        }
    }

    fn fixture(name: &str) -> &'static str {
        match name {
            "notes-page.html" => include_str!("../tests/fixtures/resources/notes-page.html"),
            "notes.json" => include_str!("../tests/fixtures/resources/notes.json"),
            "questions-page.html" => {
                include_str!("../tests/fixtures/resources/questions-page.html")
            }
            "questions.json" => include_str!("../tests/fixtures/resources/questions.json"),
            "books-page.html" => include_str!("../tests/fixtures/resources/books-page.html"),
            "books.json" => include_str!("../tests/fixtures/resources/books.json"),
            "important-topics-page.html" => {
                include_str!("../tests/fixtures/resources/important-topics-page.html")
            }
            "syllabus-page.html" => include_str!("../tests/fixtures/resources/syllabus-page.html"),
            "nptel-page.html" => include_str!("../tests/fixtures/resources/nptel-page.html"),
            "malformed-page.html" => {
                "<html><body><div class=\"post-body\"><script>fetch(</script><a href=\"javascript:bad\">Bad</a></div></body></html>"
            }
            other => panic!("unknown fixture: {other}"),
        }
    }

    fn fixture_fetcher() -> FakeFetcher {
        FakeFetcher::new()
            .with(NOTES_PAGE, fixture("notes-page.html"))
            .with(
                "https://script.google.com/macros/s/fixture-notes/exec",
                fixture("notes.json"),
            )
            .with(QUESTIONS_PAGE, fixture("questions-page.html"))
            .with(
                "https://script.google.com/macros/s/fixture-questions/exec",
                fixture("questions.json"),
            )
            .with(IMPORTANT_TOPICS_PAGE, fixture("important-topics-page.html"))
            .with(BOOKS_PAGE, fixture("books-page.html"))
            .with(
                "https://script.google.com/macros/s/fixture-books/exec",
                fixture("books.json"),
            )
            .with(SYLLABUS_PAGE, fixture("syllabus-page.html"))
            .with(NPTEL_PAGE, fixture("nptel-page.html"))
    }

    fn test_database() -> Connection {
        let mut connection =
            db::test_connection_with_defaults(NaiveDate::from_ymd_opt(2026, 10, 30).expect("date"));
        syllabus::seed(&mut connection).expect("syllabus should seed");
        connection
    }

    fn manual_resource(connection: &Connection, url: &str) -> Resource {
        create_manual(
            connection,
            ManualResourceInput {
                title: "Manual resource".to_owned(),
                description: Some("Added by the user.".to_owned()),
                category: "other".to_owned(),
                unit: "Unmapped".to_owned(),
                topic_code: None,
                original_url: url.to_owned(),
                source_page_url: None,
            },
        )
        .expect("manual resource should save")
    }

    #[test]
    fn saved_html_fixtures_discover_resources_without_duplicates() {
        let mut connection = test_database();
        let mut fetcher = fixture_fetcher();
        let fetched = fetch_sources(&mut fetcher);
        assert_eq!(fetched.pages_ok, 6, "failures: {:?}", fetched.failures);
        assert_eq!(fetched.pages_failed, 0);
        assert_eq!(fetched.payloads.len(), 6);

        let summary =
            apply_discovery(&mut connection, fetched, NOW).expect("discovery should apply");
        assert_eq!(summary.status, "success");
        assert_eq!(summary.resources_seen, 17);
        assert_eq!(summary.resources_added, 17);

        let resources = list_resources(&connection).expect("resources should list");
        assert_eq!(resources.len(), 17);
        let categories = resources
            .iter()
            .map(|resource| resource.category.as_str())
            .collect::<HashSet<_>>();
        assert!(categories.contains("handwritten_notes"));
        assert!(categories.contains("formula_sheets"));
        assert!(categories.contains("assignments"));
        assert!(categories.contains("previous_year_papers"));
        assert!(categories.contains("recommended_books"));
        assert!(categories.contains("important_topics"));
        assert!(categories.contains("nptel_courses"));
        assert!(categories.contains("youtube_playlists"));
        assert_eq!(
            resources
                .iter()
                .filter(|resource| resource.original_url.contains("noteMaths123"))
                .count(),
            1
        );
        let quantum = resources
            .iter()
            .find(|resource| resource.title == "Quantum Mechanics Short Notes")
            .expect("quantum notes should exist");
        assert_eq!(quantum.unit, "Quantum Mechanics");
        assert_eq!(quantum.availability, "online");
        let set_paper_one = resources
            .iter()
            .find(|resource| resource.title == "SET Paper I December 2019")
            .expect("SET paper one should exist");
        assert_eq!(set_paper_one.unit, "General Aptitude");
        let drive = resources
            .iter()
            .find(|resource| resource.external_id.as_deref() == Some("noteQuantum456"))
            .expect("Drive ID should parse");
        assert_eq!(drive.external_kind, "drive_file");
        assert_eq!(drive.study_status, "not_started");

        let overview = overview(&connection).expect("overview should load");
        assert_eq!(overview.total, 17);
        assert_eq!(overview.online_count, 17);
        assert_eq!(overview.last_discovery.expect("run").resources_added, 17);
    }

    #[test]
    fn rate_limit_blocks_refresh_and_preserves_existing_resources() {
        let mut connection = test_database();
        let resource = manual_resource(&connection, "https://example.com/manual.pdf");
        let mut fetcher = FakeFetcher::new().failing(NOTES_PAGE, FetchError::RateLimited);

        let summary =
            refresh_catalogue(&mut connection, &mut fetcher, NOW).expect("refresh should report");
        assert_eq!(summary.status, "blocked");
        assert!(summary.message.contains("HTTP 429"));
        assert_eq!(fetcher.calls, vec![NOTES_PAGE.to_owned()]);

        let resources = list_resources(&connection).expect("resources should list");
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].id, resource.id);
        assert_eq!(resources[0].availability, "unverified");
        let runs: i64 = connection
            .query_row("SELECT count(*) FROM resource_discovery_runs", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(runs, 1);
    }

    #[test]
    fn malformed_html_is_reported_without_deleting_saved_resources() {
        let mut connection = test_database();
        manual_resource(&connection, "https://example.com/kept.pdf");
        let mut fetcher = FakeFetcher::new()
            .with(NOTES_PAGE, fixture("malformed-page.html"))
            .failing(QUESTIONS_PAGE, FetchError::Network("offline".to_owned()));
        let summary =
            refresh_catalogue(&mut connection, &mut fetcher, NOW).expect("refresh should report");
        assert_eq!(summary.status, "failed");
        assert!(
            summary
                .message
                .contains("did not expose readable resource links")
        );
        assert_eq!(list_resources(&connection).unwrap().len(), 1);
    }

    #[test]
    fn google_drive_and_media_urls_are_classified_conservatively() {
        assert_eq!(
            classify_url("https://drive.google.com/file/d/abcDEF_123/view?usp=sharing"),
            ("drive_file", Some("abcDEF_123".to_owned()))
        );
        assert_eq!(
            classify_url("https://drive.google.com/drive/folders/folder123?usp=sharing"),
            ("drive_folder", Some("folder123".to_owned()))
        );
        assert_eq!(
            classify_url("https://drive.google.com/open?id=query123&usp=drive_copy"),
            ("drive_file", Some("query123".to_owned()))
        );
        assert_eq!(
            classify_url("https://nptel.ac.in/courses/115/106/115106086/"),
            ("nptel_course", None)
        );
        assert_eq!(
            classify_url("https://www.youtube.com/watch?v=video&list=playlist123"),
            ("youtube_playlist", Some("playlist123".to_owned()))
        );
        assert_eq!(
            classify_url("https://example.com/notes.pdf"),
            ("web_page", None)
        );
        assert_eq!(
            unit_for_text("Solid State Physics"),
            "Condensed Matter Physics"
        );
        assert_eq!(unit_for_text("Unknown Topic"), "Unmapped");
        assert_eq!(unit_for_text("General Aptitude"), "General Aptitude");

        let connection = test_database();
        assert_eq!(
            lookup_topic_code(&connection, "Variational method", "Quantum Mechanics")
                .expect("topic lookup"),
            Some(("QM-C14".to_owned(), "Quantum Mechanics".to_owned()))
        );
        assert_eq!(
            lookup_topic_code(&connection, "Quantum Mechanics", "Quantum Mechanics")
                .expect("topic lookup"),
            None
        );
        assert_eq!(
            lookup_topic_code(
                &connection,
                "Relativistic Quantum Mechanics",
                "Classical Mechanics"
            )
            .expect("unit-crossing lookup"),
            None
        );
    }

    #[test]
    fn manual_resources_validate_links_and_persist_study_progress() {
        let connection = test_database();
        let error = create_manual(
            &connection,
            ManualResourceInput {
                title: "Bad".to_owned(),
                description: None,
                category: "other".to_owned(),
                unit: "Unmapped".to_owned(),
                topic_code: None,
                original_url: "javascript:alert(1)".to_owned(),
                source_page_url: None,
            },
        )
        .expect_err("bad URL should fail");
        assert_eq!(error.field, Some("originalUrl"));

        let resource = manual_resource(
            &connection,
            "https://drive.google.com/file/d/manualFile/view?usp=sharing",
        );
        assert_eq!(resource.external_id.as_deref(), Some("manualFile"));
        assert_eq!(resource.availability, "unverified");

        let duplicate = create_manual(
            &connection,
            ManualResourceInput {
                title: "Duplicate".to_owned(),
                description: None,
                category: "other".to_owned(),
                unit: "Unmapped".to_owned(),
                topic_code: None,
                original_url: "https://drive.google.com/file/d/manualFile/view?usp=sharing"
                    .to_owned(),
                source_page_url: None,
            },
        )
        .expect_err("duplicate should fail");
        assert_eq!(duplicate.field, Some("originalUrl"));

        let updated = update_progress(
            &connection,
            resource.id,
            ResourceProgressInput {
                status: "studying".to_owned(),
                notes: "Read the first two chapters.".to_owned(),
            },
            NOW,
        )
        .expect("progress should save");
        assert_eq!(updated.study_status, "studying");
        assert_eq!(updated.personal_notes, "Read the first two chapters.");
        mark_opened(&connection, resource.id, NOW).expect("opening should save");
        let reopened = get_resource(&connection, resource.id)
            .expect("query")
            .expect("resource");
        assert_eq!(reopened.study_status, "studying");
        assert_eq!(reopened.last_opened_at.as_deref(), Some(NOW));
    }

    #[test]
    fn local_pdf_import_and_removal_keep_the_catalogue_entry() {
        let mut connection = test_database();
        let resource = manual_resource(&connection, "https://example.com/manual.pdf");
        let source_dir = tempfile::tempdir().expect("source temp");
        let destination = tempfile::tempdir().expect("destination temp");
        let source = source_dir.path().join("authorized.pdf");
        fs::write(&source, b"%PDF-1.4\ntest pdf content\n").expect("pdf fixture");
        let text_file = source_dir.path().join("not-a-pdf.txt");
        fs::write(&text_file, b"not a pdf").expect("text fixture");

        let invalid = import_local_pdf(
            &mut connection,
            resource.id,
            &text_file,
            destination.path(),
            NOW,
        )
        .expect_err("non-PDF should fail");
        assert_eq!(invalid.field, Some("sourcePath"));

        let imported = import_local_pdf(
            &mut connection,
            resource.id,
            &source,
            destination.path(),
            NOW,
        )
        .expect("PDF should import");
        assert!(Path::new(&imported.file_path).is_file());
        assert_eq!(imported.size_bytes, 26);
        assert_eq!(
            imported.sha256,
            "7e2ecaa3afacaa91daa6d82211e6ff9579c2048189c1599ac8b3b63b74cc5569"
        );
        let with_local = get_resource(&connection, resource.id).unwrap().unwrap();
        assert!(with_local.local_file.is_some());

        let duplicate = import_local_pdf(
            &mut connection,
            resource.id,
            &source,
            destination.path(),
            NOW,
        )
        .expect_err("existing local copy should require removal");
        assert_eq!(duplicate.field, Some("localFile"));

        remove_local_pdf(&mut connection, resource.id, NOW).expect("remove should work");
        assert!(!Path::new(&imported.file_path).exists());
        let after_remove = get_resource(&connection, resource.id).unwrap().unwrap();
        assert!(after_remove.local_file.is_none());
    }

    #[test]
    fn resource_progress_and_local_copies_survive_reopening_without_network() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("resources.sqlite3");
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).expect("date");
        let destination = tempfile::tempdir().expect("destination");
        let source = directory.path().join("offline.pdf");
        fs::write(&source, b"%PDF-1.4\noffline copy\n").expect("pdf");

        let resource_id;
        {
            let mut connection = db::open_database(&path, today).expect("database opens");
            syllabus::seed(&mut connection).expect("syllabus seeds");
            let resource = manual_resource(&connection, "https://example.com/offline.pdf");
            resource_id = resource.id;
            update_progress(
                &connection,
                resource_id,
                ResourceProgressInput {
                    status: "completed".to_owned(),
                    notes: "Finished offline.".to_owned(),
                },
                NOW,
            )
            .expect("progress saves");
            import_local_pdf(
                &mut connection,
                resource_id,
                &source,
                destination.path(),
                NOW,
            )
            .expect("local PDF imports");
        }

        let connection = db::open_database(&path, today).expect("database reopens");
        let resources = list_resources(&connection).expect("resources load");
        let resource = resources
            .iter()
            .find(|resource| resource.id == resource_id)
            .expect("resource persists");
        assert_eq!(resource.study_status, "completed");
        assert_eq!(resource.personal_notes, "Finished offline.");
        assert!(resource.local_file.is_some());
    }

    #[test]
    fn resource_keys_are_stable_and_distinct() {
        let first = resource_key(
            "other",
            "https://drive.google.com/file/d/abc/view?usp=sharing",
        );
        let second = resource_key(
            "other",
            "https://drive.google.com/file/d/abc/view?usp=sharing",
        );
        let different_category = resource_key(
            "recommended_books",
            "https://drive.google.com/file/d/abc/view?usp=sharing",
        );
        assert_eq!(first, second);
        assert_ne!(first, different_category);
        assert_eq!(first.len(), 64);
    }
}
