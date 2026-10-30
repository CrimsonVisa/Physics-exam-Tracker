# CSIR-NET Physics Tracker

An offline-first Windows desktop study tracker for CSIR-UGC NET Physical Sciences.

**Phase 6 status:** the dashboard, persistent planning settings, exam countdown, official-source
Physical Sciences syllabus tracker, persisted study timer, study-session history, dated checklist
tasks, planned study blocks, month/week/day calendar, previous-year question (PYQ) bank, mock-test
history, CSV import workflows, and outcome analytics are implemented. The syllabus is seeded with 182
topics across General Aptitude and the nine Physical Sciences units, and each topic can store a
personal status, priority, confidence rating, and notes. The timer records focus sessions with restart
and sleep recovery, and the dashboard shows today's focus time plus the day's tasks and planned blocks.
Exam-date changes use a review-and-apply confirmation before any future exam-anchored task moves. CSV
templates are generated locally and saved to the user's Downloads folder. Spaced-repetition scheduling,
backup/restore, and richer charting are not implemented yet.

## Stack

- [Tauri 2](https://v2.tauri.app/) desktop shell with a Rust backend
- React 19 and TypeScript (strict)
- Vite 8
- Rust-managed SQLite through `rusqlite` (bundled SQLite) with versioned migrations
- Vitest and Testing Library
- ESLint and Prettier
- MIT license

## Prerequisites

- Windows 10 or 11 (64-bit) with the WebView2 runtime
- Node.js 22.12 or newer and npm
- Rust 1.95 or newer with the `x86_64-pc-windows-msvc` toolchain
- Visual Studio with the **Desktop development with C++** workload and a Windows SDK
- The [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for Windows

This project was validated with Node.js 26.7.0, npm 11.19.0, and Rust/Cargo 1.99.1. Rust 1.95 is the
minimum because `rusqlite_migration` 2.x requires it.

### PowerShell notes

If Cargo is installed but not on `PATH`, add it for the current PowerShell session:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
```

Use `npm.cmd` rather than `npm` if PowerShell's execution policy blocks the `npm.ps1` shim.

## Setup

```powershell
git clone https://github.com/CrimsonVisa/csir-net-tracker.git
cd csir-net-tracker
npm.cmd install
```

Both `package-lock.json` and `src-tauri/Cargo.lock` are committed so installs are reproducible.

## Development

Run the desktop app with the Vite dev server and Rust backend:

```powershell
npm.cmd run tauri:dev
```

To run only the frontend in a browser:

```powershell
npm.cmd run dev
```

Tauri commands are unavailable in a plain browser, so the dashboard shows a database-unavailable
error instead of settings. The desktop app is the supported development target.

## Data storage

The database is created outside the repository in Tauri's per-user application-data directory:

```text
%APPDATA%\com.crimsonvisa.csirnetphysics\csir-net-tracker.sqlite3
```

SQLite may also create `-wal` and `-shm` files alongside it. The directory is created on first launch
if needed. The schema is versioned with deterministic, append-only migrations (current schema version:
5), and the app enables foreign keys, WAL journaling, a busy timeout, and transactions for multi-step
writes. Upgrading a version-1 database preserves its saved settings and seeds the catalog without
creating any progress rows. Upgrading a version-2 database adds timer and session tables with default
timer settings and an idle timer; it does not fabricate study sessions. Upgrading a version-3 database
adds checklist and planned-block tables without changing existing settings, syllabus progress, timer
state, or study sessions. Upgrading a version-4 database adds the PYQ and mock-test tables without
changing any Phase 1–5 data.

The initial single-row settings record uses:

- Exam date: `2026-12-19` — **provisional**; verify the actual subject date on the admit card.
- Preparation start date: the local first-run date, clamped to the exam date. No preparation history
  is fabricated.
- Daily study target: `240` minutes (4 hours).

Date-only values are stored as `YYYY-MM-DD` strings and validated in Rust as real calendar dates,
including leap years. The frontend uses one date-only utility and one reusable date input; it never
parses date-only strings through UTC timestamps.

## Syllabus catalog and progress

Migration `0002_syllabus.sql` adds four `STRICT` tables:

- `syllabus_sources` — official document metadata and the date it was checked.
- `syllabus_sections` — the ten units (General Aptitude plus the nine Physical Sciences units), with
  stable codes and display order.
- `syllabus_topics` — 182 catalog entries with stable codes, source references, category, ordering,
  and an archive flag.
- `syllabus_progress` — one optional row per topic containing the user's study record.

Catalog definitions and study progress are deliberately separate. Seeding upserts the built-in
catalog by stable code and never writes to `syllabus_progress`; the initial database therefore
contains no fabricated progress. If a future catalog removes a topic, the topic is archived rather
than deleted, and its existing progress row is preserved. Re-seeding the same catalog is idempotent.

The catalog uses only three categories: **General Aptitude**, **Core**, and **Advanced**. These are
the categories used by the syllabus document. They are distinct from the exam question paper's Parts
A, B, and C; the app labels that distinction explicitly and does not call Core "Part A" or Advanced
"Part B".

### Official sources

The catalog was checked on **29 October 2026** against:

- CSIR-HRDG, **Proposed Syllabus for Physical Sciences** —
  <https://csirhrdg.res.in/SiteContent/ManagedContent/ContentFiles/20201221140202824physicalscience_syllbus.pdf>
  (source page: <https://csirhrdg.res.in/Home/Index/1/Default/3485/78>)
- National Testing Agency, **Joint CSIR-UGC NET December 2026 Information Bulletin** —
  <https://cdnbbsr.s3waas.gov.in/s3efdf562ce2fb0ad460fd8e9d33e57f57/uploads/2026/10/202610071407707951.pdf>
  (source page: <https://csirnet.nta.nic.in/>)

The CSIR PDF's own title, **Proposed Syllabus for Physical Sciences**, is preserved in the app and in
this README. The six General Aptitude entries follow the bulletin's stated emphases: logical
reasoning, graphical analysis, analytical and numerical ability, quantitative comparison, series
formation, and puzzles.

Topic titles were transcribed from these documents with typographic normalization for spacing,
dashes, and symbols. They are not claimed to be a character-perfect OCR reproduction. No topic
weighting or guaranteed exam appearance is implied.

### Recorded progress

Each topic supports:

- Status: Not started, Learning, Revised, or Confident.
- Priority: Low, Normal, High, or High-Yield.
- Confidence: a whole number from 1 to 5.
- Notes: up to 2,000 characters, stored only in the local SQLite database.
- An updated timestamp shown by the editor after the first save.

Priority and **High-Yield** are personal planning labels. They are not official CSIR weightings.
Likewise, the summary percentage is a study-tracking approximation, not a marks prediction or an
official CSIR score. It gives every active topic equal weight and uses these status scores:

| Status      | Score |
| ----------- | ----: |
| Not started |     0 |
| Learning    |   0.5 |
| Revised     |  0.75 |
| Confident   |     1 |

The syllabus screen supports text search (topic title or code), section, category, status, and
priority filters. Search is debounced, archived topics are excluded, and filters are validated in
Rust. The UI includes loading, save-in-progress, validation, retry, and empty states.

## Study timer and sessions

Migration `0003_study_timer_and_sessions.sql` adds three `STRICT` tables:

- `timer_settings` — the single settings row: focus, short-break, and long-break durations plus the
  desktop-notification preference.
- `timer_state` — the single active-timer row: status, mode, duration, accumulated active time,
  segment start, first start, heartbeat, optional topic, local session date, and recovery/event state.
- `study_sessions` — completed focus sessions with a stable ID, optional syllabus topic, local session
  date, optional UTC start/end timestamps, duration, `timer` or `manual` source, notes, and
  created/updated timestamps.

Default durations are 25 minutes of focus, a 5-minute short break, and a 15-minute long break. Focus
durations are limited to 1–240 minutes, short breaks to 1–60 minutes, and long breaks to 1–120
minutes. One timer can be active at a time. Desktop notifications are off by default.

### How elapsed time is protected

The timer does not trust an interval callback in the webview. Running time is derived from persisted
UTC timestamps. A Rust heartbeat runs every five seconds independently of the window, records a
heartbeat timestamp, and completes a running timer when its configured duration is reached. The Timer
page polls snapshots every five seconds for authoritative state and repaints the visible clock from
the server timestamp every 250 milliseconds; minimization, delayed rendering, or a slow webview
therefore do not change the accounting.

The recovery policy is deliberately conservative:

- If the app restarts while a timer is running, the timer comes back **paused** at the last
  heartbeat. Time while the app was closed is excluded until the user resumes.
- If heartbeats stop for more than 20 seconds — for example during computer sleep or a suspended
  process — the next reconciliation pauses the timer and excludes the gap. Computer sleep is never
  counted as study time.
- Paused time is never counted. Only completed focus time is written to `study_sessions`; completed
  short and long breaks never enter study totals.
- Discarding a timer never reconciles or saves a session, even if the configured duration has passed.
- Each recovery records its reason (`restart` or `inactivity`), the excluded milliseconds, and the
  recovery timestamp. The Timer page shows this explanation alongside the saved focus time.

### Session history and totals

Timer sessions are inserted when a focus block completes, either naturally or when the user chooses
**Finish & save**. Timer start and end timestamps are kept in UTC. A focus block that crosses midnight
stays attributed to its **local start date**; the session date is not split between two days.

Sessions can also be entered or corrected manually. Manual entries have no clock timestamps, accept a
duration from 1 second to 24 hours, and support up to 2,000 characters of notes. Editing and deletion
run in transactions and recalculate the affected totals. Deleting a session never changes syllabus
progress or deletes a topic.

The Sessions page filters by date range and optional topic, groups history by session date, and shows
the selected-range, today, and current-week totals. Weeks run Monday through Sunday in the local
calendar. Daily and weekly totals exclude breaks and paused time. Archived topics are not offered for
new sessions, but they remain visible and filterable wherever historical sessions reference them; the
topic foreign key is restrictive, so a topic with study history cannot be destructively removed.

The dashboard adds a full-width study card showing today's focus time against the existing daily
target, today's session count, and this week's total. It intentionally does not attempt the full
analytics planned for a later phase.

### Notification limits

When notifications are enabled, the app asks Windows for permission and the Rust heartbeat sends a
notification only when a timer completes naturally. A manually finished timer does not notify.
Windows notification settings, Focus Assist, and system policy can still suppress delivery, so the
in-app timer state remains authoritative. Notifications remain off unless the user opts in.

## Checklist, planner, and calendar

Migration `0004_checklist_and_planner.sql` adds two `STRICT` tables:

- `checklist_tasks` — dated actions with a title, optional description, scheduled date, optional
  deadline, optional syllabus topic, priority, completion state/timestamp, and a manual or
  exam-relative schedule anchor.
- `planned_study_blocks` — manual calendar commitments with a title, optional topic, calendar date,
  start/end time, status, notes, and an optional link to an existing study session.

The migration is append-only. It does not rewrite the earlier settings, syllabus, timer, or session
tables.

### Checklist behavior

Tasks are either `open` or `completed`. Completing a task stores a completion timestamp; completing
an already-completed task is idempotent and does not replace that timestamp. Reopening clears the
completion timestamp. Editing the title, description, dates, topic, priority, or schedule anchor never
changes the completion state or timestamp.

Open tasks can be rescheduled from the checklist. A reschedule changes the scheduled date and converts
the task to a manual schedule anchor; completed tasks cannot be rescheduled. Deleting a task is
confirmed in the UI and never deletes syllabus progress or study sessions.

Tasks display overdue and upcoming indicators from the effective due date: the deadline when one is
set, otherwise the scheduled date. The checklist supports open/completed/all views and title or
description search. Priority and High-Yield remain personal planning labels.

### Planned study blocks and explicit session links

Planned blocks are calendar intentions, not recorded study time. They never contribute to daily,
weekly, or range totals, and saving or completing a block never creates a study session. Dates are
local `YYYY-MM-DD` values; times are strict local `HH:MM` values with an end later than the start on
the same date.

A block can link to an existing study session only after the block is marked completed, only through
the editor's explicit save action, and only to a session recorded on the same local date. Clearing or
cancelling the block clears the link. The link does not alter the session or its duration. If a linked
session is deleted later, the block is kept and its session reference becomes empty.

### Calendar

The Calendar page provides month, week, and day views. Weeks run Monday through Sunday and use the
same frontend `dateOnly` helpers and `DateField` input as the rest of the app; no second date system or
UTC conversion is introduced. The page supports previous/next navigation, a Today shortcut, clicking a
date, and typing a jump date.

The calendar shows scheduled tasks, planned blocks, the selected day's task and block lists, deadlines
on the selected day, upcoming deadlines for the next 30 days, and overdue open tasks. Each view has
loading, error/retry, and empty states. Planned blocks remain visually distinct from study sessions.

### Exam-date rescheduling

Changing the exam date never writes immediately. The app first requests a preview that reports:

- Future open tasks using an exam-relative offset and their old/new dates.
- Future open tasks or planned blocks that would fall outside the proposed preparation start and exam
  date.
- Counts of completed tasks, manual tasks, and planned blocks that will be preserved.

Applying the preview is transactional. Only the accepted future exam-anchored tasks move, using their
stored offsets from the new exam date; settings are updated in the same transaction. Completed tasks,
manual-dated tasks, planned blocks, historical sessions, recorded study time, and syllabus progress
never move. Out-of-window items are preserved with warnings rather than silently changed. Cancelling
the preview writes nothing. A stale preview is rejected if the exam date or an anchored task changed,
and the user must preview again. Direct settings updates also refuse an exam-date change while future
open exam-anchored tasks exist, requiring the preview flow.

## PYQ bank and mock tests

Migration `0005_pyq_and_mock_tests.sql` adds two independent `STRICT` tables:

- `pyq_attempts` — one row per historical attempt, with question year, optional exam/session,
  Part A/B/C, syllabus topic reference, user-supplied question reference, attempt date, outcome,
  optional marks, time spent, notes, optional difficulty, and an import fingerprint.
- `mock_tests` — one row per full mock test, with name, date, duration, maximum marks, optional
  reported score, per-part question/correct/incorrect/unanswered counts, per-part marking values,
  notes, review observations, and an import fingerprint.

No examination questions are seeded, generated, or fetched. The app stores only the user's own
references and attempt records. PYQ attempts and mock tests are separate domains: recording a mock
test never creates individual PYQ attempts.

### PYQ attempts

PYQ statuses are `attempted`, `correct`, `incorrect`, and `skipped`. Accuracy uses only correct and
incorrect outcomes; skipped and outcome-unknown attempts are reported separately. Multiple attempts
for the same question reference remain separate historical records when entered manually.

The PYQ screen filters by question year, part, syllabus unit, topic, status, and free-text search
across reference, session, and notes. It supports full create/edit/delete, with confirmation before
deletion. Topic references are validated against the local syllabus catalog, including archived
topics for historical data.

### Mock tests and scoring

Mock tests record question counts per part. The following invariants are validated in Rust and in
the database: correct + incorrect + unanswered must equal the part's question count, and a test must
contain at least one question across Parts A, B, and C.

Each part stores three editable marking values: marks for a correct answer, marks for an incorrect
answer, and marks for an unanswered question. The calculated score is:

```text
sum over parts of:
  correct × correct_marks
  + incorrect × incorrect_marks
  + unanswered × unanswered_marks
```

No universal marking scheme is assumed. Correct-answer marks must be supplied for any part with
questions; incorrect and unanswered values default to zero (no penalty) when left blank. The optional
reported score must not exceed maximum marks and is preserved for comparison, while analytics and
trends use the calculated score.

### Analytics

The Analytics screen uses only saved records:

- PYQ accuracy by topic and syllabus unit.
- PYQ outcomes by examination year and Part A/B/C.
- Correct, incorrect, skipped, and outcome-unknown counts.
- Mock-test calculated-score trend, average, best, latest, and part-wise performance.
- Descriptive study-time-versus-outcome rows, paired by local date. These are explicitly descriptive,
  not causal.
- A weak-topic screen that requires at least five correct-or-incorrect attempts and accuracy below
  50%. Topics with fewer attempts are labelled **Insufficient data**, never “weak” from one or two
  attempts.

## CSV import

PYQ and mock-test screens each have a separate CSV workflow. The header-only template is generated
locally and saved to the user's Downloads folder without overwriting an existing file; no network
access is used. Selecting a file first
produces a preview with valid, duplicate, and rejected rows and row-level field errors. Nothing is
written during preview. Confirmed imports run in one SQLite transaction, skip duplicates, and report
imported, skipped, and rejected counts. Cancelling a preview writes nothing.

The parser accepts UTF-8 (including a BOM), quoted fields with commas, doubled quotes, and embedded
newlines. Headers are required exactly; missing, unknown, or duplicate headers are rejected. Invalid
dates, references, numbers, and required fields reject the affected row rather than silently dropping
it.

### PYQ template columns

```text
question_year,exam_session,part,topic_code,question_reference,attempt_date,status,marks_obtained,max_marks,time_spent_seconds,notes,difficulty,external_id
```

Required: `question_year`, `part`, `topic_code`, `question_reference`, `attempt_date`, `status`.
`external_id` is optional; when supplied it is the stable duplicate identifier. Otherwise a
deterministic fingerprint of the source row is used. Imported records keep their fingerprint when
edited, so reimporting the original source remains a duplicate. Status is `attempted`, `correct`,
`incorrect`, or `skipped`; difficulty is optional 1–5.

### Mock-test template columns

```text
test_name,test_date,duration_minutes,max_marks,marks_obtained,part_a_questions,part_a_correct,part_a_incorrect,part_a_unanswered,part_a_correct_marks,part_a_incorrect_marks,part_a_unanswered_marks,part_b_questions,part_b_correct,part_b_incorrect,part_b_unanswered,part_b_correct_marks,part_b_incorrect_marks,part_b_unanswered_marks,part_c_questions,part_c_correct,part_c_incorrect,part_c_unanswered,part_c_correct_marks,part_c_incorrect_marks,part_c_unanswered_marks,notes,review_observations,external_id
```

Required: test name/date/duration/maximum marks and all question/correct/incorrect/unanswered counts.
Correct-answer marks are required for any part with questions; incorrect and unanswered marks default
to zero. `external_id` is optional and behaves as for PYQ imports.

## Typography note

Latin Modern is not currently bundled with the app. No font files or font packages were added through
Phase 5; the interface continues to use system fonts (Segoe UI for UI text and Georgia for display
headings). Adopting a self-hosted Latin Modern or comparable math-friendly face remains a UI polish
task for a later phase.

## Checks

```powershell
npm.cmd run typecheck    # strict TypeScript checking
npm.cmd run lint         # ESLint, zero warnings allowed
npm.cmd run format:check # Prettier
npm.cmd run test         # Vitest + Testing Library
npm.cmd run build        # production frontend bundle
npm.cmd run rust:fmt     # cargo fmt --check
npm.cmd run rust:clippy  # Clippy with warnings denied
npm.cmd run rust:test    # Rust unit tests
```

Run all JavaScript/TypeScript linting, formatting, tests, and Rust checks together:

```powershell
npm.cmd run check
```

`format` and `test:coverage` are also available as `npm.cmd run format` and
`npm.cmd run test:coverage`.

## Windows production build

```powershell
npm.cmd run tauri:build
```

The NSIS installer is written to:

```text
src-tauri\target\release\bundle\nsis\
```

The executable and bundle are generated from the product identity in `src-tauri/tauri.conf.json`:

- Product: CSIR-NET Physics Tracker
- Identifier: `com.crimsonvisa.csirnetphysics`
- Version: `0.1.0`

## Project structure

```text
src/                     React frontend and tests
  components/DateField.tsx      reusable accessible date input
  components/StudyTodayCard.tsx dashboard study total and target progress
  components/TodayPlanCard.tsx  dashboard tasks, blocks, and deadline summary
  components/TaskEditor.tsx     checklist form with manual/exam scheduling
  components/PlannedBlockEditor.tsx planned-block form and explicit session link
  components/PyqEditor.tsx      PYQ attempt form with topic and outcome fields
  components/MockTestEditor.tsx mock-test form with editable per-part marking
  components/CsvImportPanel.tsx preview, validation report, and transactional import UI
  screens/SyllabusScreen.tsx    syllabus catalog, filters, and topic editor
  screens/TimerScreen.tsx       persisted timer, recovery display, and settings
  screens/SessionsScreen.tsx    session history, filters, totals, and manual CRUD
  screens/ChecklistScreen.tsx   task CRUD, completion, rescheduling, and search
  screens/CalendarScreen.tsx    month/week/day views and selected-day editing
  screens/PyqScreen.tsx         PYQ filters, history, CRUD, and CSV workflow
  screens/MockTestsScreen.tsx   mock-test history, scoring, CRUD, and CSV workflow
  screens/AnalyticsScreen.tsx   PYQ, mock, part, weak-topic, and study-time analytics
  lib/dateOnly.ts               timezone-independent date and calendar utilities
  lib/timeOfDay.ts              strict local HH:MM validation and planned duration
  lib/studyTime.ts              duration and timer-clock formatting
  lib/useTodayDateOnly.ts       local-date refresh for long-running windows
  lib/settingsApi.ts            typed settings command wrappers
  lib/syllabusApi.ts            typed syllabus command wrappers
  lib/timerApi.ts               typed timer command wrappers
  lib/sessionsApi.ts            typed session and totals command wrappers
  lib/plannerApi.ts             typed checklist, calendar, and rescheduling wrappers
  lib/pyqApi.ts                 typed PYQ and PYQ-import command wrappers
  lib/mockTestApi.ts            typed mock-test and import command wrappers
  lib/analyticsApi.ts           typed analytics command wrappers
  lib/csvImportApi.ts           shared import preview/result types
  lib/commandError.ts           shared structured-error decoding
src-tauri/               Tauri configuration and Rust backend
  capabilities/          Tauri window permissions
  icons/                 Application icons
  migrations/            Append-only SQL schema migrations
  src/db.rs              App-data path, connection setup, managed state, migrations
  src/date_only.rs       Authoritative date parsing and formatting
  src/settings.rs        Settings validation, defaults, load, and guarded update
  src/syllabus_catalog.rs Official-source catalog definitions
  src/syllabus.rs        Catalog seeding, search, progress, summaries, and validation
  src/timer.rs           Timer state machine, heartbeat, recovery, and accounting
  src/sessions.rs        Session CRUD, validation, filters, and study totals
  src/planner.rs         Checklist, planned blocks, calendar, agenda, and validation
  src/reschedule.rs      Exam-date preview, outside-window warnings, and apply
  src/pyq.rs             PYQ CRUD, filters, imports, and duplicate detection
  src/mock_tests.rs      Mock-test CRUD, marking configuration, and imports
  src/csv_import.rs      Strict CSV parser and shared import report types
  src/analytics.rs       PYQ, mock, weak-topic, and study-performance calculations
  src/error.rs           Serializable structured command errors
  src/lib.rs             Tauri builder, setup, and typed commands
  src/main.rs            Desktop entry point
index.html               Vite HTML entry
vite.config.ts           Vite and Vitest configuration
```

## License

MIT © 2026 CrimsonVisa. See [LICENSE](LICENSE).
