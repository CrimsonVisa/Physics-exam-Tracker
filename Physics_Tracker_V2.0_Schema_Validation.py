#!/usr/bin/env python3
"""Standalone schema validation for Physics Tracker V2.0 Specification V1.3.2.

Uses only the Python standard library. Creates a temporary SQLite database.
Never opens or modifies the application database.
"""

from __future__ import annotations

import hashlib
import json
import re
import sqlite3
import sys
import tempfile
from pathlib import Path

SPEC_NAME = "Physics_Tracker_V2.0_Specification_V1.3.2.md"


def load_block(text: str, name: str) -> str:
    pattern = re.compile(
        rf"<!-- {re.escape(name)}:BEGIN -->(.*?)<!-- {re.escape(name)}:END -->",
        re.DOTALL,
    )
    match = pattern.search(text)
    if not match:
        raise RuntimeError(f"Missing schema block: {name}")
    block = match.group(1)
    # Strip markdown code fences if present.
    block = re.sub(r"^\s*```sql\s*$", "", block, flags=re.MULTILINE)
    block = re.sub(r"^\s*```\s*$", "", block, flags=re.MULTILINE)
    return block.strip()


class TestRunner:
    def __init__(self, conn: sqlite3.Connection) -> None:
        self.conn = conn
        self.passed = 0
        self.failed: list[tuple[str, str]] = []

    def ok(self, condition: bool, label: str, detail: str = "") -> None:
        if condition:
            self.passed += 1
            print(f"PASS  {label}")
        else:
            self.failed.append((label, detail or "assertion failed"))
            print(f"FAIL  {label}: {detail}")

    def expect_fail(self, sql: str, label: str, params: tuple = ()) -> None:
        try:
            self.conn.execute("SAVEPOINT expect_fail")
            self.conn.execute(sql, params)
            self.conn.execute("RELEASE expect_fail")
        except sqlite3.Error:
            self.conn.execute("ROLLBACK TO expect_fail")
            self.conn.execute("RELEASE expect_fail")
            self.passed += 1
            print(f"PASS  {label} (rejected)")
        else:
            self.failed.append((label, "operation unexpectedly succeeded"))
            print(f"FAIL  {label}: operation unexpectedly succeeded")

    def expect_ok(self, sql: str, label: str, params: tuple = ()) -> None:
        try:
            self.conn.execute(sql, params)
            self.passed += 1
            print(f"PASS  {label}")
        except sqlite3.Error as exc:
            self.failed.append((label, str(exc)))
            print(f"FAIL  {label}: {exc}")

    def scalar(self, sql: str, params: tuple = ()):
        cur = self.conn.execute(sql, params)
        row = cur.fetchone()
        return None if row is None else row[0]


