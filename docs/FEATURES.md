# Feature Guide

Physics Tracker supports four independent examination workspaces while sharing a single local
study database.

## Dashboard and daily study

- Combined dashboard with configurable section visibility and order.
- Today's study time, daily target, planner summary, revision due state, recent topics, and weak
  areas.
- Quick-start timer configurations saved locally.
- Four examination cards with activation state, dates, and workspace access.
- Local database health and schema facts for troubleshooting.

## Study timer and sessions

- Focus, short-break, and long-break modes.
- Pause, resume, finish, and discard controls.
- Timestamp-based elapsed time with heartbeat checkpointing.
- Restart, sleep, and stale-heartbeat recovery that excludes unverified time.
- Manual session entry and editing.
- Session allocation across examinations and syllabus topics.
- Session history, totals, and per-exam/topic duration breakdowns.
- Global timer indicator available while navigating the application.

## Syllabus and concepts

- Official-source syllabus trees for CSIR-UGC NET, GATE Physics, TIFR GS, and JEST Physics.
- Source/version metadata and review status.
- Topic progress, priority, confidence, and notes.
- Search and completion filters.
- Shared concept library with exam mappings and review status.
- Mastery evidence, replay history, manual mastery controls, and concept hierarchy.
- Explicit separation between shared concept mastery and exam-specific syllabus completion.

## Reference libraries

- Curated formula reference with equations, symbols, assumptions, conventions, and syllabus links.
- Terms and SI units reference with aliases, notation, dimensions, and conversion information.
- Read-only built-in physical constants with exact values, uncertainty, precision, and source
  metadata.
- User-managed custom formulas, units, and constants.
- KaTeX-rendered mathematics with plain-text fallback.
- CSIR-UGC NET, GATE, TIFR, and JEST relevance filters.

## Practice and mock tests

- Previous-year question (PYQ) bank with source, year, section, topic, and verification metadata.
- Question-level practice with MCQ, MSQ, and numeric-answer support.
- Full-length and question-level mock tests using verified paper patterns.
- Answer-key versions, marking rules, negative marking, and numeric tolerance.
- Attempt history, score history, accuracy, and weak-topic summaries.
- CSV import and validation workflows.
- Explicit separation of question content, answer keys, and source verification.

## Flashcards and calculator

- Shared flashcards with optional exam and syllabus links.
- Review scheduling, ratings, due queues, retention statistics, and review history.
- Active-recall review mode.
- Scientific calculator with memory, previous-answer reuse, angle modes, keyboard support, and
  session history.
- Copy and reuse calculations without transmitting them anywhere.

## Planner and revision

- Checklist tasks with exam-anchored scheduling.
- Planned study blocks with date, time, duration, status, and explicit session links.
- Month/week/day calendar and daily agenda.
- Exam-date rescheduling with preview and confirmation.
- Spaced-repetition schedules with adaptive intervals.
- Daily revision plans, workload limits, deferrals, and immutable plan history.
- Revision completion, rescheduling, and reminders.

## Sticky notes and resources

- Quick-access sticky-note panel and full Notes page.
- Autosave, pinning, search, color coding, and optional exam/subject/topic associations.
- Shared resource catalogue with external links, study progress, and managed local PDF copies.
- Source discovery with explicit review and no automatic rewriting of original files.

## Analytics

- Study time, daily/weekly trends, consistency, and targets.
- Practice accuracy, mock-test history, and weak-topic analytics.
- Syllabus completion and concept mastery summaries.
- Accessible SVG charts with equivalent data tables and sample-size caveats.
- Analytics never fabricate missing data or zero-fill absent dates.

## Data safety and portability

- Verified SQLite backup and restore with manifests and checksums.
- Automatic backup scheduling and retention.
- Pre-migration and pre-restore safety snapshots.
- CSV exports for supported datasets.
- Complete versioned JSON export and validated merge/replace import.
- Local-first storage with no telemetry or remote account.

## Appearance and accessibility

- Dark and light themes.
- Selectable interface font and text size.
- Keyboard navigation, visible focus states, skip link, and accessible dialogs.
- Reduced-motion support.
- Semantic labels for charts, forms, notes, and interactive controls.
