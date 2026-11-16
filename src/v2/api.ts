import { invoke } from '@tauri-apps/api/core';

export type V2DatabaseState =
  | 'notInitialized'
  | 'healthy'
  | 'unsupportedSchema'
  | 'metadataMismatch'
  | 'integrityFailure'
  | 'foreignKeyFailure'
  | 'inaccessible';

export type V2ExamSummary = {
  id: number;
  code: string;
  name: string;
  shortName: string;
  authority: string;
  officialUrl: string;
  displayOrder: number;
  isActive: boolean;
  examDate: string | null;
};

export type V2DatabaseStatus = {
  state: V2DatabaseState;
  appIdentifier: string;
  dataDir: string | null;
  databasePath: string | null;
  schemaVersion: number | null;
  expectedSchemaVersion: number;
  metadataSchemaVersion: number | null;
  databaseIdValid: boolean;
  migrationChecksumValid: boolean;
  initialized: boolean;
  foreignKeysEnabled: boolean;
  journalMode: string | null;
  integrityCheck: string | null;
  foreignKeyViolations: number | null;
  examCount: number;
  activeExamCount: number;
  exams: V2ExamSummary[];
  errorCode: string | null;
  errorMessage: string | null;
};

export type V2IntegrityReport = {
  ok: boolean;
  integrityCheck: string;
  foreignKeyViolations: number;
  schemaVersion: number;
  examCount: number;
  issues: string[];
};

export function getV2DatabaseStatus(): Promise<V2DatabaseStatus> {
  return invoke<V2DatabaseStatus>('v2_get_database_status');
}

export function initializeV2Database(): Promise<V2DatabaseStatus> {
  return invoke<V2DatabaseStatus>('v2_initialize_database');
}

export function listV2Exams(): Promise<V2ExamSummary[]> {
  return invoke<V2ExamSummary[]>('v2_list_exams');
}

export function checkV2Integrity(full = false): Promise<V2IntegrityReport> {
  return invoke<V2IntegrityReport>('v2_check_integrity', { full });
}

export type V2TimerStatus = 'idle' | 'running' | 'paused';
export type V2TimerMode = 'focus' | 'shortBreak' | 'longBreak';
export type V2TimerLastEvent = 'completed' | 'discarded';
export type V2TimerRecoveryReason = 'restart' | 'inactivity' | 'clockChange' | 'sleep';

export type V2TimerSettings = {
  focusMinutes: number;
  shortBreakMinutes: number;
  longBreakMinutes: number;
  notificationsEnabled: boolean;
};

export type V2TimerSettingsInput = V2TimerSettings;

export type V2TimerState = {
  status: V2TimerStatus;
  mode: V2TimerMode;
  durationMs: number;
  accumulatedMs: number;
  elapsedMs: number;
  remainingMs: number;
  segmentStartedAt: string | null;
  sessionDate: string | null;
  activeExamId: number | null;
  recoveryReason: V2TimerRecoveryReason | null;
  recoveryExcludedMs: number;
  clockSkewDetected: boolean;
  lastEvent: V2TimerLastEvent | null;
  lastEventAt: string | null;
  lastEventDurationMs: number;
  lastCompletedSessionId: number | null;
};

export type V2TopicAllocation = {
  examId: number;
  examCode: string;
  examName: string;
  nodeId: number;
  nodeTitle: string;
  subjectTitle: string;
  allocatedSeconds: number;
};

export type V2TimerSnapshot = {
  state: V2TimerState;
  settings: V2TimerSettings;
  allocations: V2TopicAllocation[];
  examIds: number[];
  serverNowMs: number;
};

export type V2TopicAllocationInput = {
  examId: number;
  nodeId: number;
  allocatedSeconds: number;
};

export type V2StartTimerInput = {
  mode: V2TimerMode;
  sessionDate: string;
  activeExamId: number | null;
  /** Full-credit exam associations. Each selected exam receives the whole session. */
  examIds?: number[];
  allocations?: V2TopicAllocationInput[];
  /**
   * Optional planned focus duration in minutes (quick-start only). When
   * omitted the persisted timer settings define the duration, unchanged.
   */
  durationMinutes?: number | null;
};

export type V2SessionAllocation = {
  examId: number;
  examCode: string;
  examName: string;
  examIsActive: boolean;
  allocatedSeconds: number;
};

export type V2StudySession = {
  id: number;
  sessionDate: string;
  startedAt: string | null;
  endedAt: string | null;
  durationSeconds: number;
  source: 'timer' | 'manual';
  notes: string;
  primaryExamId: number | null;
  allocatedSeconds: number;
  unallocatedSeconds: number;
  allocations: V2SessionAllocation[];
  topicAllocations: V2TopicAllocation[];
  createdAt: string;
  updatedAt: string;
};

export type V2SessionAllocationInput = {
  examId: number;
  allocatedSeconds: number;
};

export type V2SetSessionAllocationsInput = {
  sessionId: number;
  allocations: V2SessionAllocationInput[];
};

export type V2SetSessionExamsInput = {
  sessionId: number;
  examIds: number[];
};

export type V2ExamStudyTotal = {
  examId: number;
  examCode: string;
  examName: string;
  examIsActive: boolean;
  allocatedSeconds: number;
  sessionCount: number;
};

export type V2TopicStudyTotal = {
  examId: number;
  examCode: string;
  nodeId: number;
  nodeTitle: string;
  subjectTitle: string;
  allocatedSeconds: number;
  sessionCount: number;
  lastStudiedDate: string | null;
};

export type V2StudyTotals = {
  totalSeconds: number;
  sessionCount: number;
  allocatedSeconds: number;
  unallocatedSeconds: number;
  exams: V2ExamStudyTotal[];
  topics: V2TopicStudyTotal[];
};

export function getV2Timer(): Promise<V2TimerSnapshot> {
  return invoke<V2TimerSnapshot>('v2_get_timer');
}

export function startV2Timer(input: V2StartTimerInput): Promise<V2TimerSnapshot> {
  return invoke<V2TimerSnapshot>('v2_start_timer', { input });
}

export function setV2TimerAllocations(input: {
  allocations: V2TopicAllocationInput[];
}): Promise<V2TimerSnapshot> {
  return invoke<V2TimerSnapshot>('v2_set_timer_allocations', { input });
}

export function pauseV2Timer(): Promise<V2TimerSnapshot> {
  return invoke<V2TimerSnapshot>('v2_pause_timer');
}

export function resumeV2Timer(): Promise<V2TimerSnapshot> {
  return invoke<V2TimerSnapshot>('v2_resume_timer');
}

export function finishV2Timer(): Promise<V2TimerSnapshot> {
  return invoke<V2TimerSnapshot>('v2_finish_timer');
}

export function discardV2Timer(): Promise<V2TimerSnapshot> {
  return invoke<V2TimerSnapshot>('v2_discard_timer');
}

export function heartbeatV2Timer(): Promise<V2TimerSnapshot> {
  return invoke<V2TimerSnapshot>('v2_heartbeat_timer');
}

export function updateV2TimerSettings(settings: V2TimerSettingsInput): Promise<V2TimerSnapshot> {
  return invoke<V2TimerSnapshot>('v2_update_timer_settings', { settings });
}

export function listV2Sessions(): Promise<V2StudySession[]> {
  return invoke<V2StudySession[]>('v2_list_sessions');
}

export function setV2SessionAllocations(
  input: V2SetSessionAllocationsInput,
): Promise<V2StudySession> {
  return invoke<V2StudySession>('v2_set_session_allocations', { input });
}

export function setV2SessionExams(input: V2SetSessionExamsInput): Promise<V2StudySession> {
  return invoke<V2StudySession>('v2_set_session_exams', { input });
}

export function getV2StudyTotals(): Promise<V2StudyTotals> {
  return invoke<V2StudyTotals>('v2_get_study_totals');
}

export type V2ConceptMasterySummary = {
  conceptId: number;
  masteryScore: number;
  confidence: number;
  evidenceCount: number;
  lastEvidenceAt: string | null;
  lastReviewedOn: string | null;
  lastRecalculatedAt: string;
  lastRecalcRunId: number;
  algorithmVersion: string | null;
};

export type V2ConceptEvidence = {
  id: number;
  sourceKind: string;
  sourceId: number | null;
  sourceVersion: number;
  examId: number | null;
  examCode: string | null;
  examShortName: string | null;
  localDate: string;
  occurredAt: string;
  rawValue: unknown;
  algorithmVersionId: number;
  algorithmVersion: string;
  notes: string;
  createdAt: string;
  isVoided: boolean;
  voidReason: string | null;
  voidedAt: string | null;
  replacementEvidenceId: number | null;
};

export type V2MasteryRunSummary = {
  id: number;
  reason: string;
  algorithmVersionId: number | null;
  algorithmVersion: string | null;
  evidenceCount: number;
  evidenceFingerprint: string;
  resultingScore: number;
  resultingConfidence: number;
  startedAt: string;
  finishedAt: string;
  notes: string;
};

export type V2ConceptSummary = {
  id: number;
  slug: string;
  title: string;
  description: string;
  domain: string;
  parentConceptId: number | null;
  parentSlug: string | null;
  parentTitle: string | null;
  difficulty: number;
  isArchived: boolean;
  mastery: V2ConceptMasterySummary | null;
  examCodes: string[];
  verifiedExamCodes: string[];
};

export type V2ConceptLink = {
  id: number;
  slug: string;
  title: string;
  domain: string;
  difficulty: number;
  isArchived: boolean;
};

export type V2ConceptSyllabusMapping = {
  examId: number;
  examCode: string;
  examName: string;
  examShortName: string;
  examIsActive: boolean;
  nodeId: number;
  nodeCode: string;
  nodeTitle: string;
  nodeKind: string;
  nodeCategory: string;
  nodeDisplayOrder: number;
  nodeIsArchived: boolean;
  nodePath: string[];
  conceptId: number;
  weight: number;
  isPrimary: boolean;
  notes: string;
  status: string;
  origin: string;
  sourceReference: string;
  provenance: string;
  contentVersionId: number | null;
  reviewedAt: string | null;
  reviewedBy: string;
  updatedAt: string;
  sourceTitle: string;
  sourcePublisher: string;
  sourceUrl: string | null;
  sourceVerifiedOn: string;
  contentVersionLabel: string;
  contentReviewStatus: string;
  contentVerifiedOn: string | null;
  isApproved: boolean;
  isVerified: boolean;
  progressStatus: string;
};