def main() -> int:
    spec_path = Path(__file__).with_name(SPEC_NAME)
    if not spec_path.exists():
        print(f"FAIL  specification not found: {spec_path}", file=sys.stderr)
        return 2

    spec = spec_path.read_text(encoding="utf-8")
    tables_sql = load_block(spec, "SCHEMA-TABLES")
    indexes_sql = load_block(spec, "SCHEMA-INDEXES")
    triggers_sql = load_block(spec, "SCHEMA-TRIGGERS")
    seed_sql = load_block(spec, "SEED-SQL")

    with tempfile.TemporaryDirectory(prefix="physics_tracker_v132_") as tmp:
        db_path = Path(tmp) / "validation.sqlite3"
        conn = sqlite3.connect(db_path)
        conn.execute("PRAGMA foreign_keys = ON")
        runner = TestRunner(conn)

        try:
            version = conn.execute("SELECT sqlite_version()").fetchone()[0]
            runner.ok(tuple(map(int, version.split("."))) >= (3, 37), "SQLite >= 3.37", version)
            runner.ok(conn.execute("SELECT json_valid('{}')").fetchone()[0] == 1, "JSON1 available")
            runner.ok(conn.execute("PRAGMA foreign_keys").fetchone()[0] == 1, "foreign_keys ON")

            conn.executescript(tables_sql)
            conn.executescript(indexes_sql)
            conn.executescript(triggers_sql)
            conn.executescript(seed_sql)
            conn.commit()

            runner.ok(runner.scalar("SELECT COUNT(*) FROM exams") == 4, "exactly four exams")
            runner.ok(runner.scalar("SELECT COUNT(*) FROM exam_profiles") == 4, "exactly four profiles")
            codes = {row[0] for row in conn.execute("SELECT code FROM exams ORDER BY code")}
            runner.ok(
                codes == {"csir-net-physics", "gate-physics", "tifr-gs-physics", "jest-physics"},
                "exact exam code set",
            )
            runner.ok(
                runner.scalar("SELECT COUNT(*) FROM exam_profiles WHERE exam_date IS NOT NULL") == 0,
                "all exam dates initially NULL",
            )

            # Time validation.
            runner.expect_ok(
                "INSERT INTO planned_study_blocks(title,calendar_date,start_time,end_time,status,created_at,updated_at) "
                "VALUES('valid block','2026-11-03','09:30','10:30','planned','2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "valid HH:MM 09:30 accepted",
            )
            for bad in ("9:30", "24:00", "09:60", "09-30"):
                runner.expect_fail(
                    "INSERT INTO planned_study_blocks(title,calendar_date,start_time,end_time,status,created_at,updated_at) "
                    "VALUES('bad block','2026-11-03',?, '10:30','planned','2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                    f"invalid time {bad} rejected",
                    (bad,),
                )

            runner.expect_fail(
                "UPDATE exam_profiles SET exam_date='2026-02-30' WHERE exam_id=1",
                "invalid date rejected",
            )

            # Seed/config guards.
            runner.expect_fail("DELETE FROM exams WHERE id=1", "exam delete blocked")
            runner.expect_fail("UPDATE exams SET code='other' WHERE id=1", "exam code update blocked")
            runner.expect_fail("DELETE FROM exam_profiles WHERE exam_id=1", "profile delete blocked")
            runner.expect_fail("UPDATE schema_migrations SET name='x' WHERE version=1", "schema migration update blocked")

            # Session allocation invariants.
            runner.expect_ok(
                "INSERT INTO study_sessions(session_date,duration_seconds,source,created_at,updated_at) "
                "VALUES('2026-11-03',120,'manual','2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "session inserted",
            )
            sid = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO session_exam_allocations(session_id,exam_id,allocated_seconds,created_at,updated_at) "
                "VALUES(?,1,100,'2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "allocation within duration",
                (sid,),
            )
            runner.expect_fail(
                "INSERT INTO session_exam_allocations(session_id,exam_id,allocated_seconds,created_at,updated_at) "
                "VALUES(?,2,30,'2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "allocation over duration rejected",
                (sid,),
            )
            runner.expect_fail(
                "UPDATE study_sessions SET duration_seconds=50 WHERE id=?",
                "duration below allocation rejected",
                (sid,),
            )

            # Concept/evidence/replay.
            runner.expect_ok(
                "INSERT INTO concepts(slug,title,domain,created_at,updated_at) "
                "VALUES('quantum-mechanics','Quantum Mechanics','Quantum Mechanics','2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "concept inserted",
            )
            cid = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO concept_evidence(concept_id,source_kind,source_id,source_version,exam_id,local_date,occurred_at,raw_value_json,algorithm_version_id,created_at) "
                "VALUES(?,'study_session',?,1,1,'2026-11-03','2026-11-03T10:00:00Z','{\"duration_seconds\":3600}',1,'2026-11-03T10:00:00Z')",
                "raw evidence inserted",
                (cid, sid),
            )
            eid = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_fail("UPDATE concept_evidence SET notes='x' WHERE id=?", "evidence update blocked", (eid,))
            runner.expect_fail("DELETE FROM concept_evidence WHERE id=?", "evidence delete blocked", (eid,))
            runner.expect_ok(
                "INSERT INTO concept_evidence_voids(evidence_id,voided_at,reason,created_at) "
                "VALUES(?,'2026-11-03T11:00:00Z','source_edited','2026-11-03T11:00:00Z')",
                "evidence void inserted",
                (eid,),
            )
            runner.expect_fail(
                "UPDATE concept_evidence_voids SET reason='source_deleted' WHERE evidence_id=?",
                "evidence void update blocked",
                (eid,),
            )
            runner.expect_fail(
                "INSERT INTO mastery_replay_results(recalc_run_id,evidence_id,concept_id,algorithm_version_id,applied_delta,score_before,score_after,category,cap_consumed,created_at) "
                "VALUES(999,?,?,1,4,0,4,'study',4,'2026-11-03T12:00:00Z')",
                "replay result without run rejected",
                (eid, cid),
            )
            runner.expect_ok(
                "INSERT INTO mastery_recalc_runs(concept_id,reason,algorithm_version_id,evidence_count,evidence_fingerprint,resulting_score,resulting_confidence,started_at,finished_at) "
                "VALUES(?, 'source_created',1,1,? ,4,1,'2026-11-03T12:00:00Z','2026-11-03T12:00:01Z')",
                "recalc run inserted before results",
                (cid, "0" * 64),
            )
            run_id = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO mastery_replay_results(recalc_run_id,evidence_id,concept_id,algorithm_version_id,applied_delta,score_before,score_after,category,cap_consumed,created_at) "
                "VALUES(?,?,?,1,4,0,4,'study',4,'2026-11-03T12:00:01Z')",
                "replay result inserted after run",
                (run_id, eid, cid),
            )
            runner.expect_fail(
                "UPDATE mastery_replay_results SET applied_delta=5 WHERE recalc_run_id=?",
                "replay result update blocked",
                (run_id,),
            )

            # Mock completion and history.
            runner.expect_ok(
                "INSERT INTO exam_content_versions(exam_id,content_type,version_label,source_reference,verified_on,review_status,created_at) "
                "VALUES(2,'paper_pattern','test','fixture','2026-01-01','verified','2026-01-01T00:00:00Z')",
                "content version inserted",
            )
            cv = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO exam_question_types(exam_id,code,title,answer_format,created_at,updated_at) "
                "VALUES(2,'MCQ','MCQ','single_choice','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                "question type inserted",
            )
            qt = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO exam_sections(exam_id,content_version_id,default_question_type_id,code,title,display_order,default_correct_marks,default_incorrect_marks,default_unanswered_marks,created_at,updated_at) "
                "VALUES(2,?,?,'S1','Section 1',1,4,-1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                "exam section inserted",
                (cv, qt),
            )
            sec = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_fail(
                "INSERT INTO mock_tests(exam_id,test_name,test_date,duration_minutes,max_marks,marks_obtained,status,created_at,updated_at) "
                "VALUES(2,'direct completed','2026-11-03',60,40,30,'completed','2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "direct completed mock insert rejected",
            )
            runner.expect_ok(
                "INSERT INTO mock_tests(exam_id,test_name,test_date,duration_minutes,max_marks,status,created_at,updated_at) "
                "VALUES(2,'mock','2026-11-03',60,40,'planned','2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "mock planned inserted",
            )
            mid = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO mock_test_sections(exam_id,mock_test_id,section_id,question_count,correct,incorrect,unanswered,max_marks,marks_obtained,time_spent_minutes,created_at,updated_at) "
                "VALUES(2,?,?,10,8,1,1,40,31,50,'2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "mock section inserted",
                (mid, sec),
            )
            runner.expect_ok(
                "UPDATE mock_tests SET status='completed', marks_obtained=31 WHERE id=?",
                "mock completed with valid totals",
                (mid,),
            )
            runner.expect_fail(
                "UPDATE mock_test_sections SET correct=9 WHERE mock_test_id=?",
                "completed mock section update blocked",
                (mid,),
            )
            runner.expect_ok(
                "INSERT INTO mock_test_score_history(exam_id,mock_test_id,version,max_marks,marks_obtained,sections_json,marking_context_json,recorded_at) "
                "VALUES(2,?,1,40,31,'[]','{}','2026-11-03T01:00:00Z')",
                "mock score history inserted",
                (mid,),
            )
            runner.expect_fail(
                "UPDATE mock_test_score_history SET marks_obtained=30 WHERE mock_test_id=?",
                "mock score history update blocked",
                (mid,),
            )
            runner.expect_fail("DELETE FROM mock_tests WHERE id=?", "completed mock delete blocked", (mid,))

            # Readiness snapshot.
            runner.expect_ok(
                "INSERT INTO exam_readiness_snapshots(exam_id,as_of_date,formula_version_id,mastery_score,coverage_score,pyq_score,mock_score,revision_score,overall_score,confidence,components_json,computed_at,created_at) "
                "VALUES(2,'2026-11-03',1,50,60,55,65,70,59,'medium','{}','2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "readiness snapshot inserted",
            )
            runner.expect_fail(
                "INSERT INTO exam_readiness_snapshots(exam_id,as_of_date,formula_version_id,mastery_score,coverage_score,pyq_score,mock_score,revision_score,overall_score,confidence,components_json,computed_at,created_at) "
                "VALUES(2,'2026-11-04',1,50,60,NULL,65,70,59,'medium','{}','2026-11-04T00:00:00Z','2026-11-04T00:00:00Z')",
                "overall readiness with missing component rejected",
            )
            runner.expect_fail(
                "UPDATE exam_readiness_snapshots SET overall_score=60 WHERE exam_id=2 AND as_of_date='2026-11-03'",
                "readiness snapshot update blocked",
            )
            runner.expect_fail(
                "DELETE FROM exam_readiness_snapshots WHERE exam_id=2 AND as_of_date='2026-11-03'",
                "readiness snapshot delete blocked",
            )

            # Revision-plan lifecycle and cross-exam scope.
            runner.expect_ok(
                "INSERT INTO exam_content_versions(exam_id,content_type,version_label,source_reference,verified_on,review_status,created_at) "
                "VALUES(1,'syllabus','test','fixture','2026-01-01','verified','2026-01-01T00:00:00Z')",
                "exam 1 content version inserted",
            )
            cv1 = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO exam_syllabus_sources(exam_id,content_version_id,code,title,publisher,source_reference,verified_on,created_at,updated_at) "
                "VALUES(1,?,'S1','Source','Publisher','fixture','2026-01-01','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                "exam 1 source inserted",
                (cv1,),
            )
            src1 = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO exam_syllabus_nodes(exam_id,source_id,code,title,node_kind,display_order,created_at,updated_at) "
                "VALUES(1,?,'T1','Topic 1','topic',1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                "exam 1 node inserted",
                (src1,),
            )
            node1 = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO exam_syllabus_sources(exam_id,content_version_id,code,title,publisher,source_reference,verified_on,created_at,updated_at) "
                "VALUES(2,?,'S1','Source','Publisher','fixture','2026-01-01','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                "exam 2 source inserted",
                (cv,),
            )
            src2 = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO exam_syllabus_nodes(exam_id,source_id,code,title,node_kind,display_order,created_at,updated_at) "
                "VALUES(2,?,'T1','Topic 1','topic',1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                "exam 2 node inserted",
                (src2,),
            )
            node2 = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO exam_node_concepts(exam_id,node_id,concept_id,weight,created_at) VALUES(2,?,?,1,'2026-01-01T00:00:00Z')",
                "exam 2 concept mapping inserted",
                (node2, cid),
            )
            runner.expect_fail(
                "INSERT INTO exam_syllabus_nodes(exam_id,parent_id,source_id,code,title,node_kind,display_order,created_at,updated_at) "
                "VALUES(2,?,?,'X','X','topic',2,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                "cross-exam node parent rejected",
                (node1, src2),
            )
            runner.expect_fail(
                "INSERT INTO exam_syllabus_nodes(exam_id,parent_id,source_id,code,title,node_kind,display_order,created_at,updated_at) "
                "VALUES(2,NULL,?,'Y','Y','topic',3,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                "cross-exam node source rejected",
                (src1,),
            )
            runner.expect_ok(
                "INSERT INTO revision_daily_plans(plan_date,exam_id,version,effective_budget_minutes,max_topics,status,created_at,updated_at) "
                "VALUES('2026-11-05',NULL,1,60,10,'active','2026-11-05T00:00:00Z','2026-11-05T00:00:00Z')",
                "combined daily plan inserted",
            )
            combined_plan = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO revision_plan_items(concept_id,focus_exam_id,status,estimated_minutes,triage_score,lane,source_due_date,created_at,updated_at) "
                "VALUES(?,2,'planned',30,50,'strengthen','2026-11-05','2026-11-05T00:00:00Z','2026-11-05T00:00:00Z')",
                "revision plan item inserted",
                (cid,),
            )
            plan_item = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO revision_daily_plan_items(daily_plan_id,plan_item_id,position,estimated_minutes,lane,triage_score,membership_status,created_at,updated_at) "
                "VALUES(?,?,0,30,'strengthen',50,'planned','2026-11-05T00:00:00Z','2026-11-05T00:00:00Z')",
                "combined membership inserted",
                (combined_plan, plan_item),
            )
            runner.expect_fail(
                "UPDATE revision_daily_plan_items SET position=1 WHERE daily_plan_id=? AND plan_item_id=?",
                "active membership position update blocked",
                (combined_plan, plan_item),
            )
            runner.expect_fail(
                "DELETE FROM revision_daily_plan_items WHERE daily_plan_id=? AND plan_item_id=?",
                "membership delete blocked",
                (combined_plan, plan_item),
            )
            runner.expect_ok(
                "UPDATE revision_plan_items SET active_daily_plan_id=? WHERE id=?",
                "plan item active plan set",
                (combined_plan, plan_item),
            )
            runner.expect_ok(
                "INSERT INTO revision_daily_plans(plan_date,exam_id,version,effective_budget_minutes,max_topics,status,created_at,updated_at) "
                "VALUES('2026-11-05',2,1,60,10,'active','2026-11-05T00:00:00Z','2026-11-05T00:00:00Z')",
                "exam daily plan inserted",
            )
            exam_plan = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_fail(
                "INSERT INTO revision_daily_plan_items(daily_plan_id,plan_item_id,position,estimated_minutes,lane,triage_score,membership_status,created_at,updated_at) "
                "VALUES(?,?,0,30,'strengthen',50,'planned','2026-11-05T00:00:00Z','2026-11-05T00:00:00Z')",
                "plan item cannot belong to two active plans",
                (exam_plan, plan_item),
            )
            runner.expect_ok(
                "UPDATE revision_daily_plans SET status='closed' WHERE id=?",
                "combined plan closed",
                (combined_plan,),
            )
            runner.ok(
                runner.scalar("SELECT active_daily_plan_id FROM revision_plan_items WHERE id=?", (plan_item,)) is None,
                "closing plan clears active plan id",
            )
            runner.expect_ok(
                "INSERT INTO revision_history(concept_id,trigger_exam_id,revision_date,scheduled_for_date,outcome,interval_before_days,interval_after_days,mastery_before,mastery_after,created_at) "
                "VALUES(?,2,'2026-11-05','2026-11-05','good',30,45,50,55,'2026-11-05T00:00:00Z')",
                "revision history inserted",
                (cid,),
            )
            hist = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "UPDATE revision_plan_items SET status='completed', revision_history_id=?, active_daily_plan_id=NULL WHERE id=?",
                "revision plan item completed",
                (hist, plan_item),
            )
            runner.expect_fail(
                "UPDATE revision_plan_items SET status='planned' WHERE id=?",
                "completed revision plan item update blocked",
                (plan_item,),
            )
            runner.expect_fail(
                "DELETE FROM revision_plan_items WHERE id=?",
                "completed revision plan item delete blocked",
                (plan_item,),
            )
            runner.expect_fail(
                "INSERT INTO revision_deferrals(concept_id,plan_item_id,event_kind,previous_due_date,new_due_date,occurred_on,reason,created_at) "
                "VALUES(?,?,'defer','2026-11-05','2026-11-06','2026-11-05','time','2026-11-05T00:00:00Z')",
                "completed revision item deferral blocked",
                (cid, plan_item),
            )
            runner.expect_fail(
                "DELETE FROM revision_daily_plans WHERE id=?",
                "daily plan with membership delete blocked",
                (combined_plan,),
            )

            # Resource file history immutability.
            runner.expect_ok(
                "INSERT INTO resources(title,resource_type,created_at,updated_at) "
                "VALUES('Book','book','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                "resource inserted",
            )
            rid = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO resource_file_history(resource_id,file_name,file_path,sha256,size_bytes,storage_mode,action,occurred_at) "
                "VALUES(?,'a.pdf','C:/a.pdf',? ,10,'external','import','2026-01-01T00:00:00Z')",
                "resource file history inserted",
                (rid, "0" * 64),
            )
            runner.expect_fail(
                "UPDATE resource_file_history SET action='remove' WHERE resource_id=?",
                "resource file history update blocked",
                (rid,),
            )
            runner.expect_fail(
                "DELETE FROM resource_file_history WHERE resource_id=?",
                "resource file history delete blocked",
                (rid,),
            )

            # Composite cross-exam references.
            runner.expect_fail(
                "INSERT INTO exam_syllabus_nodes(exam_id,parent_id,source_id,code,title,node_kind,display_order,created_at,updated_at) "
                "VALUES(2,NULL,999,'X','X','topic',1,'2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "node with missing source rejected",
            )
            runner.expect_fail(
                "INSERT INTO pyq_questions(exam_id,section_id,question_type_id,year,session_label,question_number,max_marks,created_at,updated_at) "
                "VALUES(2,999,?,2026,'','1',4,'2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "PYQ with missing section rejected",
                (qt,),
            )
            runner.expect_ok(
                "INSERT INTO exam_question_types(exam_id,code,title,answer_format,created_at,updated_at) "
                "VALUES(1,'MCQ','MCQ','single_choice','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                "exam 1 question type inserted",
            )
            qt1 = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_ok(
                "INSERT INTO exam_sections(exam_id,content_version_id,default_question_type_id,code,title,display_order,default_correct_marks,default_incorrect_marks,default_unanswered_marks,created_at,updated_at) "
                "VALUES(1,?,?,'S1','Section 1',1,4,-1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                "exam 1 section inserted",
                (cv1, qt1),
            )
            sec1 = runner.scalar("SELECT last_insert_rowid()")
            runner.expect_fail(
                "INSERT INTO pyq_questions(exam_id,section_id,question_type_id,year,session_label,question_number,max_marks,created_at,updated_at) "
                "VALUES(2,?,?,2026,'','2',4,'2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "cross-exam PYQ section rejected",
                (sec1, qt),
            )
            runner.expect_fail(
                "INSERT INTO pyq_questions(exam_id,section_id,question_type_id,year,session_label,question_number,max_marks,created_at,updated_at) "
                "VALUES(2,?,?,2026,'','3',4,'2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "cross-exam PYQ question type rejected",
                (sec, qt1),
            )
            runner.expect_fail(
                "INSERT INTO mock_test_sections(exam_id,mock_test_id,section_id,question_count,correct,incorrect,unanswered,max_marks,marks_obtained,created_at,updated_at) "
                "VALUES(2,?,?,1,1,0,0,4,4,'2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "cross-exam mock section rejected",
                (mid, sec1),
            )
            runner.expect_fail(
                "INSERT INTO study_sessions(session_date,duration_seconds,source,primary_exam_id,syllabus_node_id,created_at,updated_at) "
                "VALUES('2026-11-03',60,'manual',2,?,'2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "cross-exam session node rejected",
                (node1,),
            )
            runner.expect_fail(
                "INSERT INTO resource_exam_links(resource_id,exam_id,category,syllabus_node_id,priority,created_at,updated_at) "
                "VALUES(?,2,'notes',?,'normal','2026-11-03T00:00:00Z','2026-11-03T00:00:00Z')",
                "cross-exam resource node rejected",
                (rid, node1),
            )

            # Trigger inventory.
            expected_triggers = {
                "trg_schema_migrations_no_update", "trg_schema_migrations_no_delete",
                "trg_exams_no_delete", "trg_exams_code_immutable", "trg_exam_profiles_no_delete",
                "trg_exam_date_changes_no_update", "trg_exam_date_changes_no_delete",
                "trg_syllabus_code_immutable", "trg_session_allocation_insert",
                "trg_session_allocation_update", "trg_study_session_duration_guard",
                "trg_revision_history_no_update", "trg_revision_history_no_delete",
                "trg_revision_deferrals_no_update", "trg_revision_deferrals_no_delete",
                "trg_revision_plan_completed_no_update", "trg_revision_plan_completed_no_delete",
                "trg_revision_plan_item_active_plan_guard_insert",
                "trg_revision_plan_item_active_plan_guard_update",
                "trg_revision_daily_plan_items_active_parent",
                "trg_revision_daily_plan_items_no_delete",
                "trg_revision_daily_plan_items_update_guard",
                "trg_revision_plan_close_clear_active", "trg_revision_plan_delete_guard",
                "trg_revision_deferral_completed_guard", "trg_revision_exam_scope_guard",
                "trg_revision_combined_scope_guard", "trg_concept_evidence_no_update",
                "trg_concept_evidence_no_delete", "trg_concept_evidence_voids_no_update",
                "trg_concept_evidence_voids_no_delete", "trg_mastery_recalc_runs_no_update",
                "trg_mastery_recalc_runs_no_delete", "trg_mastery_replay_results_no_update",
                "trg_mastery_replay_results_no_delete", "trg_readiness_snapshot_no_update",
                "trg_readiness_snapshot_no_delete", "trg_mock_score_history_no_update",
                "trg_mock_score_history_no_delete", "trg_resource_file_history_no_update",
                "trg_resource_file_history_no_delete", "trg_mock_insert_completed_guard",
                "trg_mock_delete_completed_guard", "trg_mock_completion_validate",
                "trg_mock_section_insert_completed_guard", "trg_mock_section_update_completed_guard",
                "trg_mock_section_delete_completed_guard",
                "trg_revision_plan_item_scope_guard",
                "trg_revision_daily_plan_exam_immutable",
            }
            actual_triggers = {row[0] for row in conn.execute("SELECT name FROM sqlite_master WHERE type='trigger'")}
            runner.ok(expected_triggers <= actual_triggers, "all expected triggers exist", str(expected_triggers - actual_triggers))

            fk_rows = conn.execute("PRAGMA foreign_key_check").fetchall()
            runner.ok(not fk_rows, "foreign_key_check empty", str(fk_rows))
            integrity = conn.execute("PRAGMA integrity_check").fetchone()[0]
            runner.ok(integrity == "ok", "integrity_check ok", integrity)

        finally:
            conn.close()

        print()
        print(f"PASSED: {runner.passed}")
        print(f"FAILED: {len(runner.failed)}")
        if runner.failed:
            for label, detail in runner.failed:
                print(f"  - {label}: {detail}")
            return 1
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
