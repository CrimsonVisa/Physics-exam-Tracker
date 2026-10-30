import { invoke } from '@tauri-apps/api/core';

export { type CommandError, toCommandError } from './commandError';

export type Settings = {
  examDate: string;
  preparationStartDate: string;
  dailyStudyTargetMinutes: number;
};

export function getSettings(): Promise<Settings> {
  return invoke<Settings>('get_settings');
}

export function saveSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>('update_settings', { settings });
}