export type V2ConceptDetail = {
  concept: V2ConceptSummary;
  children: V2ConceptLink[];
  mappings: V2ConceptSyllabusMapping[];
  evidence: V2ConceptEvidence[];
  masteryHistory: V2MasteryRunSummary[];
};

export type V2ExamConceptCoverage = {
  examId: number;
  examCode: string;
  examName: string;
  examShortName: string;
  examIsActive: boolean;
  configured: boolean;
  status: string;
  contentVersionLabel: string | null;
  contentReviewStatus: string | null;
  contentVerifiedOn: string | null;
  nodeCount: number;
  topicNodeCount: number;
  revisedNodeCount: number;
  mappedConceptCount: number;
  verifiedMappedConceptCount: number;
};

export type V2ConceptSearchResult = {
  concepts: V2ConceptSummary[];
  total: number;
  domains: string[];
  examCoverage: V2ExamConceptCoverage[];
};

export type V2ConceptSearchFilter = {
  query?: string;
  domain?: string;
  examCode?: string;
  includeArchived?: boolean;
  limit?: number;
  offset?: number;
};

export type V2ConceptDetailInput = {
  slug: string;
  evidenceLimit?: number;
  historyLimit?: number;
};

export type V2MasteryUpdate = {
  conceptId: number;
  runId: number;
  mastery: V2ConceptMasterySummary;
};

export type V2UpdateConceptMasteryInput = {
  conceptId: number;
  targetScore: number;
  localDate: string;
  notes?: string;
};

export type V2ReplayConceptMasteryInput = {
  conceptId: number;
};

export type V2VerifyMasteryRunInput = {
  runId: number;
};

export type V2MasteryReplayVerification = {
  runId: number;
  matches: boolean;
  storedScore: number;
  recomputedScore: number;
  storedConfidence: number;
  recomputedConfidence: number;
  storedFingerprint: string;
  recomputedFingerprint: string;
  resultCount: number;
  mismatches: string[];
};

export function searchV2Concepts(
  filter: V2ConceptSearchFilter = {},
): Promise<V2ConceptSearchResult> {
  return invoke<V2ConceptSearchResult>('v2_search_concepts', { filter });
}

export function getV2ConceptDetail(input: V2ConceptDetailInput): Promise<V2ConceptDetail> {
  return invoke<V2ConceptDetail>('v2_get_concept_detail', { input });
}

export type V2ConceptMappingUpdateInput = {
  examId: number;
  nodeId: number;
  conceptId: number;
  status: 'proposed' | 'reviewed' | 'approved' | 'rejected';
  notes?: string;
};

export function updateV2ConceptMapping(
  input: V2ConceptMappingUpdateInput,
): Promise<V2ConceptDetail> {
  return invoke<V2ConceptDetail>('v2_update_concept_mapping', { input });
}

export function updateV2ConceptMastery(
  input: V2UpdateConceptMasteryInput,
): Promise<V2MasteryUpdate> {
  return invoke<V2MasteryUpdate>('v2_update_concept_mastery', { input });
}

export function replayV2ConceptMastery(
  input: V2ReplayConceptMasteryInput,
): Promise<V2MasteryUpdate> {
  return invoke<V2MasteryUpdate>('v2_replay_concept_mastery', { input });
}

export function verifyV2MasteryRun(
  input: V2VerifyMasteryRunInput,
): Promise<V2MasteryReplayVerification> {
  return invoke<V2MasteryReplayVerification>('v2_verify_mastery_run', { input });
}

export type V2AppSettings = {
  preparationStartDate: string | null;
  dailyStudyTargetMinutes: number;
};

export type V2UpdateAppSettingsInput = {
  preparationStartDate: string | null;
  dailyStudyTargetMinutes: number;
};

export type V2ExamProfile = {
  id: number;
  code: string;
  name: string;
  shortName: string;
  authority: string;
  officialUrl: string;
  displayOrder: number;
  isActive: boolean;
  examDate: string | null;
  dateUpdatedAt: string | null;
};

export type V2ExamActivationInput = {
  examId: number;
  isActive: boolean;
};

export type V2ExamDateChangeInput = {
  examId: number;
  examDate: string | null;
  reason?: string;
};

export type V2AffectedAnchoredTask = {
  id: number;
  title: string;
  currentDate: string;
  newDate: string;
};

export type V2ExamDateChangePreview = {
  examId: number;
  oldDate: string | null;
  newDate: string | null;
  affectedTasks: V2AffectedAnchoredTask[];
};

export function getV2AppSettings(): Promise<V2AppSettings> {
  return invoke<V2AppSettings>('v2_get_app_settings');
}

export function updateV2AppSettings(input: V2UpdateAppSettingsInput): Promise<V2AppSettings> {
  return invoke<V2AppSettings>('v2_update_app_settings', { input });
}

export function listV2ExamProfiles(): Promise<V2ExamProfile[]> {
  return invoke<V2ExamProfile[]>('v2_list_exam_profiles');
}

export function setV2ExamActive(input: V2ExamActivationInput): Promise<V2ExamProfile> {
  return invoke<V2ExamProfile>('v2_set_exam_active', { input });
}

export function previewV2ExamDateChange(
  input: V2ExamDateChangeInput,
): Promise<V2ExamDateChangePreview> {
  return invoke<V2ExamDateChangePreview>('v2_preview_exam_date_change', { input });
}

export function applyV2ExamDateChange(input: V2ExamDateChangeInput): Promise<V2ExamProfile> {
  return invoke<V2ExamProfile>('v2_apply_exam_date_change', { input });
}

export type V2CreateManualSessionInput = {
  sessionDate: string;
  durationSeconds: number;
  notes?: string;
  primaryExamId: number | null;
  /** Full-credit exam associations. Each selected exam receives the whole session. */
  examIds?: number[];
};

export type V2UpdateSessionInput = {
  sessionId: number;
  sessionDate: string;
  durationSeconds: number;
  notes?: string;
  primaryExamId: number | null;
};

export function createV2ManualSession(input: V2CreateManualSessionInput): Promise<V2StudySession> {
  return invoke<V2StudySession>('v2_create_manual_session', { input });
}

export function updateV2StudySession(input: V2UpdateSessionInput): Promise<V2StudySession> {
  return invoke<V2StudySession>('v2_update_study_session', { input });
}

export function deleteV2StudySession(sessionId: number): Promise<void> {
  return invoke<void>('v2_delete_study_session', { sessionId });
}

export type V2PlannerTask = {
  id: number;
  title: string;
  description: string;
  scheduledDate: string;
  deadlineDate: string | null;
  examId: number | null;
  examCode: string | null;
  conceptId: number | null;
  conceptTitle: string | null;
  syllabusNodeId: number | null;
  priority: string;
  status: string;
  scheduleAnchor: string;
  scheduleOffsetDays: number | null;
  completedAt: string | null;
  linkedSessionId: number | null;
  createdAt: string;
  updatedAt: string;
};

export type V2PlannerTaskInput = {
  id?: number | null;
  title: string;
  description?: string;
  scheduledDate: string;
  deadlineDate?: string | null;
  examId?: number | null;
  conceptId?: number | null;
  syllabusNodeId?: number | null;
  priority?: string;
  scheduleAnchor?: string;
  scheduleOffsetDays?: number | null;
  linkedSessionId?: number | null;
};

export type V2PlannerBlock = {
  id: number;
  title: string;
  examId: number | null;
  examCode: string | null;
  conceptId: number | null;
  conceptTitle: string | null;
  syllabusNodeId: number | null;
  calendarDate: string;
  startTime: string;
  endTime: string;
  status: string;
  notes: string;
  linkedSessionId: number | null;
  createdAt: string;
  updatedAt: string;
};

export type V2PlannerBlockInput = {
  id?: number | null;
  title: string;
  examId?: number | null;
  conceptId?: number | null;
  syllabusNodeId?: number | null;
  calendarDate: string;
  startTime: string;
  endTime: string;
  status?: string;
  notes?: string;
  linkedSessionId?: number | null;
};

export type V2PlannerDay = {
  date: string;
  tasks: V2PlannerTask[];
  blocks: V2PlannerBlock[];
};

export type V2TaskStatusInput = { taskId: number; status: string };
export type V2RescheduleTaskInput = { taskId: number; scheduledDate: string };
export type V2BlockStatusInput = { blockId: number; status: string };
export type V2LinkBlockSessionInput = { blockId: number; sessionId: number | null };

export function getV2PlannerDay(date: string): Promise<V2PlannerDay> {
  return invoke<V2PlannerDay>('v2_get_planner_day', { input: { date } });
}

export function getV2Agenda(date: string): Promise<V2PlannerDay> {
  return invoke<V2PlannerDay>('v2_get_agenda', { input: { date } });
}

export function saveV2Task(input: V2PlannerTaskInput): Promise<V2PlannerTask> {
  return invoke<V2PlannerTask>('v2_save_task', { input });
}

export function setV2TaskStatus(input: V2TaskStatusInput): Promise<V2PlannerTask> {
  return invoke<V2PlannerTask>('v2_set_task_status', { input });
}

export function rescheduleV2Task(input: V2RescheduleTaskInput): Promise<V2PlannerTask> {
  return invoke<V2PlannerTask>('v2_reschedule_task', { input });
}

export function deleteV2Task(taskId: number): Promise<void> {
  return invoke<void>('v2_delete_task', { taskId });
}

export function saveV2Block(input: V2PlannerBlockInput): Promise<V2PlannerBlock> {
  return invoke<V2PlannerBlock>('v2_save_block', { input });
}

export function setV2BlockStatus(input: V2BlockStatusInput): Promise<V2PlannerBlock> {
  return invoke<V2PlannerBlock>('v2_set_block_status', { input });
}

export function linkV2BlockSession(input: V2LinkBlockSessionInput): Promise<V2PlannerBlock> {
  return invoke<V2PlannerBlock>('v2_link_block_session', { input });
}

export function deleteV2Block(blockId: number): Promise<void> {
  return invoke<void>('v2_delete_block', { blockId });
}

export type V2ResourceProgress = {
  studyStatus: string;
  personalNotes: string;
  lastOpenedAt: string | null;
  updatedAt: string;
};

