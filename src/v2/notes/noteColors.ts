export const NOTE_COLORS = [
  { id: 'yellow', label: 'Yellow' },
  { id: 'blue', label: 'Blue' },
  { id: 'green', label: 'Green' },
  { id: 'pink', label: 'Pink' },
  { id: 'purple', label: 'Purple' },
  { id: 'orange', label: 'Orange' },
] as const;

export type NoteColor = (typeof NOTE_COLORS)[number]['id'];

export function noteColorLabel(color: string): string {
  return NOTE_COLORS.find((entry) => entry.id === color)?.label ?? color;
}
