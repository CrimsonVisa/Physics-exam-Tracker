# CSIR-NET Physics Tracker

An offline-first Windows desktop study tracker for CSIR-UGC NET Physical Sciences.

**Release 1.2.0 development status:** the preparation dashboard, persistent planning settings, exam countdown,
official-source Physical Sciences syllabus tracker, persisted study timer, study-session history, dated
checklist tasks, planned study blocks, month/week/day calendar, previous-year question (PYQ) bank,
mock-test history, CSV import workflows, outcome analytics, configurable spaced-repetition revision
scheduler, study trends, topic intelligence, preparation timeline, SQLite-backed dashboard
customization, verified full-database backup and restore, automatic backup retention, repeat-safe JSON
imports, per-dataset CSV export, complete versioned JSON portability, and a Study Resources catalogue
with manual source refresh and an optional local PDF library are implemented. The syllabus
is seeded with 182 topics across General Aptitude and the nine Physical Sciences units, and each topic
can store a personal status, priority, confidence rating, and notes. The timer records focus sessions
with restart and sleep recovery. Exam-date changes use a review-and-apply confirmation before any future
exam-anchored task moves. CSV templates are generated locally and saved to the user's Downloads folder.

See [RELEASE_NOTES_1.2.0.md](RELEASE_NOTES_1.2.0.md) for the v1.2.0 Stage 1 notes.

## Release

- Product: **CSIR-NET Physics Tracker 1.2.0**
- Platform: Windows 10/11 x64
- Installer: NSIS, per-current-user by default
- Data: local SQLite database under `%APPDATA%\com.crimsonvisa.csirnetphysics`
- Network requirement: none for core study functionality; resource discovery and remote links require
  internet access
- Backup encryption: **not implemented**; backup and export files are unencrypted

**v1.2.0 is in development.** The verified installer below is the current public 1.1.0 release; a
1.2.0 installer will be produced after the remaining v1.2.0 stages and final verification.

### Verified public 1.1.0 artifact

- Installer: `src-tauri\target\release\bundle\nsis\CSIR-NET Physics Tracker_1.1.0_x64-setup.exe`
- Installer size: 5,901,285 bytes (5.63 MiB)
- Installer SHA-256: `378839FD7A9D558E8257D955D94964972D85B4B7ECD336151475D8AF0785BD2E`
- Standalone executable SHA-256: `48942C7C25012F874A81D984B0FF4726B14BA2F0E010449E7E2E7D412B574C0C`
- Default install location: `%LOCALAPPDATA%\CSIR-NET Physics Tracker`
- Start Menu shortcut: `CSIR-NET Physics Tracker`
- Uninstall removes program files and shortcuts. `%APPDATA%\com.crimsonvisa.csirnetphysics` is retained.

On 30 October 2026 the installer was verified with silent install, launch, uninstall, reinstall, and a
second uninstall. The existing schema-10 database and settings survived every step. First-run
initialization was also verified by the release executable against an isolated empty data profile
(schema 10, default settings, 182 topics, 9 dashboard sections). A separate physical clean Windows VM
was not available, so that last check is application-level rather than a second-machine install test.

## System requirements

- Windows 10 or Windows 11, 64-bit
- Microsoft Edge WebView2 runtime (the installer can bootstrap it if missing)
- Enough local disk space for the application, SQLite database, and any backups/exports you create
- No internet connection is required after installation

## Installation

1. Run `CSIR-NET Physics Tracker_1.1.0_x64-setup.exe`.
2. The default installer mode is per-current-user and does not require Administrator access.
3. A Start Menu shortcut is created under **CSIR-NET Physics Tracker**.
4. Launch from the Start Menu.
5. The default exam date is **provisional**; verify it and edit it in Overview settings.

A desktop shortcut is not configured by default. Creating one from the Start Menu shortcut is safe.

## Uninstall and data preservation

The uninstaller removes installed application files and shortcuts. It does **not** remove the user
database, backups, or exports stored outside the installation directory. Reinstalling the application
will reopen the existing database and run any required append-only schema upgrades.

To remove all personal data after uninstalling, manually delete:

```text
%APPDATA%\com.crimsonvisa.csirnetphysics
```

Only do that after creating and verifying a backup if you may need the data again.

## Troubleshooting

- **Database unavailable on startup:** close other instances, confirm the application-data folder is
  writable, and restart. Do not delete the database before copying it aside.