export type V2ResourceExamLink = {
  examId: number;
  examCode: string;
  examShortName: string;
  category: string;
  syllabusNodeId: number | null;
  priority: string;
  notes: string;
};

export type V2ResourceFile = {
  fileName: string;
  sha256: string;
  sizeBytes: number;
  storageMode: string;
  status: string;
  importedAt: string;
  lastVerifiedAt: string | null;
  missingSince: string | null;
};

export type V2Resource = {
  id: number;
  title: string;
  description: string;
  subjectArea: string;
  url: string | null;
  resourceType: string;
  source: string;
  isArchived: boolean;
  createdAt: string;
  updatedAt: string;
  progress: V2ResourceProgress;
  conceptIds: number[];
  examLinks: V2ResourceExamLink[];
  file: V2ResourceFile | null;
};

export type V2ResourceExamLinkInput = {
  examId: number;
  category: string;
  syllabusNodeId?: number | null;
  priority?: string;
  notes?: string;
};

export type V2ResourceInput = {
  id?: number | null;
  title: string;
  description?: string;
  subjectArea?: string;
  url?: string | null;
  resourceType: string;
  source?: string;
  conceptIds?: number[];
  examLinks?: V2ResourceExamLinkInput[];
};

export type V2ResourceSearchFilter = {
  query?: string;
  examCode?: string;
  includeArchived?: boolean;
};

export type V2ResourceProgressInput = {
  resourceId: number;
  studyStatus: string;
  personalNotes?: string;
};

export type V2ResourceExamLinkCommandInput = {
  resourceId: number;
  link: V2ResourceExamLinkInput;
};

export type V2ResourceArchiveInput = { resourceId: number; archived: boolean };
export type V2ResourceFileInput = {
  resourceId: number;
  filePath: string;
  storageMode?: 'managed' | 'external';
};

export function listV2Resources(filter: V2ResourceSearchFilter = {}): Promise<V2Resource[]> {
  return invoke<V2Resource[]>('v2_list_resources', { filter });
}

export function saveV2Resource(input: V2ResourceInput): Promise<V2Resource> {
  return invoke<V2Resource>('v2_save_resource', { input });
}

export function archiveV2Resource(input: V2ResourceArchiveInput): Promise<V2Resource> {
  return invoke<V2Resource>('v2_archive_resource', { input });
}

export function updateV2ResourceProgress(input: V2ResourceProgressInput): Promise<V2Resource> {
  return invoke<V2Resource>('v2_update_resource_progress', { input });
}

export function linkV2ResourceExam(input: V2ResourceExamLinkCommandInput): Promise<V2Resource> {
  return invoke<V2Resource>('v2_link_resource_exam', { input });
}

export function importV2ResourceFile(input: V2ResourceFileInput): Promise<V2Resource> {
  return invoke<V2Resource>('v2_import_resource_file', { input });
}

export function validateV2ResourcePdf(filePath: string): Promise<void> {
  return invoke<void>('v2_validate_resource_pdf', { filePath });
}

export function removeV2ResourceFile(resourceId: number): Promise<V2Resource> {
  return invoke<V2Resource>('v2_remove_resource_file', { resourceId });
}

export function openV2ResourceFile(resourceId: number): Promise<void> {
  return invoke<void>('v2_open_resource_file', { resourceId });
}

export type V2BackupSettings = {
  automaticEnabled: boolean;
  frequency: string;
  destination: string | null;
  retentionCount: number;
  lastSuccessAt: string | null;
  lastSuccessFile: string | null;
  lastFailureAt: string | null;
  lastFailureMessage: string | null;
};

export type V2BackupSettingsInput = {
  automaticEnabled: boolean;
  frequency: string;
  destination: string | null;
  retentionCount: number;
};

export type V2BackupFile = {
  fileName: string;
  path: string;
  sizeBytes: number;
  modifiedAt: string;
};

export type V2BackupOverview = {
  settings: V2BackupSettings;
  defaultDirectory: string;
  schemaVersion: number;
  databasePath: string;
  files: V2BackupFile[];
};

export type V2BackupRecord = {
  fileName: string;
  path: string;
  sizeBytes: number;
  createdAt: string;
  schemaVersion: number;
  examCount: number;
  removedOldBackups: number;
};

export type V2RestorePreview = {
  path: string;
  fileName: string;
  sizeBytes: number;
  schemaVersion: number;
  targetSchemaVersion: number;
  requiresUpgrade: boolean;
  examCount: number;
  integrityCheck: string;
};

export type V2RestoreResult = {
  restoredFrom: string;
  safetyBackup: string;
  schemaVersion: number;
  examCount: number;
  integrityCheck: string;
};

export type V2RestoreRecovery = {
  safetyDirectory: string;
  files: V2BackupFile[];
};

export function getV2BackupOverview(): Promise<V2BackupOverview> {
  return invoke<V2BackupOverview>('v2_get_backup_overview');
}

export function updateV2BackupSettings(input: V2BackupSettingsInput): Promise<V2BackupSettings> {
  return invoke<V2BackupSettings>('v2_update_backup_settings', { input });
}

export function createV2ManualBackup(): Promise<V2BackupRecord> {
  return invoke<V2BackupRecord>('v2_create_manual_backup');
}

export function runV2AutomaticBackupIfDue(): Promise<V2BackupRecord | null> {
  return invoke<V2BackupRecord | null>('v2_run_automatic_backup_if_due');
}

export function previewV2RestoreBackup(path: string): Promise<V2RestorePreview> {
  return invoke<V2RestorePreview>('v2_preview_restore_backup', { path });
}

export function applyV2RestoreBackup(path: string): Promise<V2RestoreResult> {
  return invoke<V2RestoreResult>('v2_apply_restore_backup', { input: { path } });
}

export function getV2RestoreRecovery(): Promise<V2RestoreRecovery> {
  return invoke<V2RestoreRecovery>('v2_get_restore_recovery');
}

export function restoreV2RecoverySafety(path: string): Promise<V2RestoreResult> {
  return invoke<V2RestoreResult>('v2_restore_recovery_safety', { input: { path } });
}

export type V2SyllabusNode = {
  id: number;
  examId: number;
  parentId: number | null;
  code: string;
  title: string;
  nodeKind: string;
  category: string;
  displayOrder: number;
  readinessWeight: number;
  isArchived: boolean;
  sourceTitle: string;
  sourceVerifiedOn: string;
  sourceUrl: string | null;
  sourceReference: string;
  contentVersionLabel: string;
  contentReviewStatus: string;
  progressStatus: string;
  priority: string;
  confidence: number;
  notes: string;
  nodePath: string[];
};

export type V2SyllabusSummary = {
  nodeCount: number;
  topicCount: number;
  revisedOrConfidentCount: number;
  mappedConceptCount: number;
};

export type V2ExamSyllabus = {
  examId: number;
  examCode: string;
  examName: string;
  examIsActive: boolean;
  configured: boolean;
  contentVersionLabel: string | null;
  contentReviewStatus: string | null;
  nodes: V2SyllabusNode[];
  summary: V2SyllabusSummary;
};

export type V2SyllabusProgressInput = {
  examId: number;
  nodeId: number;
  status: string;
  priority?: string;
  confidence?: number;
  notes?: string;
};

export function getV2ExamSyllabus(examId: number): Promise<V2ExamSyllabus> {
  return invoke<V2ExamSyllabus>('v2_get_exam_syllabus', { input: { examId } });
}

export function updateV2SyllabusNodeProgress(
  input: V2SyllabusProgressInput,
): Promise<V2ExamSyllabus> {
  return invoke<V2ExamSyllabus>('v2_update_syllabus_node_progress', { input });
}

export type V2AnalyticsPoint = {
  label: string;
  seconds: number;
  sessionCount: number;
};

export type V2MasteryBucket = {
  label: string;
  conceptCount: number;
};

export type V2AnalyticsSubject = {
  examId: number;
  examCode: string;
  subjectTitle: string;
  allocatedSeconds: number;
  sessionCount: number;
};

export type V2AnalyticsTopic = {
  examId: number;
  examCode: string;
  subjectTitle: string;
  nodeId: number;
  nodeTitle: string;
  allocatedSeconds: number;
  sessionCount: number;
};

export type V2Analytics = {
  examId: number | null;
  rangeFrom: string;
  rangeTo: string;
  totalSeconds: number;
  sessionCount: number;
  allocatedSeconds: number;
  unallocatedSeconds: number;
  topicAllocatedSeconds: number;
  topicUnallocatedSeconds: number;
  activeDays: number;
  consistencyPercent: number;
  daily: V2AnalyticsPoint[];
  weekly: V2AnalyticsPoint[];
  subjects: V2AnalyticsSubject[];
  topics: V2AnalyticsTopic[];
  mappedConceptCount: number;
  evidencedConceptCount: number;
  averageMastery: number | null;
  masteryBuckets: V2MasteryBucket[];
  activeRevisionSchedules: number;
  dueRevisionCount: number;
  overdueRevisionCount: number;
  revisionCompletedLast30Days: number;
};

export function getV2Analytics(input: {
  examId: number | null;
  today: string;
  windowDays?: number;
}): Promise<V2Analytics> {
  return invoke<V2Analytics>('v2_get_analytics', { input });
}

export type V2RevisionSettings = {
  initialAgainDays: number;
  initialDifficultDays: number;
  initialGoodDays: number;
  initialEasyDays: number;
  growthMultiplier: number;
  minimumIntervalDays: number;
  maximumIntervalDays: number;
  includeWeekends: boolean;
  dailyRevisionTarget: number;
  remindersEnabled: boolean;
};

export type V2RevisionSchedule = {
  conceptId: number;
  conceptSlug: string;
  conceptTitle: string;
  conceptDomain: string;
  status: string;
  lastRevisionDate: string | null;
  nextRevisionDate: string;
  currentIntervalDays: number;
  completedRevisions: number;
  previousOutcome: string | null;
  notes: string;
  overdueDays: number;
  masteryScore: number | null;
  selectionReason: string;
  eligible: boolean;
  approvedExamCodes: string[];
  pendingExamCodes: string[];
  alreadyScheduled: boolean;
};

export type V2RevisionHistoryEntry = {
  id: number;
  conceptId: number;
  conceptTitle: string;
  triggerExamId: number | null;
  revisionDate: string;
  scheduledForDate: string;
  outcome: string;
  notes: string;
  intervalBeforeDays: number;
  intervalAfterDays: number;
  masteryBefore: number;
  masteryAfter: number;
};

