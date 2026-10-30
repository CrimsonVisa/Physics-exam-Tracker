export type CommandError = {
  code: string;
  message: string;
  field?: string;
};

const FALLBACK_MESSAGE = 'The local database is unavailable. Open the desktop app and try again.';

export function toCommandError(error: unknown): CommandError {
  if (typeof error === 'object' && error !== null) {
    const candidate = error as Record<string, unknown>;

    if (typeof candidate.code === 'string' && typeof candidate.message === 'string') {
      return {
        code: candidate.code,
        message: candidate.message,
        field: typeof candidate.field === 'string' ? candidate.field : undefined,
      };
    }
  }

  return { code: 'unknown_error', message: FALLBACK_MESSAGE };
}