- **Database locked:** close any duplicate app process and retry. The app uses a busy timeout for short
  contention, not for permanently locked files.
- **Backup folder unavailable:** choose an existing local or removable-drive folder. Explicit UNC
  network paths are rejected.
- **Restore rejected:** keep the `.sqlite3` and `.sqlite3.manifest.json` files together and unmodified.
  A missing manifest, changed checksum, truncated file, or newer schema is refused.
- **Notifications do not appear:** check Windows notification permission and Focus Assist. Revision
  reminders are optional and off by default.
- **A screen shows cached data after import:** use the app's reload action. Backup/restore and import
  operations are completed in the Rust process before the webview reloads.
- **Need to recover after a failed operation:** look under
  `%APPDATA%\com.crimsonvisa.csirnetphysics\backups\safety` for verified safety snapshots.

## Offline and accessibility

Core functionality has no required external API or remote asset. The Content Security Policy blocks
remote scripts/styles/network connections except the Tauri IPC endpoint and local assets. Fonts are
installed Windows system fonts; no web font is downloaded. The app supports keyboard navigation,
visible focus rings, a skip-to-content link, labelled forms, semantic tables, status announcements, and
native dialogs. Charts provide textual supporting values and empty states. Official-source links open
in the external browser only when the user selects them.

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
10), and the app enables foreign keys, WAL journaling, a busy timeout, and transactions for multi-step
writes. Upgrading a version-1 database preserves its saved settings and seeds the catalog without
creating any progress rows. Upgrading a version-2 database adds timer and session tables with default
timer settings and an idle timer; it does not fabricate study sessions. Upgrading a version-3 database
adds checklist and planned-block tables without changing existing settings, syllabus progress, timer
state, or study sessions. Upgrading a version-4 database adds the PYQ and mock-test tables without
changing any Phase 1–5 data. Upgrading a version-5 database adds revision scheduler tables without
changing settings, syllabus progress, sessions, planner records, PYQ attempts, or mock tests.
Upgrading a version-6 database adds the dashboard section-preference table and seeds the default
layout without changing any Phase 1–7 data. Upgrading a version-7 database adds database identity and
backup configuration tables without changing any study record, and creates the identifier used by
backup manifests. Upgrading a version-8 database adds import-batch provenance for repeat-safe JSON
merges without changing any study record. Upgrading a version-9 database adds the resource catalogue,
independent resource-study progress, local PDF records, and discovery-run history without changing
settings, syllabus progress, sessions, planner records, PYQs, mocks, or revision history. Every upgrade
from an existing database first creates a verified pre-migration snapshot; if that snapshot fails, the
upgrade is aborted.

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

The preparation dashboard shows today's focus time against the daily target, today's session count,
the current-week total, and the selected study-analytics trends. Planned blocks and unfinished timers
remain separate from completed session totals.

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

- PYQ accuracy by topic and syllabus unit, outcomes by examination year and Part A/B/C, and
  correct/incorrect/skipped/outcome-unknown counts.
- Mock-test calculated-score trend, average, best, latest, part-wise performance, attempted versus
  unanswered counts, and response trends. PYQ and mock metrics are never combined.
- Daily, weekly, and monthly study trends, duration buckets, unit attribution, focus/break support,
  date filters, and same-length period comparisons.
- A weak-topic screen that requires at least five correct-or-incorrect attempts and accuracy below
  50%. Topics with fewer attempts are labelled **Insufficient data**, never “weak” from one or two
  attempts.
- Topic intelligence, the preparation timeline, and actionable insights described in the Phase 8
  section below.

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

## Spaced repetition and revision scheduler

Migration `0006_revision_scheduler.sql` adds:

- `revision_settings` — the singleton scheduling configuration: initial intervals for Again,
  Difficult, Good, and Easy; growth multiplier; minimum/maximum interval; weekend handling; daily
  target; optional reminders; and the last reminder date.
- `revision_schedules` — at most one current schedule per syllabus topic, with active/suspended state,
  last and next revision dates, current interval, completed count, previous outcome, mastery estimate,
  and schedule notes.
- `revision_history` — append-only completed outcomes with the date revised, original due date, outcome,
  revision notes, interval before/after, mastery before/after, and creation timestamp.

History update and delete triggers reject mutation at the database level. Scheduling rows can change,
but completed outcomes are never deleted when a topic is rescheduled, suspended, or reactivated.

### Default algorithm

