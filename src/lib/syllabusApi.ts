import { invoke } from '@tauri-apps/api/core';

export const TOPIC_STATUSES = ['Not started', 'Learning', 'Revised', 'Confident'] as const;
export type TopicStatus = (typeof TOPIC_STATUSES)[number];

export const TOPIC_PRIORITIES = ['Low', 'Normal', 'High', 'High-Yield'] as const;
export type TopicPriority = (typeof TOPIC_PRIORITIES)[number];

export const TOPIC_CATEGORIES = ['General Aptitude', 'Core', 'Advanced'] as const;
export type TopicCategory = (typeof TOPIC_CATEGORIES)[number];

export type SyllabusSource = {
  code: string;
  title: string;
  publisher: string;
  url: string;
  landingUrl: string | null;
  verifiedOn: string;
  notes: string;
};

export type SyllabusSection = {
  code: string;
  title: string;
  sourceCode: string;
  sourceReference: string;
  displayOrder: number;
};

export type SyllabusCatalog = {
  sources: SyllabusSource[];
  sections: SyllabusSection[];
};

export type TopicProgress = {
  status: TopicStatus;
  priority: TopicPriority;
  confidence: number;
  notes: string;
  updatedAt: string | null;
  recorded: boolean;
};

export type SyllabusTopic = {
  id: number;
  code: string;
  sectionCode: string;
  sectionTitle: string;
  title: string;
  category: TopicCategory;
  sourceCode: string;
  sourceTitle: string;
  sourceUrl: string;
  sourceReference: string;
  progress: TopicProgress;
};

export type SyllabusFilter = {
  query?: string;
  sectionCode?: string;
  category?: TopicCategory;
  status?: TopicStatus;
  priority?: TopicPriority;
};

export type StatusCounts = {
  notStarted: number;
  learning: number;
  revised: number;
  confident: number;
};

export type SectionSummary = {
  code: string;
  title: string;
  totalTopics: number;
  statusCounts: StatusCounts;
  approximateProgress: number;
};

export type SyllabusSummary = {
  totalTopics: number;
  statusCounts: StatusCounts;
  approximateProgress: number;
  sections: SectionSummary[];
};

export type TopicProgressInput = {
  status: TopicStatus;
  priority: TopicPriority;
  confidence: number;
  notes: string;
};

export function getSyllabusCatalog(): Promise<SyllabusCatalog> {
  return invoke<SyllabusCatalog>('get_syllabus_catalog');
}

export function searchSyllabusTopics(filter: SyllabusFilter): Promise<SyllabusTopic[]> {
  return invoke<SyllabusTopic[]>('search_syllabus_topics', { filter });
}

export function getTopicProgress(topicId: number): Promise<TopicProgress> {
  return invoke<TopicProgress>('get_topic_progress', { topicId });
}

export function updateTopicProgress(
  topicId: number,
  progress: TopicProgressInput,
): Promise<TopicProgress> {
  return invoke<TopicProgress>('update_topic_progress', { topicId, progress });
}

export function getSyllabusSummary(): Promise<SyllabusSummary> {
  return invoke<SyllabusSummary>('get_syllabus_summary');
}