export type V2RevisionStats = {
  dueToday: number;
  overdue: number;
  completedToday: number;
  completedLast7Days: number;
  totalCompleted: number;
};

export type V2RevisionDashboard = {
  settings: V2RevisionSettings;
  dueToday: V2RevisionSchedule[];
  overdue: V2RevisionSchedule[];
  upcoming: V2RevisionSchedule[];
  awaitingApproval: V2RevisionSchedule[];
  recentHistory: V2RevisionHistoryEntry[];
  stats: V2RevisionStats;
  examId: number | null;
};

export type V2RevisionPlanItem = {
  planItemId: number;
  conceptId: number;
  conceptTitle: string;
  conceptDomain: string;
  membershipStatus: string;
  lane: string;
  triageScore: number;
  estimatedMinutes: number;
  sourceDueDate: string;
  position: number;
  plannedBlockId: number | null;
  plannedBlockTitle: string | null;
  selectionReason: string;
};

export type V2RevisionDailyPlan = {
  id: number;
  planDate: string;
  examId: number | null;
  version: number;
  effectiveBudgetMinutes: number;
  maxTopics: number;
  plannedMinutes: number;
  completedMinutes: number;
  status: string;
  items: V2RevisionPlanItem[];
};

export function getV2RevisionDashboard(input: {
  examId: number | null;
  today: string;
}): Promise<V2RevisionDashboard> {
  return invoke<V2RevisionDashboard>('v2_get_revision_dashboard', { input });
}

export function updateV2RevisionSettings(
  input: Omit<V2RevisionSettings, 'remindersEnabled'> & { remindersEnabled?: boolean },
): Promise<V2RevisionSettings> {
  return invoke<V2RevisionSettings>('v2_update_revision_settings', { input });
}

export function scheduleV2RevisionConcept(input: {
  conceptId: number;
  nextRevisionDate?: string | null;
  notes?: string;
}): Promise<V2RevisionSchedule> {
  return invoke<V2RevisionSchedule>('v2_schedule_revision_concept', { input });
}

export function rescheduleV2Revision(input: {
  conceptId: number;
  nextRevisionDate: string;
}): Promise<V2RevisionSchedule> {
  return invoke<V2RevisionSchedule>('v2_reschedule_revision', { input });
}

export function setV2RevisionStatus(input: {
  conceptId: number;
  status: string;
}): Promise<V2RevisionSchedule> {
  return invoke<V2RevisionSchedule>('v2_set_revision_status', { input });
}

export function completeV2Revision(input: {
  conceptId: number;
  outcome: string;
  revisionDate: string;
  triggerExamId?: number | null;
  notes?: string;
  planItemId?: number | null;
  sessionId?: number | null;
}): Promise<V2RevisionDashboard> {
  return invoke<V2RevisionDashboard>('v2_complete_revision', { input });
}

export function deferV2Revision(input: {
  conceptId: number;
  newRevisionDate: string;
  reason: string;
  notes?: string;
  planItemId?: number | null;
}): Promise<V2RevisionDashboard> {
  return invoke<V2RevisionDashboard>('v2_defer_revision', { input });
}

export function getV2RevisionDailyPlan(input: {
  planDate: string;
  examId: number | null;
}): Promise<V2RevisionDailyPlan | null> {
  return invoke<V2RevisionDailyPlan | null>('v2_get_revision_daily_plan', { input });
}

export function generateV2RevisionDailyPlan(input: {
  planDate: string;
  examId: number | null;
}): Promise<V2RevisionDailyPlan> {
  return invoke<V2RevisionDailyPlan>('v2_generate_revision_daily_plan', { input });
}

export function rebalanceV2RevisionDailyPlan(input: {
  planDate: string;
  examId: number | null;
}): Promise<V2RevisionDailyPlan> {
  return invoke<V2RevisionDailyPlan>('v2_rebalance_revision_daily_plan', { input });
}

export function moveV2RevisionPlanItem(input: {
  planItemId: number;
  target: string;
  planDate: string;
  examId: number | null;
}): Promise<V2RevisionDailyPlan | null> {
  return invoke<V2RevisionDailyPlan | null>('v2_move_revision_plan_item', { input });
}

export function linkV2RevisionPlanBlock(input: {
  planItemId: number;
  blockId: number;
  planDate: string;
  examId: number | null;
}): Promise<V2RevisionDailyPlan | null> {
  return invoke<V2RevisionDailyPlan | null>('v2_link_revision_plan_block', { input });
}

export type V2DashboardSection = {
  sectionKey: string;
  title: string;
  visible: boolean;
  displayOrder: number;
};

export function getV2DashboardSections(
  examId: number | null = null,
): Promise<V2DashboardSection[]> {
  return invoke<V2DashboardSection[]>('v2_get_dashboard_sections', {
    input: { examId },
  });
}

export type V2QuickStartAllocation = {
  examId: number;
  examCode: string;
  examName: string;
  nodeId: number;
  nodeTitle: string;
  subjectTitle: string;
  allocatedMinutes: number;
};

export type V2QuickStartConfig = {
  id: number;
  name: string;
  durationMinutes: number;
  isPinned: boolean;
  createdAt: string;
  updatedAt: string;
  allocations: V2QuickStartAllocation[];
};

export type V2QuickStartAllocationInput = {
  examId: number;
  syllabusNodeId: number;
  allocatedMinutes: number;
};

export type V2QuickStartConfigInput = {
  id?: number | null;
  name: string;
  durationMinutes: number;
  isPinned?: boolean | null;
  allocations: V2QuickStartAllocationInput[];
};

export function listV2QuickStartConfigs(): Promise<V2QuickStartConfig[]> {
  return invoke<V2QuickStartConfig[]>('v2_list_quick_start_configs');
}

export function saveV2QuickStartConfig(
  input: V2QuickStartConfigInput,
): Promise<V2QuickStartConfig[]> {
  return invoke<V2QuickStartConfig[]>('v2_save_quick_start_config', { input });
}

export function deleteV2QuickStartConfig(input: { id: number }): Promise<V2QuickStartConfig[]> {
  return invoke<V2QuickStartConfig[]>('v2_delete_quick_start_config', { input });
}

export type V2GlobalSyllabusHit = {
  nodeId: number;
  examId: number;
  examCode: string;
  examShortName: string;
  examIsActive: boolean;
  title: string;
  nodeCode: string;
  nodeKind: string;
  breadcrumb: string;
  progressStatus: string;
};

export type V2GlobalConceptHit = {
  conceptId: number;
  slug: string;
  title: string;
  domain: string;
  masteryScore: number | null;
  evidenceCount: number;
  examCodes: string[];
};

export type V2GlobalPyqHit = {
  questionId: number;
  examId: number;
  examCode: string;
  examShortName: string;
  year: number;
  sessionLabel: string;
  questionNumber: string;
  sectionTitle: string;
  statementSnippet: string;
};

export type V2GlobalResourceHit = {
  resourceId: number;
  title: string;
  resourceType: string;
  subjectArea: string;
  examCodes: string[];
};

export type V2GlobalFlashcardHit = {
  flashcardId: number;
  frontSnippet: string;
  backSnippet: string;
  subjectArea: string;
  examCodes: string[];
};

export type V2GlobalSearchResult = {
  query: string;
  examId: number | null;
  syllabus: V2GlobalSyllabusHit[];
  concepts: V2GlobalConceptHit[];
  pyqs: V2GlobalPyqHit[];
  resources: V2GlobalResourceHit[];
  flashcards: V2GlobalFlashcardHit[];
};

export function globalSearchV2(input: {
  query: string;
  examId: number | null;
  limitPerCategory?: number;
}): Promise<V2GlobalSearchResult> {
  return invoke<V2GlobalSearchResult>('v2_global_search', { input });
}

export function saveV2DashboardSections(input: {
  examId: number | null;
  sections: Array<{ sectionKey: string; visible: boolean; displayOrder: number }>;
}): Promise<V2DashboardSection[]> {
  return invoke<V2DashboardSection[]>('v2_save_dashboard_sections', { input });
}

export function resetV2DashboardSections(
  examId: number | null = null,
): Promise<V2DashboardSection[]> {
  return invoke<V2DashboardSection[]>('v2_reset_dashboard_sections', {
    input: { examId },
  });
}

export type V2PracticeExamOption = {
  id: number;
  code: string;
  name: string;
  shortName: string;
  isActive: boolean;
};

export type V2PracticePatternSection = {
  id: number;
  sectionId: number;
  code: string;
  title: string;
  displayOrder: number;
  questionCount: number;
  maxAttempts: number | null;
  maxMarks: number;
  correctMarks: number;
  incorrectMarks: number;
  unansweredMarks: number;
  questionTypes: string[];
  markingVariants: Array<Record<string, unknown>>;
  numericTolerance: number;
  partialRule: string;
  notes: string;
};

export type V2PracticePattern = {
  id: number;
  patternCode: string;
  patternTitle: string;
  variant: string;
  versionLabel: string;
  verificationStatus: 'verified' | 'pending' | 'rejected';
  effectiveFrom: string | null;
  durationMinutes: number | null;
  totalMarks: number;
  totalQuestions: number;
  attemptLimit: number | null;
  sourceTitle: string;
  sourceUrl: string | null;
  sourceReference: string;
  verifiedOn: string | null;
  notes: string;
  sections: V2PracticePatternSection[];
};

export type V2PracticeSectionOption = {
  id: number;
  code: string;
  title: string;
  defaultQuestionTypeId: number | null;
};

export type V2PracticeQuestionTypeOption = {
  id: number;
  code: string;
  title: string;
  answerFormat: string;
};

export type V2PracticeTopicOption = {
  id: number;
  code: string;
  title: string;
  nodeKind: string;
  category: string;
  parentId: number | null;
  displayOrder: number;
};

export type V2PracticeConceptOption = {
  id: number;
  slug: string;
  title: string;
  domain: string;
};

export type V2PracticeResourceOption = {
  id: number;
  title: string;
  resourceType: string;
};

export type V2PracticeOptions = {
  exam: V2PracticeExamOption;
  patterns: V2PracticePattern[];
  sections: V2PracticeSectionOption[];
  questionTypes: V2PracticeQuestionTypeOption[];
  topics: V2PracticeTopicOption[];
  concepts: V2PracticeConceptOption[];
  resources: V2PracticeResourceOption[];
};