The defaults are **not** claimed to be scientifically optimal and can be edited:

| Outcome      | Default next interval |
| ------------ | --------------------: |
| Again/Forgot |                 1 day |
| Difficult    |                2 days |
| Good         |                5 days |
| Easy         |               10 days |

The deterministic rules are:

- Again and Difficult reset to their configured initial interval.
- Good and Easy grow from the current interval by the configured growth multiplier, while never going
  below their configured initial interval.
- The result is clamped between the configured minimum and maximum.
- When weekends are excluded, a next date landing on Saturday or Sunday moves to Monday.
- The date is calculated with real calendar arithmetic, including leap years, month/year transitions,
  and a bounded 1–9999 supported calendar range.

The default growth multiplier is `1.5`, the minimum interval is 1 day, and the maximum is 3,650 days.
The mastery estimate is a transparent planning estimate, not an official score: Again/Difficult/Good/
Easy contribute 0/35/70/95 points; subsequent outcomes use a 70% previous + 30% new blended value.
Revision completion never changes syllabus status, syllabus confidence, or study time.

### Workflow and integration

Every syllabus topic is listed in the revision screen even before it is scheduled. Users can create a
schedule, start a revision, record Again/Difficult/Good/Easy with notes, cancel without a write,
manually reschedule, suspend/reactivate, and view the complete per-topic history. Starting a revision is
not persisted as a separate session; only a completed outcome is written, in one transaction with its
schedule update. Revision duration is not recorded as study time and no study session is created
automatically.

The syllabus screen shows each topic's revision state and next due date. The calendar shows active
revision dates as a separate calendar-item type; one schedule row per topic prevents duplicate calendar
entries when a date changes. The revision detail can prefill the existing planned-block editor with the
topic and a revision-title suggestion, but the user must explicitly save the block. Planned blocks and
revisions remain outside study-time totals unless a separate study session is recorded.

### Advisory recommendations

Recommendations are deterministic and advisory. They combine overdue/due dates, previous outcomes,
mastery estimate, and PYQ accuracy only when at least five scored attempts exist. Topics with fewer
attempts are explicitly annotated as insufficient evidence rather than labelled weak. Scheduling
changes never happen from a recommendation alone; the user must open the topic or choose **Add to
schedule**.

Mock tests are stored at part level without a topic mapping. They remain available in Analytics, but
they are not used to rank individual topic recommendations; the revision dashboard says this
explicitly instead of implying a topic-level mock signal that does not exist.

Review reminders are optional and off by default. When enabled and Windows grants permission, the app
checks once per local day for due/overdue revisions and records the reminder date so the same day is
not notified repeatedly. Notification failures never alter schedules or block revision work.

## Preparation dashboard and advanced analytics

Migration `0007_dashboard_layout.sql` adds `dashboard_sections`, a `STRICT` table with one row per
known dashboard section, a visibility flag, a unique order, and an update timestamp. The default layout
is seeded for existing and new users. Section order and visibility are written to SQLite in a
transaction, and **Restore default layout** re-seeds the documented defaults. The browser's
`localStorage` is not used for this preference.

The preparation dashboard combines saved records only:

- The exam countdown and preparation day use the local settings and local calendar-date arithmetic.
- Syllabus progress reuses the existing equal-weight status summary. It is not treated as exam
  readiness or topic mastery.
- Total, current-week, current-month, and today's study time come from completed focus sessions.
- Revision due/overdue/completed counts reuse the revision scheduler's saved schedules and append-only
  history.
- Upcoming open deadlines cover the planner's next-seven-day window; upcoming planned blocks cover the
  next seven calendar days. Planned blocks are not study time.
- Recent PYQ attempts are the five latest attempt dates, and the latest mock test uses its saved
  per-part marking configuration.
- The dashboard states that an in-progress timer or unfinished revision form is not saved study time
  until the session or outcome is completed.

### Study analytics

The analytics view supports local date ranges and compares the selected range with the immediately
preceding range of the same day count. It provides:

- Daily, Monday-based weekly, and calendar-month study trends. The daily chart has a zero axis and a
  target reference line, plus a supporting-value table.
- Session-duration buckets (`under 15`, `15-29`, `30-59`, `60-89`, `90-119`, `120+` minutes).
- Recorded time by syllabus unit and an explicit unassigned-session total.
- Focus time from saved sessions. Break time is marked **not tracked**, because timer breaks are not
  saved as study sessions; the app does not estimate break time.
