export const COMPACT_NOTES_LAYOUT_MAX_WIDTH = 920;

export function isCompactNotesWidth(width: number): boolean {
  return width <= COMPACT_NOTES_LAYOUT_MAX_WIDTH;
}
