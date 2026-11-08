import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import {
  checkV2Integrity,
  discardV2Timer,
  finishV2Timer,
  getV2ConceptDetail,
  getV2DatabaseStatus,
  getV2FlashcardStats,
  getV2StudyTotals,
  getV2Timer,
  heartbeatV2Timer,
  initializeV2Database,
  listV2Exams,
  listV2Flashcards,
  listV2Sessions,
  pauseV2Timer,
  replayV2ConceptMastery,
  resumeV2Timer,
  reviewV2Flashcard,
  saveV2Flashcard,
  searchV2Concepts,
  setV2SessionAllocations,
  startV2Timer,
  updateV2ConceptMastery,
  updateV2TimerSettings,
  verifyV2MasteryRun,
} from './api';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

describe('V2 API layer', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  it('maps database status to the V2 command', async () => {
    await getV2DatabaseStatus();
    expect(invokeMock).toHaveBeenCalledWith('v2_get_database_status');
  });

  it('maps database initialization to the V2 command', async () => {
    await initializeV2Database();
    expect(invokeMock).toHaveBeenCalledWith('v2_initialize_database');
  });

  it('maps the exam list to the V2 command', async () => {
    await listV2Exams();
    expect(invokeMock).toHaveBeenCalledWith('v2_list_exams');
  });

  it('maps integrity checks with the requested depth', async () => {
    await checkV2Integrity();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_check_integrity', { full: false });

    await checkV2Integrity(true);
    expect(invokeMock).toHaveBeenLastCalledWith('v2_check_integrity', { full: true });
  });

  it('maps timer commands without extra arguments', async () => {
    await getV2Timer();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_get_timer');

    await pauseV2Timer();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_pause_timer');

    await resumeV2Timer();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_resume_timer');

    await finishV2Timer();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_finish_timer');

    await discardV2Timer();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_discard_timer');

    await heartbeatV2Timer();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_heartbeat_timer');
  });

  it('maps timer start and settings inputs exactly', async () => {
    await startV2Timer({
      mode: 'focus',
      sessionDate: '2026-11-04',
      activeExamId: 2,
    });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_start_timer', {
      input: {
        mode: 'focus',
        sessionDate: '2026-11-04',
        activeExamId: 2,
      },
    });

    await updateV2TimerSettings({
      focusMinutes: 45,
      shortBreakMinutes: 10,
      longBreakMinutes: 20,
      notificationsEnabled: false,
    });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_update_timer_settings', {
      settings: {
        focusMinutes: 45,
        shortBreakMinutes: 10,
        longBreakMinutes: 20,
        notificationsEnabled: false,
      },
    });
  });

  it('maps session and allocation commands', async () => {
    await listV2Sessions();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_list_sessions');

    await setV2SessionAllocations({
      sessionId: 7,
      allocations: [
        { examId: 1, allocatedSeconds: 600 },
        { examId: 2, allocatedSeconds: 300 },
      ],
    });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_set_session_allocations', {
      input: {
        sessionId: 7,
        allocations: [
          { examId: 1, allocatedSeconds: 600 },
          { examId: 2, allocatedSeconds: 300 },
        ],
      },
    });

    await getV2StudyTotals();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_get_study_totals');
  });

  it('maps concept library, mastery, and replay commands', async () => {
    await searchV2Concepts({ query: 'mechanics', examCode: 'gate-physics', limit: 20 });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_search_concepts', {
      filter: { query: 'mechanics', examCode: 'gate-physics', limit: 20 },
    });

    await getV2ConceptDetail({ slug: 'kinematics' });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_get_concept_detail', {
      input: { slug: 'kinematics' },
    });

    await updateV2ConceptMastery({
      conceptId: 4,
      targetScore: 72.5,
      localDate: '2026-11-04',
      notes: 'Manual check',
    });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_update_concept_mastery', {
      input: {
        conceptId: 4,
        targetScore: 72.5,
        localDate: '2026-11-04',
        notes: 'Manual check',
      },
    });

    await replayV2ConceptMastery({ conceptId: 4 });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_replay_concept_mastery', {
      input: { conceptId: 4 },
    });

    await verifyV2MasteryRun({ runId: 9 });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_verify_mastery_run', {
      input: { runId: 9 },
    });
  });

  it('maps flashcard commands and inputs exactly', async () => {
    const filter = {
      query: 'gradient',
      examId: 1,
      subjectArea: 'Mathematical Methods',
      nodeId: 11,
      conceptSlug: 'kinematics',
      includeArchived: false,
      dueOnly: true,
      today: '2026-11-08',
      sort: 'due',
      limit: 100,
    };
    await listV2Flashcards(filter);
    expect(invokeMock).toHaveBeenLastCalledWith('v2_list_flashcards', { filter });

    await saveV2Flashcard({
      front: 'Prompt',
      back: 'Answer',
      subjectArea: 'Mechanics',
      conceptId: 7,
      examLinks: [{ examId: 1, syllabusNodeId: 11 }],
      today: '2026-11-08',
    });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_save_flashcard', {
      input: {
        front: 'Prompt',
        back: 'Answer',
        subjectArea: 'Mechanics',
        conceptId: 7,
        examLinks: [{ examId: 1, syllabusNodeId: 11 }],
        today: '2026-11-08',
      },
    });

    await reviewV2Flashcard({
      flashcardId: 3,
      rating: 'good',
      today: '2026-11-08',
    });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_review_flashcard', {
      input: { flashcardId: 3, rating: 'good', today: '2026-11-08' },
    });

    await getV2FlashcardStats({ today: '2026-11-08', examId: 1 });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_get_flashcard_stats', {
      input: { today: '2026-11-08', examId: 1 },
    });
  });
});