export type V2QuestionOption = {
  key: string;
  text: string;
};

export type V2PyqQuestionSummary = {
  id: number;
  examId: number;
  year: number;
  sessionLabel: string;
  questionNumber: string;
  sectionId: number;
  sectionCode: string;
  sectionTitle: string;
  questionTypeId: number;
  questionTypeCode: string;
  patternVersionId: number | null;
  patternVersionLabel: string | null;
  statement: string;
  maxMarks: number;
  negativeMarks: number;
  unansweredMarks: number;
  difficulty: number;
  sourceTitle: string;
  sourcePage: string;
  sourceUrl: string | null;
  answerKeySource: string;
  answerKeyVersion: string;
  answerKeyVerified: boolean;
  verificationStatus: string;
  numericTolerance: number;
  correctRange: string;
  officialExplanation: string;
  thirdPartySolution: string;
  userNote: string;
  resourceId: number | null;
  isArchived: boolean;
  attemptCount: number;
  conceptCount: number;
  syllabusNodeId: number | null;
  syllabusNodeTitle: string | null;
  topicIds: number[];
  options: V2QuestionOption[];
  sourceId: number | null;
  answerKeyId: number | null;
};

export type V2PyqQuestionTopic = {
  nodeId: number;
  nodeCode: string;
  nodeTitle: string;
  isPrimary: boolean;
};

export type V2PyqQuestionConcept = {
  conceptId: number;
  slug: string;
  title: string;
  isPrimary: boolean;
};

export type V2PyqAttempt = {
  id: number;
  examId: number;
  questionId: number;
  questionNumber: string;
  questionStatement: string;
  attemptDate: string;
  selectedAnswer: string;
  status: string;
  marksObtained: number;
  maxMarksAtAttempt: number;
  negativeMarksAtAttempt: number;
  unansweredMarksAtAttempt: number;
  timeSpentSeconds: number;
  confidence: number | null;
  mistakes: string;
  reviewStatus: string;
  notes: string;
  patternVersionId: number | null;
  answerKeyVersion: string;
  correctAnswerAtAttempt: string;
  scoringEngine: string;
};

export type V2PyqQuestionDetail = {
  question: V2PyqQuestionSummary;
  correctAnswer: string;
  topics: V2PyqQuestionTopic[];
  concepts: V2PyqQuestionConcept[];
  attempts: V2PyqAttempt[];
};

export type V2PyqFilter = {
  examId: number;
  year?: number | null;
  sectionId?: number | null;
  questionTypeId?: number | null;
  syllabusNodeId?: number | null;
  verificationStatus?: string | null;
  query?: string | null;
  includeArchived?: boolean;
  limit?: number;
  offset?: number;
};

export type V2PyqQuestionInput = {
  id?: number | null;
  examId: number;
  year: number;
  sessionLabel: string;
  sectionId: number;
  questionTypeId: number;
  questionNumber: string;
  statement: string;
  correctAnswer: string;
  correctRange: string;
  maxMarks: number;
  negativeMarks: number;
  unansweredMarks: number;
  difficulty: number;
  sourceTitle: string;
  sourcePage: string;
  sourceUrl?: string | null;
  answerKeySource: string;
  answerKeyVersion: string;
  answerKeyVerified: boolean;
  verificationStatus: string;
  numericTolerance: number;
  options: V2QuestionOption[];
  patternVersionId?: number | null;
  syllabusNodeId?: number | null;
  topicIds: number[];
  conceptIds: number[];
  resourceId?: number | null;
  sourceId?: number | null;
  answerKeyId?: number | null;
  dedupeKey: string;
  officialExplanation: string;
  thirdPartySolution: string;
  userNote: string;
};

export type V2PyqAttemptInput = {
  examId: number;
  questionId: number;
  attemptDate: string;
  selectedAnswer: string;
  timeSpentSeconds: number;
  confidence?: number | null;
  mistakes: string;
  reviewStatus: string;
  notes: string;
};

export type V2PracticeImportPreview = {
  preview: {
    totalRows: number;
    validRows: number;
    duplicateRows: number;
    invalidRows: number;
    rows: Array<{
      rowNumber: number;
      status: string;
      summary: string;
      errors: Array<{ field: string; message: string }>;
    }>;
  };
  headers: string[];
  template: string;
};

export type V2PracticeImportResult = {
  result: {
    imported: number;
    skipped: number;
    rejected: number;
    rows: Array<{
      rowNumber: number;
      status: string;
      summary: string;
      errors: Array<{ field: string; message: string }>;
    }>;
  };
};

export type V2MockSection = {
  id: number;
  sectionId: number;
  patternSectionId: number | null;
  sectionCode: string;
  sectionTitle: string;
  questionTypeId: number | null;
  questionTypeCode: string | null;
  questionCount: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  maxMarks: number;
  marksObtained: number;
  correctMarks: number;
  incorrectMarks: number;
  unansweredMarks: number;
  timeSpentMinutes: number | null;
  notes: string;
  sectionSnapshotJson: string;
};

export type V2MockScoreVersion = {
  version: number;
  maxMarks: number;
  marksObtained: number;
  percentage: number | null;
  recordedAt: string;
  reason: string;
  sectionsJson: string;
  markingContextJson: string;
};

export type V2MockTest = {
  id: number;
  examId: number;
  patternVersionId: number | null;
  patternVersionLabel: string | null;
  patternTitle: string | null;
  focusNodeId: number | null;
  focusNodeTitle: string | null;
  testName: string;
  testDate: string;
  durationMinutes: number;
  testMode: string;
  maxMarks: number;
  marksObtained: number | null;
  percentage: number | null;
  status: string;
  notes: string;
  reviewObservations: string;
  isArchived: boolean;
  availableQuestions: number;
  attemptedQuestions: number;
  correctQuestions: number;
  incorrectQuestions: number;
  unansweredQuestions: number;
  startedAt: string | null;
  endedAt: string | null;
  schemeSnapshotJson: string;
  mockKind: 'aggregate' | 'question_level';
  deadlineAt: string | null;
  currentQuestionId: number | null;
  lastActivityAt: string | null;
  submissionKind: string;
  sections: V2MockSection[];
  scoreHistory: V2MockScoreVersion[];
};

export type V2MockSectionInput = {
  patternSectionId: number;
  questionCount: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  maxMarks?: number | null;
  marksObtained?: number | null;
  timeSpentMinutes?: number | null;
  notes: string;
};

export type V2MockTestInput = {
  id?: number | null;
  examId: number;
  patternVersionId: number;
  testName: string;
  testDate: string;
  durationMinutes: number;
  testMode: string;
  focusNodeId?: number | null;
  notes: string;
  reviewObservations: string;
  sections: V2MockSectionInput[];
};

export type V2MockAnalytics = {
  completedCount: number;
  averagePercentage: number | null;
  bestPercentage: number | null;
  lastPercentage: number | null;
  byPattern: Array<{
    patternVersionId: number | null;
    patternVersionLabel: string;
    completedCount: number;
    averagePercentage: number | null;
  }>;
  history: Array<{
    testId: number;
    testDate: string;
    testName: string;
    percentage: number;
    patternVersionId: number | null;
    patternVersionLabel: string | null;
  }>;
  insufficientData: boolean;
};

export type V2PracticeAnalytics = {
  examId: number;
  pyqAttempts: number;
  distinctQuestions: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  accuracyPercentage: number | null;
  marksObtained: number;
  maxMarksAttempted: number;
  percentage: number | null;
  timeSpentSeconds: number;
  averageSecondsPerQuestion: number | null;
  verifiedQuestions: number;
  verifiedQuestionsInScope: number;
  totalQuestionsInScope: number;
  verificationCoveragePercentage: number | null;
  byYear: Array<{
    key: string;
    label: string;
    attempts: number;
    correct: number;
    incorrect: number;
    unanswered: number;
    accuracyPercentage: number | null;
    percentage: number | null;
  }>;
  bySection: Array<{
    key: string;
    label: string;
    attempts: number;
    correct: number;
    incorrect: number;
    unanswered: number;
    accuracyPercentage: number | null;
    percentage: number | null;
  }>;
  byTopic: Array<{
    nodeId: number | null;
    label: string;
    attempts: number;
    correct: number;
    incorrect: number;
    unanswered: number;
    accuracyPercentage: number | null;
  }>;
  pyqTrend: Array<{ date: string; attempts: number; accuracyPercentage: number | null }>;
  mocks: V2MockAnalytics;
  insufficientPyqData: boolean;
};

export function getV2PracticeOptions(examId: number): Promise<V2PracticeOptions> {
  return invoke<V2PracticeOptions>('v2_get_practice_options', { input: { examId } });
}

export function listV2PyqQuestions(filter: V2PyqFilter): Promise<V2PyqQuestionSummary[]> {
  return invoke<V2PyqQuestionSummary[]>('v2_list_pyq_questions', { filter });
}

export function getV2PyqQuestionDetail(
  examId: number,
  questionId: number,
): Promise<V2PyqQuestionDetail> {
  return invoke<V2PyqQuestionDetail>('v2_get_pyq_question_detail', { examId, questionId });
}

export function saveV2PyqQuestion(input: V2PyqQuestionInput): Promise<V2PyqQuestionDetail> {
  return invoke<V2PyqQuestionDetail>('v2_save_pyq_question', { input });
}

export function archiveV2PyqQuestion(
  examId: number,
  questionId: number,
  archived: boolean,
): Promise<void> {
  return invoke<void>('v2_archive_pyq_question', { examId, questionId, archived });
}

export function recordV2PyqAttempt(input: V2PyqAttemptInput): Promise<V2PyqAttempt> {
  return invoke<V2PyqAttempt>('v2_record_pyq_attempt', { input });
}

export function listV2PyqAttempts(
  examId: number,
  questionId?: number | null,
): Promise<V2PyqAttempt[]> {
  return invoke<V2PyqAttempt[]>('v2_list_pyq_attempts', { examId, questionId });
}

export function previewV2PyqImport(csv: string): Promise<V2PracticeImportPreview> {
  return invoke<V2PracticeImportPreview>('v2_preview_pyq_import', { csv });
}

