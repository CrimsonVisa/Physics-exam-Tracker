import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import {
  checkV2Integrity,
  deleteV2CustomConstant,
  deleteV2CustomFormula,
  deleteV2CustomUnit,
  discardV2Timer,
  finishV2Timer,
  getV2ConceptDetail,
  getV2DatabaseStatus,
  getV2FlashcardStats,
  getV2StudyTotals,
  getV2Timer,
  getV2AnalyticsCharts,
  getV2BuiltinConstants,
  heartbeatV2Timer,
  initializeV2Database,
  listV2CustomConstants,
  listV2CustomFormulas,
  listV2CustomUnits,
  listV2Exams,
  listV2Flashcards,
  listV2Sessions,
  listV2StickyNotes,
  pauseV2Timer,
  replayV2ConceptMastery,
  resumeV2Timer,
  reviewV2Flashcard,
  saveV2CustomConstant,
  saveV2CustomFormula,
  saveV2CustomUnit,
  saveV2Flashcard,
  saveV2StickyNote,
  deleteV2StickyNote,
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

  it('maps custom reference commands and inputs exactly', async () => {
    await listV2CustomFormulas();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_list_custom_formulas');

    await saveV2CustomFormula({
      id: null,
      name: 'Local relation',
      latex: 'E = mc^2',
      plainText: 'E = m c^2',
      category: 'Special relativity',
      description: 'Local formula.',
      notes: '',
      source: '',
      variables: [{ symbol: 'E', meaning: 'energy', unit: 'J' }],
    });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_save_custom_formula', {
      input: {
        id: null,
        name: 'Local relation',
        latex: 'E = mc^2',
        plainText: 'E = m c^2',
        category: 'Special relativity',
        description: 'Local formula.',
        notes: '',
        source: '',
        variables: [{ symbol: 'E', meaning: 'energy', unit: 'J' }],
      },
    });

    await deleteV2CustomFormula(3);
    expect(invokeMock).toHaveBeenLastCalledWith('v2_delete_custom_formula', { id: 3 });

    await listV2CustomUnits();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_list_custom_units');

    await saveV2CustomUnit({
      id: null,
      name: 'Local unit',
      symbol: 'lu',
      quantity: 'Length',
      dimension: 'L',
      siClassification: 'non_si',
      conversionKind: 'scalar',
      conversionFactor: '2.5',
      conversionReferenceUnit: 'm',
      conversionNote: '',
      description: 'Local unit.',
      notes: '',
      source: '',
    });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_save_custom_unit', {
      input: {
        id: null,
        name: 'Local unit',
        symbol: 'lu',
        quantity: 'Length',
        dimension: 'L',
        siClassification: 'non_si',
        conversionKind: 'scalar',
        conversionFactor: '2.5',
        conversionReferenceUnit: 'm',
        conversionNote: '',
        description: 'Local unit.',
        notes: '',
        source: '',
      },
    });

    await deleteV2CustomUnit(4);
    expect(invokeMock).toHaveBeenLastCalledWith('v2_delete_custom_unit', { id: 4 });

    await listV2CustomConstants();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_list_custom_constants');

    await saveV2CustomConstant({
      id: null,
      name: 'Local constant',
      symbol: 'lc',
      valueText: '6.62607015×10^-34',
      unit: 'J s',
      category: 'Fundamental',
      uncertaintyText: 'exact',
      precisionText: '9',
      source: '',
      description: 'Local constant.',
      notes: '',
    });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_save_custom_constant', {
      input: {
        id: null,
        name: 'Local constant',
        symbol: 'lc',
        valueText: '6.62607015×10^-34',
        unit: 'J s',
        category: 'Fundamental',
        uncertaintyText: 'exact',
        precisionText: '9',
        source: '',
        description: 'Local constant.',
        notes: '',
      },
    });

    await deleteV2CustomConstant(5);
    expect(invokeMock).toHaveBeenLastCalledWith('v2_delete_custom_constant', { id: 5 });
  });

  it('maps built-in constants, sticky notes and chart commands', async () => {
    await getV2BuiltinConstants();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_get_builtin_constants');

    await listV2StickyNotes();
    expect(invokeMock).toHaveBeenLastCalledWith('v2_list_sticky_notes');

    const noteInput = {
      id: null,
      title: 'Note',
      body: 'Body',
      color: 'yellow',
      isPinned: false,
      examId: null,
      subjectNodeId: null,
      topicNodeId: null,
    };
    await saveV2StickyNote(noteInput);
    expect(invokeMock).toHaveBeenLastCalledWith('v2_save_sticky_note', {
      input: noteInput,
    });

    await deleteV2StickyNote(8);
    expect(invokeMock).toHaveBeenLastCalledWith('v2_delete_sticky_note', { id: 8 });

    await getV2AnalyticsCharts({
      examId: 1,
      fromDate: '2026-10-12',
      toDate: '2026-11-10',
    });
    expect(invokeMock).toHaveBeenLastCalledWith('v2_get_analytics_charts', {
      input: {
        examId: 1,
        fromDate: '2026-10-12',
        toDate: '2026-11-10',
      },
    });
  });
});