- Daily target completion, weekly proportional targets, active days, consistency, missed completed-day
  targets, and current/longest streaks.
- Revision consistency: completed revisions whose `revision_date` is on or before
  `scheduled_for_date`, plus currently overdue active schedules. It is a historical outcome measure,
  not a claim that every unfinished due schedule was missed.

Consistency and targets only include days on or after the configured preparation start and on or before
today. Pre-preparation records remain in recorded-time totals but are marked ineligible in the daily
chart and never count toward target completion, consistency, or streaks. Today can be active without
being called a missed target yet.

### Streak definition

A study streak is a run of distinct local dates with at least one completed focus session, starting no
earlier than the preparation start date. The current streak ends today when today has recorded study.
If today has no study yet, one grace day is allowed: a streak ending yesterday is still current. A full
missed calendar day ends the current streak. The longest streak is the longest such run in saved history.
Duplicate sessions on one date count as one active day. Planned blocks never extend a streak.

### Topic intelligence

Scored PYQ accuracy is `correct / (correct + incorrect)`. Skipped and outcome-unknown attempts are
shown but not counted as scored attempts.

- Weak with evidence: at least 5 scored attempts and accuracy below 50%.
- Strong with evidence: at least 5 scored attempts and accuracy of 75% or higher.
- Insufficient evidence: 1-4 scored attempts. Topics with no PYQ attempts are reported as no-data
  counts, not classified weak or strong.
- Needs revision: saved due/overdue signals, a previous Again/Difficult outcome, or an upcoming schedule
  with a saved mastery estimate below 40 after at least one completed revision.
- Confidence mismatches: confidence 4-5 with sufficient weak PYQ evidence, or confidence 1-2 with
  sufficient strong PYQ evidence.

Every evidence item displays its supporting attempts, correct/incorrect counts, accuracy, revision
dates/outcomes, and reason text. Syllabus status and confidence are user context, not proof of mastery.
Mock tests remain part-level and are never assigned to individual topics.

### Performance separation

PYQ and mock-test analytics are separate structures, separate tables, and separate UI sections. The app
shows no combined exam score. Mock percentages are calculated from each saved test's own marking rules
and part counts; reported marks remain a separate field. PYQ accuracy is never blended with mock results.

### Preparation timeline

The timeline merges exam date, open and completed checklist deadlines, planned blocks, active revision
dates, completed study sessions, and mock tests. Date-range and activity-type filters are available.
Events are read-only projections and never alter source records. When a planned block is linked to a
completed study session and both fall in the selected range, the session is shown once and the linked
block is omitted to avoid duplicate representation of the same activity.

### Actionable insights

At most five deterministic insights are shown. They can flag a sufficiently supported weak PYQ topic,
overdue revisions, an approaching checklist deadline, lower recorded study time than the prior seven
completed days, or a PYQ Part A accuracy gap. Each insight includes the supporting numbers, a reason,
and a link to the relevant dashboard section. Comparison text is descriptive and does not claim
causation. No readiness score, rank prediction, selection probability, or success percentage is
generated.

## Study resources