export function applyV2PyqImport(csv: string): Promise<V2PracticeImportResult> {
  return invoke<V2PracticeImportResult>('v2_apply_pyq_import', { csv });
}

export function listV2MockTests(examId: number, includeArchived = false): Promise<V2MockTest[]> {
  return invoke<V2MockTest[]>('v2_list_mock_tests', { examId, includeArchived });
}

export function saveV2MockTest(input: V2MockTestInput): Promise<V2MockTest> {
  return invoke<V2MockTest>('v2_save_mock_test', { input });
}

export function setV2MockTestStatus(
  examId: number,
  mockId: number,
  action: string,
): Promise<V2MockTest> {
  return invoke<V2MockTest>('v2_set_mock_test_status', { examId, mockId, action });
}

export function archiveV2MockTest(
  examId: number,
  mockId: number,
  archived: boolean,
): Promise<void> {
  return invoke<void>('v2_archive_mock_test', { examId, mockId, archived });
}

export function getV2PracticeAnalytics(input: {
  examId: number;
  fromDate?: string | null;
  toDate?: string | null;
  year?: number | null;
  sectionId?: number | null;
  syllabusNodeId?: number | null;
}): Promise<V2PracticeAnalytics> {
  return invoke<V2PracticeAnalytics>('v2_get_practice_analytics', { input });
}

export type V2QuestionMockCandidate = {
  question: V2PyqQuestionSummary;
  patternSectionId: number;
  patternSectionCode: string;
  patternSectionTitle: string;
};

export type V2QuestionMockConcept = {
  conceptId: number;
  slug: string;
  title: string;
};

export type V2QuestionMockQuestion = {
  id: number;
  questionId: number;
  displayOrder: number;
  sectionId: number;
  patternSectionId: number;
  sectionCode: string;
  sectionTitle: string;
  sectionOrder: number;
  questionTypeCode: string;
  questionNumber: string;
  topicId: number | null;
  topicTitle: string;
  statement: string;
  options: V2QuestionOption[];
  selectedAnswer: string;
  visited: boolean;
  markedForReview: boolean;
  notEvaluated: boolean;
  responseStatus: string;
  marksObtained: number;
  maxMarks: number;
  negativeMarks: number;
  unansweredMarks: number;
  numericTolerance: number;
  timeSpentSeconds: number;
  correctAnswer: string | null;
  correctRange: string | null;
  answerKeyVersion: string;
  officialExplanation: string;
  thirdPartySolution: string;
  userNote: string;
  sourceTitle: string;
  sourcePage: string;
  sourceUrl: string | null;
  difficulty: number;
  year: number;
  sessionLabel: string;
  concepts: V2QuestionMockConcept[];
};

export type V2QuestionMockSection = {
  id: number;
  sectionId: number;
  patternSectionId: number | null;
  sectionCode: string;
  sectionTitle: string;
  displayOrder: number;
  maxAttempts: number | null;
  questionCount: number;
  answered: number;
  marked: number;
  notVisited: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  maxMarks: number;
  marksObtained: number;
  timeSpentSeconds: number;
  averageTimeSeconds: number | null;
  accuracyPercentage: number | null;
  correctMarks: number;
  incorrectMarks: number;
  unansweredMarks: number;
};

export type V2QuestionMockTopic = {
  topicId: number;
  topicTitle: string;
  questions: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  accuracyPercentage: number | null;
  averageTimeSeconds: number | null;
};

export type V2QuestionMockCompatibleAttempt = {
  mockId: number;
  testDate: string;
  testName: string;
  marksObtained: number | null;
  maxMarks: number;
  percentage: number | null;
  current: boolean;
};

export type V2QuestionMockSession = {
  mockId: number;
  examId: number;
  patternVersionId: number | null;
  patternVersionLabel: string | null;
  patternTitle: string | null;
  testName: string;
  testDate: string;
  durationMinutes: number;
  testMode: string;
  status: string;
  mockKind: string;
  startedAt: string | null;
  deadlineAt: string | null;
  endedAt: string | null;
  serverNow: string;
  remainingSeconds: number | null;
  currentQuestionId: number | null;
  lastActivityAt: string | null;
  submissionKind: string;
  maxMarks: number;
  marksObtained: number | null;
  percentage: number | null;
  availableQuestions: number;
  attemptedQuestions: number;
  correctQuestions: number;
  incorrectQuestions: number;
  unansweredQuestions: number;
  totalTimeSeconds: number;
  averageTimeSeconds: number | null;
  questions: V2QuestionMockQuestion[];
  sections: V2QuestionMockSection[];
  topics: V2QuestionMockTopic[];
  compatibleAttempts: V2QuestionMockCompatibleAttempt[];
  schemeSnapshotJson: string;
  notes: string;
  reviewObservations: string;
  isArchived: boolean;
};

export type V2QuestionMockSaveResult = {
  saved: boolean;
  status: string;
  autoSubmitted: boolean;
  serverNow: string;
  remainingSeconds: number | null;
  currentQuestionId: number | null;
  answeredQuestions: number;
  markedQuestions: number;
  notVisitedQuestions: number;
};

export type V2StartQuestionMockInput = {
  examId: number;
  patternVersionId: number;
  testName: string;
  testDate: string;
  testMode: string;
  durationMinutes?: number | null;
  focusNodeId?: number | null;
  notes: string;
  questionIds: number[];
};

export type V2SaveQuestionMockInput = {
  examId: number;
  mockId: number;
  mockQuestionId?: number | null;
  selectedAnswer?: string | null;
  markedForReview?: boolean | null;
  currentQuestionId?: number | null;
};

export type V2SubmitQuestionMockInput = {
  examId: number;
  mockId: number;
  confirmation: boolean;
  automatic: boolean;
};

export function listV2QuestionMockCandidates(
  examId: number,
  patternVersionId: number,
): Promise<V2QuestionMockCandidate[]> {
  return invoke<V2QuestionMockCandidate[]>('v2_list_question_mock_candidates', {
    examId,
    patternVersionId,
  });
}

export function startV2QuestionMock(
  input: V2StartQuestionMockInput,
): Promise<V2QuestionMockSession> {
  return invoke<V2QuestionMockSession>('v2_start_question_mock', { input });
}

export function getV2QuestionMockSession(
  examId: number,
  mockId: number,
): Promise<V2QuestionMockSession> {
  return invoke<V2QuestionMockSession>('v2_get_question_mock_session', {
    examId,
    mockId,
  });
}

export function saveV2QuestionMockState(
  input: V2SaveQuestionMockInput,
): Promise<V2QuestionMockSaveResult> {
  return invoke<V2QuestionMockSaveResult>('v2_save_question_mock_state', {
    input,
  });
}

export function submitV2QuestionMock(
  input: V2SubmitQuestionMockInput,
): Promise<V2QuestionMockSession> {
  return invoke<V2QuestionMockSession>('v2_submit_question_mock', { input });
}

export type V2PyqSource = {
  id: number;
  examId: number;
  year: number;
  paperCode: string;
  sourceType: 'question_paper' | 'answer_key' | 'supporting';
  authority: string;
  institution: string;
  title: string;
  sourceUrl: string | null;
  resourceId: number | null;
  sha256: string | null;
  sizeBytes: number | null;
  pageCount: number | null;
  publishedAt: string | null;
  acquiredAt: string | null;
  sessionLabel: string;
  documentVersion: string;
  keyStatus: string;
  verificationStatus: string;
  verificationNotes: string;
  associatedSourceId: number | null;
  resourceFileName: string | null;
  resourceFilePath: string | null;
  resourceFileStatus: string | null;
  createdAt: string;
  updatedAt: string;
};

export type V2PyqSourceInput = {
  id?: number | null;
  examId: number;
  year: number;
  paperCode: string;
  sourceType: 'question_paper' | 'answer_key' | 'supporting';
  authority?: string;
  institution?: string;
  title: string;
  sourceUrl?: string | null;
  resourceId?: number | null;
  pageCount?: number | null;
  publishedAt?: string | null;
  sessionLabel?: string;
  documentVersion?: string;
  keyStatus?: string;
  verificationStatus?: string | null;
  verificationNotes?: string;
  associatedSourceId?: number | null;
};

export type V2AcquireSourceInput = V2PyqSourceInput & {
  filePath: string;
  storageMode?: 'managed' | 'external';
};

export type V2PyqSourceAcquisition = {
  source: V2PyqSource;
  duplicate: boolean;
};

export type V2PyqSourceFilter = {
  examId?: number | null;
  year?: number | null;
  sourceType?: string | null;
};

export type V2SourceStatusInput = {
  sourceId: number;
  verificationStatus: string;
  verificationNotes?: string;
};

export type V2PyqAnswerKeyEntry = {
  id: number;
  answerKeyId: number;
  questionNumber: string;
  sessionLabel: string;
  questionTypeCode: string;
  sectionCode: string;
  keyValue: string;
  marks: number;
  keyStatus: string;
  officialQuestionId: string;
  notes: string;
};

export type V2PyqAnswerKey = {
  id: number;
  sourceId: number;
  examId: number;
  year: number;
  paperCode: string;
  sessionLabel: string;
  versionLabel: string;
  keyStatus: string;
  publishedAt: string | null;
  verifiedAt: string | null;
  verifiedBy: string;
  notes: string;
  supersedesKeyId: number | null;
  entryCount: number;
  createdAt: string;
  updatedAt: string;
  sourceTitle: string;
  sourceUrl: string | null;
  entries: V2PyqAnswerKeyEntry[];
};

export type V2ParsedAnswerKeyEntryInput = {
  questionNumber: string;
  sessionLabel?: string;
  questionTypeCode: string;
  sectionCode: string;
  keyValue: string;
  marks: number;
  keyStatus?: string;
  officialQuestionId?: string;
  notes?: string;
};

export type V2PyqAnswerKeyInput = {
  sourceId: number;
  versionLabel: string;
  keyStatus?: string;
  publishedAt?: string | null;
  verified?: boolean;
  verifiedBy?: string;
  notes?: string;
  supersedesKeyId?: number | null;
  text?: string;
  entries?: V2ParsedAnswerKeyEntryInput[];
};

export type V2ParsedKeyEntry = {
  questionNumber: string;
  sessionLabel: string;
  questionTypeCode: string;
  sectionCode: string;
  keyValue: string;
  marks: number;
  keyStatus: string;
};

