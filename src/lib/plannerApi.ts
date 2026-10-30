import { invoke } from '@tauri-apps/api/core';
import type { Settings } from './settingsApi';

export type PlannerTopic = {
  id: number;
  code: string;
  title: string;
  archived: boolean;
};

export type TaskPriority = 'Low' | 'Normal' | 'High' | 'High-Yield';
export type TaskStatus = 'open' | 'completed';
export type ScheduleAnchor = 'manual' | 'exam';

export type ChecklistTask = {
  id: number;
  title: string;
  description: string;
  scheduledDate: string;
  deadlineDate: string | null;
  topic: PlannerTopic | null;
  priority: TaskPriority;
  status: TaskStatus;
  scheduleAnchor: ScheduleAnchor;
  scheduleOffsetDays: number | null;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
  dueDate: string;
  isOverdue: boolean;
  isUpcoming: boolean;
};

export type TaskInput = {
  title: string;
  description: string;
  scheduledDate: string;
  deadlineDate: string | null;
  topicId: number | null;
  priority: TaskPriority;
  scheduleAnchor: ScheduleAnchor;
  scheduleOffsetDays: number | null;
};

export type TaskFilter = {
  status?: TaskStatus | 'all';
  query?: string;
  fromDate?: string;
  toDate?: string;
};

export type PlannedBlockStatus = 'planned' | 'completed' | 'cancelled';

export type PlannedBlock = {
  id: number;
  title: string;
  topic: PlannerTopic | null;
  calendarDate: string;
  startTime: string;
  endTime: string;
  status: PlannedBlockStatus;
  notes: string;
  linkedSessionId: number | null;
  createdAt: string;
  updatedAt: string;
};

export type PlannedBlockInput = {
  title: string;
  topicId: number | null;
  calendarDate: string;
  startTime: string;
  endTime: string;
  status: PlannedBlockStatus;
  notes: string;
  linkedSessionId: number | null;
};

export type CalendarData = {
  fromDate: string;
  toDate: string;
  today: string;
  tasks: ChecklistTask[];
  blocks: PlannedBlock[];
  deadlines: ChecklistTask[];
  upcomingDeadlines: ChecklistTask[];
  overdueTasks: ChecklistTask[];
};

export type Agenda = {
  today: string;
  todayTasks: ChecklistTask[];
  todayBlocks: PlannedBlock[];
  upcomingDeadlines: ChecklistTask[];
  overdueTasks: ChecklistTask[];
};

export type TaskDateChange = {
  taskId: number;
  title: string;
  currentScheduledDate: string;
  proposedScheduledDate: string;
  offsetDays: number;
  outsideWindow: boolean;
};

export type OutsideWindowItem = {
  kind: 'task' | 'task_deadline' | 'planned_block';
  id: number;
  title: string;
  date: string;
  reason: string;
};

export type ExamReschedulePreview = {
  currentExamDate: string;
  proposedExamDate: string;
  proposedPreparationStartDate: string;
  proposedDailyStudyTargetMinutes: number;
  changes: TaskDateChange[];
  outsideWindow: OutsideWindowItem[];
  completedTasksUntouched: number;
  manualTasksUntouched: number;
  plannedBlocksUntouched: number;
};

export type AcceptedTaskChange = {
  taskId: number;
  expectedScheduledDate: string;
};

export type ApplyExamDateChangeInput = {
  settings: Settings;
  expectedExamDate: string;
  acceptedChanges: AcceptedTaskChange[];
};

export type ApplyExamDateChangeResult = {
  settings: Settings;
  appliedTaskCount: number;
  preservedOutsideWindowCount: number;
};

export function getPlannerTopics(): Promise<PlannerTopic[]> {
  return invoke<PlannerTopic[]>('get_planner_topics');
}

export function getChecklistTasks(filter: TaskFilter = {}): Promise<ChecklistTask[]> {
  return invoke<ChecklistTask[]>('get_checklist_tasks', { filter });
}

export function createChecklistTask(task: TaskInput): Promise<ChecklistTask> {
  return invoke<ChecklistTask>('create_checklist_task', { task });
}

export function updateChecklistTask(taskId: number, task: TaskInput): Promise<ChecklistTask> {
  return invoke<ChecklistTask>('update_checklist_task', { taskId, task });
}

export function completeChecklistTask(taskId: number): Promise<ChecklistTask> {
  return invoke<ChecklistTask>('complete_checklist_task', { taskId });
}

export function reopenChecklistTask(taskId: number): Promise<ChecklistTask> {
  return invoke<ChecklistTask>('reopen_checklist_task', { taskId });
}

export function rescheduleChecklistTask(
  taskId: number,
  scheduledDate: string,
): Promise<ChecklistTask> {
  return invoke<ChecklistTask>('reschedule_checklist_task', { taskId, scheduledDate });
}

export function deleteChecklistTask(taskId: number): Promise<void> {
  return invoke<void>('delete_checklist_task', { taskId });
}

export function getCalendar(fromDate: string, toDate: string): Promise<CalendarData> {
  return invoke<CalendarData>('get_calendar', { fromDate, toDate });
}

export function getAgenda(): Promise<Agenda> {
  return invoke<Agenda>('get_agenda');
}

export function createPlannedBlock(block: PlannedBlockInput): Promise<PlannedBlock> {
  return invoke<PlannedBlock>('create_planned_block', { block });
}

export function updatePlannedBlock(
  blockId: number,
  block: PlannedBlockInput,
): Promise<PlannedBlock> {
  return invoke<PlannedBlock>('update_planned_block', { blockId, block });
}

export function updatePlannedBlockStatus(
  blockId: number,
  status: PlannedBlockStatus,
): Promise<PlannedBlock> {
  return invoke<PlannedBlock>('update_planned_block_status', { blockId, status });
}

export function linkPlannedBlockSession(
  blockId: number,
  sessionId: number | null,
): Promise<PlannedBlock> {
  return invoke<PlannedBlock>('link_planned_block_session', { blockId, sessionId });
}

export function deletePlannedBlock(blockId: number): Promise<void> {
  return invoke<void>('delete_planned_block', { blockId });
}

export function previewExamDateChange(settings: Settings): Promise<ExamReschedulePreview> {
  return invoke<ExamReschedulePreview>('preview_exam_date_change', { settings });
}

export function applyExamDateChange(
  change: ApplyExamDateChangeInput,
): Promise<ApplyExamDateChangeResult> {
  return invoke<ApplyExamDateChangeResult>('apply_exam_date_change', { change });
}
