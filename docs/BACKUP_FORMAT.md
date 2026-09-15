# BACKUP_FORMAT.md — backup files

Implemented in `crates/vidya-backup`.

## File name
`vidya-<schoolcode>-<YYYY-MM-DD>-<HHMM>-<kind>.vidyabak` (local time), kind = daily, monthly, manual, pendrive, drive, safety.

## Version 2 container
| Part | Size | Content |
|---|---|---|
| magic | 8 bytes | ASCII `VIDYABK2` |
| header_len | u32 little-endian | length of header JSON |
| header | header_len bytes | UTF-8 JSON, below |
| ciphertext | rest | XChaCha20-Poly1305(key, nonce, plaintext, associated_data = magic + header_len + header) |

Header JSON:
```json
{
  "format": "vidya-backup",
  "version": 2,
  "schoolName": "Vaani Public School",
  "schoolCode": "vaani",
  "createdAt": "2026-09-15T12:30:00Z",
  "appVersion": "1.0.0",
  "schemaVersion": 1,
  "kind": "daily",
  "kdf": { "alg": "argon2id", "memoryKib": 65536, "iterations": 3, "parallelism": 1, "saltB64": "..." },
  "cipher": { "alg": "xchacha20poly1305", "nonceB64": "..." },
  "counts": { "students": 512, "receipts": 1830 }
}
```
Plaintext: `zstd(level 10)` of a SQLite database file. The database inside the backup is **not** SQLCipher-encrypted (the file encryption protects it) so it can be restored on any computer with a new database key.

## Making a backup
1. `VACUUM INTO` a temporary plain SQLite file inside the app's private temp folder (using SQLCipher's export to a plaintext attached database if `VACUUM INTO` cannot write unencrypted; verify which works and document it).
2. Run `PRAGMA integrity_check` on the temporary file.
3. Compress, encrypt, write to `<final>.partial`, `fsync`, rename to final name.
4. Securely delete the temporary plaintext file (overwrite then remove) and never leave it on a pen drive.
5. Log in `backups_log`.

## Key
- Argon2id from the backup password with the header's salt and parameters → 32-byte key.
- For automatic backups the derived key (not the password) is stored protected by DPAPI (Windows) or Keychain (macOS), together with the salt it belongs to. Each automatic backup reuses that salt; manual backups with a typed password use a fresh salt.
- Changing the backup password creates a new salt and key; old files keep working only with the old password.
- `backupCheck` verifier: Argon2id PHC hash of the backup password stored in `app_settings` to confirm the password when typed.

## Version 1 (prototype) import
JSON file: `{ format: "vidya-backup", v: 1, kdf: { alg: "PBKDF2-SHA256", iter, salt }, cipher: { alg: "AES-256-GCM", iv }, data }`, where `data` decrypts to the prototype's JSON database. Import maps it into the v1 schema through services (users keep must_change = 1 because PBKDF2 hashes cannot be converted; generate new temporary passwords and show slips).

## Restore rules
- Refuse if `schoolCode` differs from the activated school (unless no school exists yet).
- Refuse if `schemaVersion` is newer than the app supports.
- Older schema: restore then run migrations.
- Always make a `safety` backup of current data first.
- Replace the database atomically: write new encrypted database beside the old one, swap, restart the app.
- After restore on the office computer: mark all devices `needs_resync = 1`.

## Retention
- Local: 30 newest daily, 12 newest monthly (first backup of each month), all manual and safety backups for 90 days.
- Google Drive: 30 daily and 12 monthly files created by the app.