export type V2AnswerKeyPreview = {
  entries: V2ParsedKeyEntry[];
  missingQuestionNumbers: number[];
  duplicateQuestionNumbers: string[];
  parsedCount: number;
};

export type V2ExtractionRun = {
  id: number;
  sourceId: number;
  answerKeyId: number | null;
  method: string;
  status: string;
  pagesProcessed: number;
  questionsFound: number;
  questionsReconciled: number;
  notes: string;
  startedAt: string;
  completedAt: string | null;
  createdAt: string;
};

export type V2ExtractionInput = {
  sourceId: number;
  text: string;
  method?: string;
  answerKeyId?: number | null;
  notes?: string;
};

export type V2ExtractionResult = {
  run: V2ExtractionRun;
  extracted: number;
  clean: number;
  needsReview: number;
  missingKey: number;
  skippedExisting: number;
};

export type V2ExtractionCandidate = {
  id: number;
  extractionRunId: number;
  sourceId: number;
  sourceTitle: string;
  sourceUrl: string | null;
  answerKeyId: number | null;
  answerKeyVersion: string | null;
  answerKeyEntryId: number | null;
  examId: number;
  year: number;
  paperCode: string;
  sessionLabel: string;
  pageNumber: number | null;
  questionNumber: string;
  sectionCode: string;
  questionTypeCode: string;
  statement: string;
  options: V2QuestionOption[];
  correctAnswer: string;
  correctRange: string;
  maxMarks: number | null;
  negativeMarks: number;
  unansweredMarks: number;
  numericTolerance: number;
  difficulty: number;
  syllabusNodeId: number | null;
  syllabusNodeTitle: string | null;
  conceptIds: number[];
  reviewStatus: string;
  reviewNotes: string;
  reviewedAt: string | null;
  reviewedBy: string;
  verifiedQuestionId: number | null;
  rawText: string;
  dedupeKey: string;
  createdAt: string;
  updatedAt: string;
};

export type V2ExtractionCandidateFilter = {
  examId?: number | null;
  year?: number | null;
  sourceId?: number | null;
  reviewStatus?: string | null;
  query?: string | null;
  limit?: number | null;
  offset?: number | null;
};

export type V2ExtractionCandidateUpdate = {
  candidateId: number;
  statement: string;
  options?: V2QuestionOption[];
  correctAnswer?: string;
  correctRange?: string;
  maxMarks: number;
  negativeMarks?: number;
  numericTolerance?: number;
  difficulty?: number;
  sectionCode?: string;
  questionTypeCode?: string;
  syllabusNodeId?: number | null;
  conceptIds?: number[];
  reviewNotes?: string;
  reviewedBy?: string;
};

export type V2CandidateDecisionInput = {
  candidateId: number;
  reviewedBy?: string;
  reviewNotes?: string;
};

export type V2AnswerKeyConflict = {
  id: number;
  questionId: number;
  oldAnswerKeyId: number | null;
  newAnswerKeyId: number | null;
  oldKeyValue: string;
  newKeyValue: string;
  status: string;
  notes: string;
  createdAt: string;
  resolvedAt: string | null;
};

export type V2SourceDetail = {
  source: V2PyqSource;
  answerKeys: V2PyqAnswerKey[];
  extractionRuns: V2ExtractionRun[];
  candidateCounts: Record<string, number>;
};

export type V2SourceCounts = {
  sources: number;
  answerKeys: number;
  keyEntries: number;
  extractionRuns: number;
  candidates: number;
  candidatesVerified: number;
  candidatesPending: number;
  candidatesRejected: number;
  verifiedQuestions: number;
  keyConflicts: number;
};

export function listV2PyqSources(filter: V2PyqSourceFilter = {}): Promise<V2PyqSource[]> {
  return invoke<V2PyqSource[]>('v2_list_pyq_sources', { filter });
}

export function getV2PyqSourceDetail(sourceId: number): Promise<V2SourceDetail> {
  return invoke<V2SourceDetail>('v2_get_pyq_source_detail', { sourceId });
}

export function saveV2PyqSource(input: V2PyqSourceInput): Promise<V2PyqSource> {
  return invoke<V2PyqSource>('v2_save_pyq_source', { input });
}

export function acquireV2PyqSource(input: V2AcquireSourceInput): Promise<V2PyqSourceAcquisition> {
  return invoke<V2PyqSourceAcquisition>('v2_acquire_pyq_source', { input });
}

export function setV2PyqSourceStatus(input: V2SourceStatusInput): Promise<V2PyqSource> {
  return invoke<V2PyqSource>('v2_set_pyq_source_status', { input });
}

export function previewV2PyqAnswerKey(text: string): Promise<V2AnswerKeyPreview> {
  return invoke<V2AnswerKeyPreview>('v2_preview_pyq_answer_key', { text });
}

export function listV2PyqAnswerKeys(
  examId: number | null = null,
  year: number | null = null,
): Promise<V2PyqAnswerKey[]> {
  return invoke<V2PyqAnswerKey[]>('v2_list_pyq_answer_keys', {
    examId,
    year,
  });
}

export function getV2PyqAnswerKey(answerKeyId: number): Promise<V2PyqAnswerKey> {
  return invoke<V2PyqAnswerKey>('v2_get_pyq_answer_key', { answerKeyId });
}

export function saveV2PyqAnswerKey(input: V2PyqAnswerKeyInput): Promise<V2PyqAnswerKey> {
  return invoke<V2PyqAnswerKey>('v2_save_pyq_answer_key', { input });
}

export function listV2PyqAnswerKeyConflicts(
  examId: number | null = null,
): Promise<V2AnswerKeyConflict[]> {
  return invoke<V2AnswerKeyConflict[]>('v2_list_pyq_answer_key_conflicts', {
    examId,
  });
}

export function extractV2PyqSource(input: V2ExtractionInput): Promise<V2ExtractionResult> {
  return invoke<V2ExtractionResult>('v2_extract_pyq_source', { input });
}

export function reconcileV2PyqSource(sourceId: number, answerKeyId: number): Promise<number> {
  return invoke<number>('v2_reconcile_pyq_source', {
    sourceId,
    answerKeyId,
  });
}

export function listV2PyqExtractionCandidates(
  filter: V2ExtractionCandidateFilter = {},
): Promise<V2ExtractionCandidate[]> {
  return invoke<V2ExtractionCandidate[]>('v2_list_pyq_extraction_candidates', {
    filter,
  });
}

export function getV2PyqExtractionCandidate(candidateId: number): Promise<V2ExtractionCandidate> {
  return invoke<V2ExtractionCandidate>('v2_get_pyq_extraction_candidate', {
    candidateId,
  });
}

export function updateV2PyqExtractionCandidate(
  input: V2ExtractionCandidateUpdate,
): Promise<V2ExtractionCandidate> {
  return invoke<V2ExtractionCandidate>('v2_update_pyq_extraction_candidate', {
    input,
  });
}

export function verifyV2PyqExtractionCandidate(
  input: V2CandidateDecisionInput,
): Promise<V2ExtractionCandidate> {
  return invoke<V2ExtractionCandidate>('v2_verify_pyq_extraction_candidate', {
    input,
  });
}

export function rejectV2PyqExtractionCandidate(
  input: V2CandidateDecisionInput,
): Promise<V2ExtractionCandidate> {
  return invoke<V2ExtractionCandidate>('v2_reject_pyq_extraction_candidate', {
    input,
  });
}

export function getV2PyqSourceCounts(): Promise<V2SourceCounts> {
  return invoke<V2SourceCounts>('v2_get_pyq_source_counts');
}

export type V2FlashcardExamLink = {
  examId: number;
  examCode: string;
  examShortName: string;
  syllabusNodeId: number | null;
  nodeTitle: string | null;
};

export type V2FlashcardSchedule = {
  dueOn: string;
  intervalDays: number;
  easeFactor: number;
  reviewCount: number;
  lapseCount: number;
  lastRating: string | null;
  lastReviewedOn: string | null;
  lastReviewedAt: string | null;
  algorithmVersion: number;
};

export type V2FlashcardSummary = {
  id: number;
  frontPreview: string;
  backPreview: string;
  subjectArea: string;
  conceptId: number | null;
  conceptTitle: string | null;
  isArchived: boolean;
  archivedAt: string | null;
  schedule: V2FlashcardSchedule;
  examLinks: V2FlashcardExamLink[];
  createdAt: string;
  updatedAt: string;
};

export type V2FlashcardReview = {
  id: number;
  reviewedOn: string;
  reviewedAt: string;
  rating: string;
  scheduledDueOn: string;
  nextDueOn: string;
  intervalBeforeDays: number;
  intervalAfterDays: number;
  easeBefore: number;
  easeAfter: number;
  algorithmVersion: number;
};

export type V2FlashcardDetail = {
  flashcard: V2FlashcardSummary;
  front: string;
  back: string;
  reviews: V2FlashcardReview[];
};

export type V2FlashcardFilter = {
  query?: string;
  examId?: number | null;
  subjectArea?: string;
  nodeId?: number | null;
  conceptSlug?: string;
  includeArchived?: boolean;
  dueOnly?: boolean;
  today: string;
  sort?: string;
  limit?: number;
  offset?: number;
};

export type V2FlashcardExamLinkInput = {
  examId: number;
  syllabusNodeId?: number | null;
};

export type V2FlashcardInput = {
  id?: number | null;
  front: string;
  back: string;
  subjectArea: string;
  conceptId?: number | null;
  examLinks: V2FlashcardExamLinkInput[];
  today: string;
};

export type V2FlashcardArchiveInput = {
  flashcardId: number;
  archived: boolean;
};

export type V2FlashcardLibrary = {
  cards: V2FlashcardSummary[];
  total: number;
  subjects: string[];
};

export type V2FlashcardQueueInput = {
  examId?: number | null;
  subjectArea?: string;
  nodeId?: number | null;
  conceptSlug?: string;
  dueOnly: boolean;
  today: string;
  limit?: number;
};

export type V2FlashcardReviewCard = {
  id: number;
  front: string;
  back: string;
  subjectArea: string;
  conceptTitle: string | null;
  schedule: V2FlashcardSchedule;
};

export type V2FlashcardQueue = {
  cards: V2FlashcardReviewCard[];
  remaining: number;
};

export type V2FlashcardReviewInput = {
  flashcardId: number;
  rating: 'again' | 'hard' | 'good' | 'easy';
  today: string;
};

