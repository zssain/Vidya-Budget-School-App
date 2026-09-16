// Activity log. Ported from the prototype (principal only).
import * as db from './db.js';

export function listActivity({ kind, userId, beforeSeq, limit = 300 } = {}) {
  db.need('activity.view');
  return db.state.DB.changes
    .filter(
      (c) =>
        (!kind || kind === 'all' || c.kind === kind) &&
        (!userId || userId === 'all' || c.user === userId) &&
        (beforeSeq == null || c.seq < beforeSeq),
    )
    .slice(0, limit)
    .map((c) => ({
      seq: c.seq,
      at: c.at,
      kind: c.kind,
      text: c.text,
      who: c.who,
      device: c.device,
      userId: c.user,
    }));
}
