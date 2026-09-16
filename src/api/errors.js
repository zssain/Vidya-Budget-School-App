// Error type shared by the whole frontend. Commands convert anything thrown by
// the mock (or, later, a Tauri command) into an AppError with a safe message.
// Shape matches docs/API.md: { kind, messageKey, params, message, field }.

export class AppError extends Error {
  constructor({ kind = 'internal', messageKey, params, message, field } = {}) {
    super(message || messageKey || 'Error');
    this.name = 'AppError';
    this.kind = kind;
    this.messageKey = messageKey;
    this.params = params;
    this.message = message || messageKey || 'Something went wrong.';
    this.field = field;
  }
}

/** Turn anything thrown into an AppError. Unknown throwables become `internal`. */
export function toAppError(unknown) {
  if (unknown instanceof AppError) return unknown;
  if (unknown && typeof unknown === 'object' && (unknown.kind || unknown.messageKey || unknown.message)) {
    return new AppError({
      kind: unknown.kind,
      messageKey: unknown.messageKey,
      params: unknown.params,
      message: unknown.message,
      field: unknown.field,
    });
  }
  return new AppError({ kind: 'internal', message: 'Something went wrong.' });
}