export type V2FlashcardReviewResult = {
  review: V2FlashcardReview;
  schedule: V2FlashcardSchedule;
};

export type V2FlashcardStatsInput = {
  query?: string;
  examId?: number | null;
  subjectArea?: string;
  nodeId?: number | null;
  conceptSlug?: string;
  includeArchived?: boolean;
  today: string;
};

export type V2FlashcardStats = {
  totalCards: number;
  dueCards: number;
  reviewedCards: number;
  newCards: number;
  totalReviews: number;
  reviewsLast30Days: number;
  successfulLast30Days: number;
  retentionPercent: number | null;
};

export function listV2Flashcards(filter: V2FlashcardFilter): Promise<V2FlashcardLibrary> {
  return invoke<V2FlashcardLibrary>('v2_list_flashcards', { filter });
}

export function getV2FlashcardDetail(flashcardId: number): Promise<V2FlashcardDetail> {
  return invoke<V2FlashcardDetail>('v2_get_flashcard_detail', { flashcardId });
}

export function saveV2Flashcard(input: V2FlashcardInput): Promise<V2FlashcardDetail> {
  return invoke<V2FlashcardDetail>('v2_save_flashcard', { input });
}

export function setV2FlashcardArchived(input: V2FlashcardArchiveInput): Promise<V2FlashcardDetail> {
  return invoke<V2FlashcardDetail>('v2_set_flashcard_archived', { input });
}

export function getV2FlashcardReviewQueue(input: V2FlashcardQueueInput): Promise<V2FlashcardQueue> {
  return invoke<V2FlashcardQueue>('v2_get_flashcard_review_queue', { input });
}

export function reviewV2Flashcard(input: V2FlashcardReviewInput): Promise<V2FlashcardReviewResult> {
  return invoke<V2FlashcardReviewResult>('v2_review_flashcard', { input });
}

export function getV2FlashcardStats(input: V2FlashcardStatsInput): Promise<V2FlashcardStats> {
  return invoke<V2FlashcardStats>('v2_get_flashcard_stats', { input });
}

export type V2CustomFormulaVariable = {
  id: number;
  position: number;
  symbol: string;
  meaning: string;
  unit: string;
};

export type V2CustomFormula = {
  id: number;
  isBuiltin: boolean;
  name: string;
  latex: string;
  plainText: string;
  category: string;
  description: string;
  notes: string;
  source: string;
  createdAt: string;
  updatedAt: string;
  variables: V2CustomFormulaVariable[];
};

export type V2CustomFormulaVariableInput = {
  symbol: string;
  meaning: string;
  unit: string;
};

export type V2CustomFormulaInput = {
  id: number | null;
  name: string;
  latex: string;
  plainText: string;
  category: string;
  description: string;
  notes: string;
  source: string;
  variables: V2CustomFormulaVariableInput[];
};

export type V2CustomUnitSiClassification = 'si_base' | 'si_derived' | 'non_si';
export type V2CustomUnitConversionKind = 'none' | 'scalar' | 'contextual';

export type V2CustomUnit = {
  id: number;
  isBuiltin: boolean;
  name: string;
  symbol: string;
  quantity: string;
  dimension: string;
  siClassification: V2CustomUnitSiClassification;
  conversionKind: V2CustomUnitConversionKind;
  conversionFactor: string;
  conversionReferenceUnit: string;
  conversionNote: string;
  description: string;
  notes: string;
  source: string;
  createdAt: string;
  updatedAt: string;
};

export type V2CustomUnitInput = {
  id: number | null;
  name: string;
  symbol: string;
  quantity: string;
  dimension: string;
  siClassification: V2CustomUnitSiClassification;
  conversionKind: V2CustomUnitConversionKind;
  conversionFactor: string;
  conversionReferenceUnit: string;
  conversionNote: string;
  description: string;
  notes: string;
  source: string;
};

export type V2CustomConstant = {
  id: number;
  isBuiltin: boolean;
  name: string;
  symbol: string;
  valueText: string;
  unit: string;
  category: string;
  uncertaintyText: string;
  precisionText: string;
  source: string;
  description: string;
  notes: string;
  createdAt: string;
  updatedAt: string;
};

export type V2CustomConstantInput = {
  id: number | null;
  name: string;
  symbol: string;
  valueText: string;
  unit: string;
  category: string;
  uncertaintyText: string;
  precisionText: string;
  source: string;
  description: string;
  notes: string;
};

export function listV2CustomFormulas(): Promise<V2CustomFormula[]> {
  return invoke<V2CustomFormula[]>('v2_list_custom_formulas');
}

export function saveV2CustomFormula(input: V2CustomFormulaInput): Promise<V2CustomFormula> {
  return invoke<V2CustomFormula>('v2_save_custom_formula', { input });
}

export function deleteV2CustomFormula(id: number): Promise<void> {
  return invoke<void>('v2_delete_custom_formula', { id });
}

export function listV2CustomUnits(): Promise<V2CustomUnit[]> {
  return invoke<V2CustomUnit[]>('v2_list_custom_units');
}

export function saveV2CustomUnit(input: V2CustomUnitInput): Promise<V2CustomUnit> {
  return invoke<V2CustomUnit>('v2_save_custom_unit', { input });
}

export function deleteV2CustomUnit(id: number): Promise<void> {
  return invoke<void>('v2_delete_custom_unit', { id });
}

export function listV2CustomConstants(): Promise<V2CustomConstant[]> {
  return invoke<V2CustomConstant[]>('v2_list_custom_constants');
}

export function saveV2CustomConstant(input: V2CustomConstantInput): Promise<V2CustomConstant> {
  return invoke<V2CustomConstant>('v2_save_custom_constant', { input });
}

export function deleteV2CustomConstant(id: number): Promise<void> {
  return invoke<void>('v2_delete_custom_constant', { id });
}

export type V2BuiltinConstant = {
  id: number;
  stableKey: string;
  name: string;
  symbol: string;
  valueText: string;
  unit: string;
  uncertaintyText: string;
  precisionText: string;
  isExact: boolean;
  category: string;
  description: string;
  notes: string;
  sourceLocator: string;
  sourceReference: string;
  sourceUrl: string;
  displayOrder: number;
};

export type V2BuiltinConstantCatalog = {
  versionLabel: string;
  editionYear: number;
  publishedOn: string;
  retrievedOn: string;
  publisher: string;
  sourceUrl: string;
  citation: string;
  seedSha256: string;
  constants: V2BuiltinConstant[];
};

export function getV2BuiltinConstants(): Promise<V2BuiltinConstantCatalog> {
  return invoke<V2BuiltinConstantCatalog>('v2_get_builtin_constants');
}

export type V2StickyNote = {
  id: number;
  title: string;
  body: string;
  color: string;
  isPinned: boolean;
  examId: number | null;
  examCode: string | null;
  subjectNodeId: number | null;
  subjectTitle: string | null;
  topicNodeId: number | null;
  topicTitle: string | null;
  createdAt: string;
  updatedAt: string;
};

export type V2StickyNoteInput = {
  id: number | null;
  title: string;
  body: string;
  color: string;
  isPinned: boolean;
  examId: number | null;
  subjectNodeId: number | null;
  topicNodeId: number | null;
};

export function listV2StickyNotes(): Promise<V2StickyNote[]> {
  return invoke<V2StickyNote[]>('v2_list_sticky_notes');
}

export function saveV2StickyNote(input: V2StickyNoteInput): Promise<V2StickyNote> {
  return invoke<V2StickyNote>('v2_save_sticky_note', { input });
}

export function deleteV2StickyNote(id: number): Promise<void> {
  return invoke<void>('v2_delete_sticky_note', { id });
}

export type V2ChartPoint = {
  label: string;
  seconds: number;
  sessionCount: number;
};

export type V2StudyByExam = {
  examId: number;
  examCode: string;
  examShortName: string;
  allocatedSeconds: number;
  sessionCount: number;
};

export type V2StudyBySubject = {
  examId: number;
  examCode: string;
  subjectNodeId: number | null;
  subjectTitle: string;
  allocatedSeconds: number;
  sessionCount: number;
};

export type V2WeeklyConsistency = {
  weekStart: string;
  label: string;
  eligibleDays: number;
  activeDays: number;
  studySeconds: number;
  sessionCount: number;
};

export type V2QuestionOutcomes = {
  attempts: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  gradedAttempts: number;
  accuracyPercentage: number | null;
  insufficientSample: boolean;
};

export type V2ExamQuestionOutcomes = {
  examId: number;
  examCode: string;
  outcomes: V2QuestionOutcomes;
};

export type V2TopicAccuracy = {
  examId: number;
  examCode: string;
  nodeId: number;
  topicTitle: string;
  attempts: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  gradedAttempts: number;
  accuracyPercentage: number | null;
  insufficientSample: boolean;
};

export type V2SyllabusCompletion = {
  examId: number;
  examCode: string;
  examShortName: string;
  topicCount: number;
  completedTopicCount: number;
  completionPercentage: number | null;
};

export type V2MockTrendPoint = {
  examId: number;
  examCode: string;
  testId: number;
  testDate: string;
  testName: string;
  percentage: number;
};

export type V2MockExamSummary = {
  examId: number;
  examCode: string;
  completedCount: number;
  averagePercentage: number | null;
  bestPercentage: number | null;
  lastPercentage: number | null;
};

export type V2AnalyticsCharts = {
  rangeFrom: string;
  rangeTo: string;
  totalSeconds: number;
  allocatedSeconds: number;
  unallocatedSeconds: number;
  studyByExam: V2StudyByExam[];
  studyBySubject: V2StudyBySubject[];
  dailyStudy: V2ChartPoint[];
  weeklyConsistency: V2WeeklyConsistency[];
  questionOutcomes: V2QuestionOutcomes;
  outcomesByExam: V2ExamQuestionOutcomes[];
  topicAccuracy: V2TopicAccuracy[];
  syllabusCompletion: V2SyllabusCompletion[];
  mockTrend: V2MockTrendPoint[];
  mockByExam: V2MockExamSummary[];
};

export function getV2AnalyticsCharts(input: {
  examId: number | null;
  fromDate: string;
  toDate: string;
}): Promise<V2AnalyticsCharts> {
  return invoke<V2AnalyticsCharts>('v2_get_analytics_charts', { input });
}
