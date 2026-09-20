// Backup: encrypted file save and restore (AES-256-GCM). Ported from the prototype.
import * as db from './db.js';

const restores = new Map();
let restoreSeq = 0;

export function backupStatus() {
  db.need('backup.run');
  const since = db.state.DB.meta.seq - (db.state.DB.meta.lastBackupSeq || 0);
  return {
    lastBackupAt: db.state.DB.meta.lastBackupAt,
    lastBackupSeq: db.state.DB.meta.lastBackupSeq,
    changesSince: since,
    overdue: db.backupOverdue(),
  };
}

export function runBackupNow() {
  db.need('backup.run');
  const at = db.nowISO();
  db.commit('backup', 'Automatic backup run');
  db.state.DB.meta.lastBackupAt = at;
  db.state.DB.meta.lastBackupSeq = db.state.DB.meta.seq;
  return { createdAt: at, filename: `vidya-${db.CODE()}-${db.todayKey()}.vidyabak` };
}

export async function backupSaveFile({ backupPassword }) {
  db.need('backup.manage');
  if (!(await db.verifyPassword(backupPassword, db.state.DB.backupCheck)))
    throw db.userError('That is not the backup password.', 'backupPassword');
  const at = db.nowISO();
  db.commit('backup', 'Backup file saved');
  db.state.DB.meta.lastBackupAt = at;
  db.state.DB.meta.lastBackupSeq = db.state.DB.meta.seq;
  const salt = db.randBytes(16);
  const iv = db.randBytes(12);
  const key = await db.aesKeyFrom(backupPassword, salt, db.BK_ITER);
  const data = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    key,
    db.enc.encode(JSON.stringify(db.state.DB)),
  );
  const file = {
    format: 'vidya-backup',
    v: 1,
    school: db.state.DB.school.name,
    code: db.CODE(),
    createdAt: at,
    kdf: { alg: 'PBKDF2-SHA256', iter: db.BK_ITER, salt: db.b64(salt) },
    cipher: { alg: 'AES-256-GCM', iv: db.b64(iv) },
    data: db.b64(data),
  };
  const d = new Date(at);
  const filename = `vidya-${db.CODE()}-${db.dateKey(d)}-${db.pad(d.getHours())}${db.pad(d.getMinutes())}.vidyabak`;
  db.state.DIRTY = false;
  return { filename, content: JSON.stringify(file) };
}

export async function restoreInspect({ file, backupPassword }) {
  if (!file || file.format !== 'vidya-backup' || !file.data)
    throw db.userError('This is not a Vidya backup file.');
  let restored;
  try {
    const key = await db.aesKeyFrom(backupPassword, db.unb64(file.kdf.salt), file.kdf.iter);
    const plain = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv: db.unb64(file.cipher.iv) },
      key,
      db.unb64(file.data),
    );
    restored = JSON.parse(db.dec.decode(plain));
  } catch {
    throw db.userError('Wrong backup password, or the file is damaged.', 'backupPassword');
  }
  if (!restored.meta || restored.meta.app !== 'vidya' || !Array.isArray(restored.users))
    throw db.userError('The file opened but does not contain Vidya data.');
  const previewId = 'r' + ++restoreSeq;
  restores.set(previewId, restored);
  return {
    previewId,
    school: file.school,
    code: file.code,
    createdAt: file.createdAt,
    users: restored.users.length,
    students: restored.students.length,
  };
}

export function restoreCommit({ previewId }) {
  const restored = restores.get(previewId);
  if (!restored) throw db.userError('Restore preview expired. Choose the file again.');
  db.state.DB = restored;
  db.state._paid = null;
  db.state.DIRTY = false;
  db.state.session.user = null;
  db.state.session.pending = null;
  db.commit('backup', 'Restored backup');
  db.state.DIRTY = false;
  restores.clear();
  return {};
}

export async function backupChangePassword({ current, new: next }) {
  db.need('backup.manage');
  if (!(await db.verifyPassword(current, db.state.DB.backupCheck)))
    throw db.userError('The current backup password is wrong.', 'current');
  if ((next || '').length < 8) throw db.userError('Use at least 8 characters.', 'new');
  db.state.DB.backupCheck = await db.hashPassword(next);
  db.commit('backup', 'Backup password changed');
  return {};
}