The **Resources** screen catalogues study material linked from
[csirnetphysics.in](https://www.csirnetphysics.in/). Metadata is stored locally; files are not
redistributed by this application.

### Discovery sources

Manual **Refresh catalogue** reads these known pages only:

- `https://www.csirnetphysics.in/p/hand-written-notes_28.html`
- `https://www.csirnetphysics.in/p/previous-questions-paper.html`
- `https://www.csirnetphysics.in/p/important-topics.html`
- `https://www.csirnetphysics.in/p/book.html`
- `https://www.csirnetphysics.in/p/syllabus.html`
- `https://www.csirnetphysics.in/p/nptel-video-link.html`

The notes, questions, and books pages expose public Google Apps Script catalogues referenced in their
HTML. The app fetches those page data sources only during a manual refresh, waits at least 1.25 seconds
between requests, caps responses at 5 MB, and stops immediately on HTTP 429. There is no startup
scraping, no background polling, no retry storm, and no authentication bypass. If a page fails, its
existing resources and every user note/status remain in the database. A source page that successfully
parses and no longer lists a link marks that link `removed` rather than deleting it.

The catalogue currently groups resources into handwritten notes, classroom notes, assignments,
formula sheets, previous-year papers, important topics, recommended books, NPTEL courses, YouTube
playlists, and other educational resources. Unit mapping covers General Aptitude and all nine Physical
Sciences units; resource titles are mapped to a specific syllabus topic code only when the title
matches a topic conservatively. Full multi-unit papers use **All Units**, and unresolved entries use
**Unmapped** rather than an invented mapping.

### Manual resources and study tracking

If discovery is blocked, use **Add a resource manually**. The form accepts a title, resource type,
unit, optional topic code, original URL, optional source page URL, and description. Links must use
HTTP or HTTPS, duplicates are rejected, and any topic code must exist in the active syllabus.

Each resource has independent study status (Not started, Studying, Completed, Skipped), personal
notes, and a last-opened timestamp. These values do not change syllabus progress or revision history.

### Google Drive and local PDFs

Original Google Drive file and folder URLs are preserved exactly. The app extracts file/folder IDs
where possible, opens the original URL in the system browser, and never requests Google credentials.
It does not enumerate folders, bypass sharing restrictions, or assume a Drive file is downloadable.

Use **Import authorized PDF** to copy a PDF you already have permission to store:

1. Choose the PDF.
2. Choose a local PDF library folder.
3. The app validates the `%PDF-` header and the 200 MB limit, avoids overwriting files, records
   the absolute path and SHA-256, and keeps the original catalogue entry.
4. Open the local copy through the default PDF reader or remove it later. Removing the local copy keeps
   the catalogue entry and study history.

The Resources screen checks the filesystem whenever it lists resources:

- **Offline copy:** the file exists and its size matches the stored size.
- **Local copy missing:** the file path is unavailable or the file was deleted.
- **Local copy changed:** a file exists at the path but its size differs from the recorded size.
- **Online only:** no local-file record exists; internet access is required.

Routine listing checks existence and size only; SHA-256 is not recalculated for every PDF. The stored
checksum is verified during import and re-link. **Re-link moved file** updates the path only when the
selected file's size and SHA-256 match the stored copy. **Replace with another PDF** copies a new
authorized file after validation, without overwriting or deleting the previous file. Missing or
changed files never delete catalogue metadata, study status, notes, or history.

Opening a local PDF re-checks the path immediately; a missing or changed file produces a useful error
instead of launching the reader. Inaccessible directories and filesystem permission errors are
reported safely.

Local PDF bytes are never stored in SQLite. The full SQLite backup includes the resource catalogue,
study progress, and local-copy records (but not the PDF files themselves), so keep the chosen PDF
folder alongside backups if you need the files. JSON format 2 includes the resource catalogue,
resource study progress/notes, local-file metadata, and discovery history. Local PDF paths are
references, not file contents; after importing or restoring on another computer, missing or changed
files are reported as unavailable and can be re-linked without losing the resource or progress.

### Offline behavior

Previously discovered resources are always read from SQLite and remain visible without internet.
Resources without a verified local copy are marked **Online only** and require internet access.
**Offline copy** is shown only when the local file exists and its size matches the stored size.
Opening a remote link never claims that the file is available offline.

## Backup, restore, and data portability

Migration `0008_backup_and_portability.sql` adds two `STRICT` tables:

- `database_metadata` stores one stable 128-bit database identifier.
- `backup_settings` stores automatic-backup configuration and the most recent success/failure
  metadata. It contains settings only, never a duplicate copy of the database.

### Full SQLite backups

Backups use SQLite's online backup API against the live connection, so uncheckpointed WAL data is
included rather than copied incorrectly from only the main database file. Each snapshot is:

1. Written to a temporary sibling file selected from a safe generated filename.
2. Opened read-only and checked with `PRAGMA integrity_check`.
3. Checked with `PRAGMA foreign_key_check`.
4. Hash-checked with SHA-256.
5. Renamed into place only after verification.
6. Accompanied by `<backup>.sqlite3.manifest.json` containing format version, application version,
   schema version, creation timestamp, database identifier, reason, filename, size, SHA-256 and
   integrity result.

Restore requires the manifest. A renamed backup, missing manifest, truncated file, changed checksum,
or schema newer than this app is rejected. Manual filenames identify the reason, schema, and local
timestamp, for example `csir-net-tracker-manual-v8-20261030-180000.sqlite3`.

The default automatic backup directory is `%APPDATA%\com.crimsonvisa.csirnetphysics\backups`. Safety
backups created before migrations, restore, and JSON import are written under `backups\safety`; they
are never pruned by automatic retention.

### Automatic backups and retention

Automatic backups are **off by default**. Supported frequencies are daily, weekly, and
before-important-operations only. Daily and weekly checks run at application start and once per hour
while the app remains open. Destructive operations always create their own safety backup regardless of
this setting.

Retention applies only to files named as automatic backups. The newest configured number of valid
automatic backups are kept; manual and safety backups are never deleted. If no valid automatic backup
can be verified, pruning is skipped with a warning. Unavailable drives and permission failures are
recorded in `backup_settings` and shown in the Backup screen.

### Restore workflow

The Restore screen selects a backup, reads and validates its manifest and SQLite content, previews
metadata and row counts, and requires explicit confirmation. It then creates and verifies a safety
snapshot of the current database before closing the live connection. The restore candidate is copied
to a verified temporary file, WAL/SHM sidecars are handled, and the candidate is installed atomically.
Any failure before installation leaves the original file untouched. Any failure after installation
recovers the database from the verified safety snapshot and reopens it. Older compatible schemas are
upgraded through normal migrations; newer schemas are never downgraded.

### CSV dataset export

Export creates a timestamped folder containing ten UTF-8 files with a BOM and CRLF rows:

- `syllabus_topics_progress.csv`
- `study_sessions.csv`
- `checklist_tasks.csv`
- `planned_study_blocks.csv`
- `pyq_attempts.csv`
- `mock_tests.csv`
- `revision_schedules.csv`
- `revision_history.csv`
- `settings.csv`
- `dashboard_layout.csv`

Stable IDs and topic/session relationships are included. Every field from the source table is exported,
with section/topic codes added for readability. Commas, quotes, CR/LF and other text are escaped
according to CSV rules. Export queries are read-only.

### Complete JSON export and validated import

The complete JSON format is versioned. **Format 2** records application version, schema version, export
timestamp, database identifier, a unique `importBatchId`, per-table record counts, and all 20 user
tables: settings, syllabus sources/sections/topics/progress, timer settings/state, sessions, tasks,
blocks, PYQs, mocks, revision settings/schedules/history, dashboard sections, study resources,
resource study progress, local-file metadata, and discovery history. Safe backup preferences are
included without machine-specific destination paths. The serialized file is parsed and inserted into a
temporary migrated database before success is reported.

Format 1 exports from versions 1.0.0 and 1.1.0 remain importable; they contain the original 16 core
tables and do not modify the resource catalogue. Format 2 requires schema 10.

Migration `0009_json_import_batches.sql` adds `json_import_batches`. Successful imports record the
batch identifier and result counts there. Preview detects an export that was already merged and
disables merge, preventing duplicate sessions and tasks. Complete replacement remains an explicit
opt-in and can intentionally reapply a file.

Format 1 imports accept schema 8, 9, or 10; format 2 requires schema 10. The whole file is validated in
a temporary schema-10 database before taking a safety backup or writing current data. Duplicate
primary keys, duplicate resource keys, orphaned progress/local-file references, missing fields,
unknown fields, malformed JSON, incompatible format/schema versions, and relationship violations
reject the entire file.

Two explicit strategies are available:

- **Merge:** appends sessions, tasks, blocks, PYQs without conflicting import keys, mocks, and revision
  history under new local IDs; maps syllabus topics by stable code; matches resources by stable
  `resource_key`; preserves existing resource study status, personal notes, and local PDF references;
  skips matching progress/schedules/settings/timer state/revision rules/dashboard layout; rejects rows
  whose required topic or resource is unavailable.
- **Replace:** after the safety backup, removes current core records and, for format 2, resource
  records in foreign-key-safe order; inserts the complete validated export with exported IDs and
  relationships; restores append-only history triggers; verifies foreign keys and exact counts; then
  commits. Local PDF references are imported only when the path exists and its size and SHA-256 match.
  Missing or changed PDFs are skipped, while resource metadata and progress remain. PDF files on disk
  are never deleted. Current backup settings and database identity remain installation-local.

Local PDF references are verified on import. A missing file, wrong size, or checksum mismatch is
reported and skipped rather than imported as a misleading **Offline copy**. The same reference is never
overwritten during merge.

Imports report imported, skipped, conflicting, and rejected counts per table. Any failure rolls back
the entire transaction. The app must be reloaded after import to refresh every screen.

Backup, restore, CSV/JSON export, JSON preview/import, and PYQ/mock CSV preview/import commands run on a Tauri blocking worker
rather than the UI thread. The Backup screen shows a persistent busy state and disables conflicting
actions until completion. Cancellation is not exposed because interrupting a replacement or restore
would create an avoidable recovery risk; the safety-snapshot and rollback procedures remain available
if an operation fails.

### Privacy and file handling

Backup and export files are personal study data. They are written only to user-selected local or
removable-drive folders, never sent over the network; explicit UNC network paths are rejected.
Generated filenames prevent traversal; selected paths must be absolute.
User-facing errors describe the folder or operation without dumping internal absolute paths. No
credentials or unrelated machine information are placed in manifests or JSON exports. Format 2 includes
the local PDF paths you chose for imported files so they can be re-validated; those paths may reveal
folder names, so treat exports as private data.

Backups and exports are **not encrypted**. Anyone with file access can read them. Keep them in a
private folder or encrypted removable drive. Restore requires the adjacent manifest, so move or copy
both files together. Automatic backups run only while the app is running; they are not a background
Windows service. JSON format 1 imports accept schema 8, 9, or 10; format 2 requires schema 10. Use the
SQLite backup restore workflow for older database schemas.

## Typography note

Latin Modern is not currently bundled with the app. No font files or font packages were added through
Phase 10; the interface continues to use local system fonts (Segoe UI for UI text and Georgia for
display headings). Adopting a self-hosted Latin Modern or comparable math-friendly face remains a UI polish
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
- Version: `1.2.0`

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
  components/RevisionSettingsPanel.tsx interval, weekend, target, and reminder settings
  components/RevisionTopicDetail.tsx revision workflow, history, rescheduling, and block planning
  components/PreparationDashboard.tsx customizable preparation overview and insights
  components/DashboardCustomizer.tsx SQLite section ordering, visibility, and reset
  components/AnalyticsCharts.tsx lightweight accessible trend and comparison charts
  screens/SyllabusScreen.tsx    syllabus catalog, filters, topic editor, and revision status
  screens/TimerScreen.tsx       persisted timer, recovery display, and settings
  screens/SessionsScreen.tsx    session history, filters, totals, and manual CRUD
  screens/ChecklistScreen.tsx   task CRUD, completion, rescheduling, and search
  screens/CalendarScreen.tsx    month/week/day views, revisions, and selected-day editing
  screens/RevisionScreen.tsx    due queues, recommendations, filters, and revision dashboard
  screens/PyqScreen.tsx         PYQ filters, history, CRUD, and CSV workflow
  screens/MockTestsScreen.tsx   mock-test history, scoring, CRUD, and CSV workflow
  screens/AnalyticsScreen.tsx   study trends, performance, topic evidence, and timeline tabs
  screens/StudyResourcesScreen.tsx source catalogue, filters, manual registration, and local PDFs
  screens/BackupScreen.tsx      verified backups, restore, automatic settings, CSV/JSON portability
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
  lib/analyticsApi.ts           typed filtered PYQ/mock performance command wrappers
  lib/preparationApi.ts         typed dashboard, study analytics, topic, timeline, and layout wrappers
  lib/backupApi.ts              typed backup, restore, CSV, and JSON portability wrappers
  lib/resourcesApi.ts           typed resource catalogue, discovery, progress, and PDF wrappers
  lib/revisionApi.ts            typed revision scheduler and reminder command wrappers
  lib/revisionScheduling.ts     frontend interval-preview mirror of the Rust algorithm
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
  src/analytics.rs       Filtered PYQ/mock performance, response trends, and study performance
  src/study_analytics.rs Study trends, targets, streaks, consistency, and period comparisons
  src/topic_intelligence.rs Evidence thresholds, topic groups, and confidence mismatches
  src/dashboard.rs       Preparation overview, insights, timeline, and SQLite layout preferences
  src/backup.rs          Online SQLite snapshots, manifests, automatic retention, and restore validation
  src/portability.rs     CSV datasets, complete JSON export, validated merge/replace import
  src/resources.rs       Source-page discovery, resource metadata, study status, and local PDFs
  src/revision.rs        Spaced-repetition scheduling, history, recommendations, and reminders
  src/error.rs           Serializable structured command errors
  src/lib.rs             Tauri builder, setup, and typed commands
  src/main.rs            Desktop entry point
index.html               Vite HTML entry
vite.config.ts           Vite and Vitest configuration
```

## License

MIT © 2026 CrimsonVisa. See [LICENSE](LICENSE).
