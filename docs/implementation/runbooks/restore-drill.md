# Backup → restore drill (local, fake school)

An automated rehearsal of the owner's pre-sale check "the school PC dies; a replacement PC is
restored from last night's backup and the school carries on". It runs the app's real code on a
synthetic school and can be re-run at any time:

```sh
cargo test -p vidya --test restore_drill        # about 20 s after the build
```

## What it does

PC A, the old school PC:

1. Builds a synthetic school on an encrypted DB. It starts from the demo seed: 670 students,
   teachers, an accountant, 3 devices, requests and expenses.
2. Adds the rest through the real command logic:
   - 8 new classes with 400 admissions via `create_student_logic`, giving 1,070 students in
     17 classes, with their fee dues;
   - 6 days of past attendance sheets per new class via `submit_attendance_logic`;
   - the half-yearly marks sheet via `submit_marks_logic`;
   - 134 accountant payments via `record_payment_logic`, with allocations, receipt vouchers
     and ledger entries;
   - a Principal reversal via `reverse_payment_logic`, which posts a reversal voucher;
   - a pending reversal request and three more devices.
3. Takes the backup through the real "Turn on backups / Back up now" path:
   `schedule::derive_and_cache` (backup key = Argon2id of the recovery key and the school salt)
   then `schedule::run_and_record`. That path exports the encrypted `.vbak`, verifies it,
   writes the `.vbak.meta` sidecar and records a `backup_run` row.

PC B, the replacement: a data folder that does not exist yet (no DB, no key). The drill takes
only the `.vbak` + `.meta` across, as if on a USB stick, then:

1. Runs the restore commands' path: `restore::summary_with_recovery`. It checks the school,
   the student and payment counts, the last receipt and that the chain is OK. Then it runs
   `restore::install_then` under a fresh DB key; storing that key is the commit step.
2. Verifies the restored DB:
   - `integrity_check` passes;
   - **every table** has the same row count and identical content (SHA-256 over the sorted
     rows). The exceptions are the rows the restore changes on purpose: `audit_log` (+1
     `restore` entry), `school` (epoch), `device` (rejoin flags) and the derived
     `data_revision` / `role_permission`;
   - the **trial balance** (Σ debit / Σ credit per account) is identical, the book balances,
     every voucher balances, and every payment's allocations add up to its amount;
   - the **audit chain verifies** (the `verify_audit_chain` command's check) and ends with the
     `restore` entry;
   - **fencing** per §8.9 / §12: `server_epoch` + 1, and every device has `needs_rejoin = 1`.
3. Opens the DB the way the app does at boot (migrations, voucher backfill, role seed) and
   checks the app works:
   - app state is `locked` and the Principal's PIN unlocks;
   - students, dues, an attendance sheet, the day book (with the reversal) and the profit
     summary all read;
   - a new payment continues the receipt series, and a new admission works;
   - the chain still verifies;
   - PC B can take its own backup, and the same recovery key gives the same backup key.

It also checks two failure cases:

- **Wrong recovery key.** The summary is unreadable and the install is refused before staging.
  The PC's data folder, the `.vbak` and the `.meta` stay byte-identical, and no folder is
  created on a fresh PC.
- **Interrupted install.**
  - A stale staged file left by a crash is removed at boot.
  - A failed commit (the key store refuses the new key) rolls the swap back.
  - A power cut between "old DB moved aside" and "restored DB renamed in" is repaired at the
    next boot: `restore::recover_interrupted_install`, called by `build_ctx`.
  - In every case the previous live DB opens under its own key and still takes writes.

The rename-failure-mid-swap case is a unit test in `src-tauri/src/backup/restore.rs`. It needs
a failing rename injected.

Not covered (needs a real machine): the OS keychain write, `app.restart()`, a new TLS cert,
relay re-registration, and devices actually re-joining.
