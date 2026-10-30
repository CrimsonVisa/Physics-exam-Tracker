import { invoke } from '@tauri-apps/api/core';

export type ResourceCategory =
  | 'handwritten_notes'
  | 'classroom_notes'
  | 'assignments'
  | 'formula_sheets'
  | 'previous_year_papers'
  | 'important_topics'
  | 'recommended_books'
  | 'nptel_courses'
  | 'youtube_playlists'
  | 'other';

export type ResourceUnit =
  | 'General Aptitude'
  | 'Mathematical Methods of Physics'
  | 'Classical Mechanics'
  | 'Electromagnetic Theory'
  | 'Quantum Mechanics'
  | 'Thermodynamic and Statistical Physics'
  | 'Electronics and Experimental Methods'
  | 'Atomic & Molecular Physics'
  | 'Condensed Matter Physics'
  | 'Nuclear and Particle Physics'
  | 'All Units'
  | 'Unmapped';

export type ResourceStudyStatus = 'not_started' | 'studying' | 'completed' | 'skipped';

export type LocalResourceFile = {
  fileName: string;
  filePath: string;
  sizeBytes: number;
  sha256: string;
  importedAt: string;
};

export type Resource = {
  id: number;
  resourceKey: string;
  title: string;
  description: string;
  category: ResourceCategory;
  unit: ResourceUnit;
  topicCode: string | null;
  sourcePageUrl: string | null;
  originalUrl: string;
  externalId: string | null;
  externalKind: 'drive_file' | 'drive_folder' | 'nptel_course' | 'youtube_playlist' | 'web_page';
  mimeType: string | null;
  availability: 'online' | 'removed' | 'unverified' | 'invalid';
  discoveredAt: string;
  lastSeenAt: string | null;
  lastVerifiedAt: string | null;
  removedAt: string | null;
  studyStatus: ResourceStudyStatus;
  personalNotes: string;
  lastOpenedAt: string | null;
  localFile: LocalResourceFile | null;
};

export type DiscoveryRunInfo = {
  status: 'running' | 'success' | 'partial' | 'blocked' | 'failed';
  finishedAt: string | null;
  pagesOk: number;
  pagesFailed: number;
  resourcesSeen: number;
  resourcesAdded: number;
  resourcesUpdated: number;
  resourcesRemoved: number;
  message: string;
};

export type ResourceOverview = {
  total: number;
  localCount: number;
  onlineCount: number;
  removedCount: number;
  studyingCount: number;
  completedCount: number;
  lastDiscovery: DiscoveryRunInfo | null;
};

export type DiscoverySummary = {
  status: 'success' | 'partial' | 'blocked' | 'failed';
  pagesOk: number;
  pagesFailed: number;
  resourcesSeen: number;
  resourcesAdded: number;
  resourcesUpdated: number;
  resourcesRemoved: number;
  message: string;
};

export type ManualResourceInput = {
  title: string;
  description: string;
  category: ResourceCategory;
  unit: ResourceUnit;
  topicCode: string | null;
  originalUrl: string;
  sourcePageUrl: string | null;
};

export function getResourceCatalogue(): Promise<Resource[]> {
  return invoke<Resource[]>('get_resource_catalogue');
}

export function getResourceOverview(): Promise<ResourceOverview> {
  return invoke<ResourceOverview>('get_resource_overview');
}

export function refreshResourceCatalogue(): Promise<DiscoverySummary> {
  return invoke<DiscoverySummary>('refresh_resource_catalogue');
}

export function createManualResource(resource: ManualResourceInput): Promise<Resource> {
  return invoke<Resource>('create_manual_resource', { resource });
}

export function updateResourceProgress(
  resourceId: number,
  status: ResourceStudyStatus,
  notes: string,
): Promise<Resource> {
  return invoke<Resource>('update_resource_progress', {
    resourceId,
    progress: { status, notes },
  });
}

export function markResourceOpened(resourceId: number): Promise<void> {
  return invoke<void>('mark_resource_opened', { resourceId });
}

export function importResourcePdf(
  resourceId: number,
  sourcePath: string,
  destinationDir: string,
): Promise<LocalResourceFile> {
  return invoke<LocalResourceFile>('import_resource_pdf', {
    resourceId,
    sourcePath,
    destinationDir,
  });
}

export function removeResourcePdf(resourceId: number): Promise<void> {
  return invoke<void>('remove_resource_pdf', { resourceId });
}
