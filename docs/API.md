# API.md — commands and endpoints

Every command listed here must exist in three places, checked by `scripts/check-api-drift.mjs` in `npm run verify`:
1. `src-tauri/src/commands/*.rs` and the `generate_handler!` list
2. `src/api/commands.js` (camelCase wrapper)
3. This file

## Conventions
- Command names: `snake_case` in Rust, `camelCase` in JS (`collect_fee` → `collectFee`).
- Every command except those marked **no session** takes `token: String` as the first argument. The JS wrapper adds it automatically from the session module.
- Inputs are one struct named `<Command>Input`; outputs are DTOs named `<Thing>Dto`. All use `serde(rename_all = "camelCase")`.
- Desktop-only commands (feature `desktop-server`) are marked **D**. Mobile-only are marked **M**. Unmarked commands exist on both.
- Permissions refer to `PERMISSIONS.md`.

## Error format (same for Tauri and HTTP)
```json
{ "kind": "permission|validation|not_found|conflict|auth|locked|license|offline|internal",
  "messageKey": "fees.error.over_balance",
  "params": { "balance": 9100 },
  "message": "That is more than the balance of ₹9,100.",
  "field": "amount" }
```
- `message` is already translated to the user's language by Rust, so the UI can show it directly.
- `field` is set for validation errors tied to a form field, otherwise omitted.
- `internal` errors never include technical details in `message`; details go to the log.
- HTTP status mapping: validation 422, permission 403, auth 401, locked 423, not_found 404, conflict 409, license 402, offline 503, internal 500.

## App and setup
| Command | Permission | Input → Output | Notes |
|---|---|---|---|
| `app_status` | no session | → `AppStatusDto { hasSchool, licensed, deviceLocked, wizardStep, platform, version, schoolName?, schoolCode? }` | First call on start |
| `get_device_id` **D** | no session | → `{ deviceId }` | |
| `activate` **D** | no session, only before setup | `{ code }` → `LicenseDto` | |
| `wizard_save_step` **D** | no session, only while no school | `{ step, data }` → `WizardStateDto` | |
| `wizard_get_state` **D** | no session, only while no school | → `WizardStateDto` | |
| `wizard_create_school` **D** | no session, only while no school | → `{ principalUsername, credentials: [CredentialSlipDto] }` | Temporary passwords only in this response |
| `load_sample_school` **D** | no session, debug builds only | → `{ logins }` | Compiled out of release builds |
| `get_license` **D** | license.view | → `LicenseDto` | |
| `replace_activation` **D** | no session, only when licensed to another computer | `{ code }` → `LicenseDto` | Same school code only |

## Auth and account
| Command | Permission | Input → Output |
|---|---|---|
| `sign_in` | no session | `{ username, password }` → `SignInResultDto { status: "ok", token, user: CurrentUserDto }` or `{ status: "must_change_password", pendingToken }` |
| `set_first_password` | pending token | `{ pendingToken, newPassword }` → `{ token, user }` |
| `sign_out` | any | → `()` |
| `current_user` | any | → `CurrentUserDto { id, name, username, role, sections, language, permissions: [string] }` |
| `change_password` | account.change_own_password | `{ currentPassword, newPassword }` → `()` |
| `set_language` | account.set_own_language | `{ language }` → `()` |
| `principal_reset_with_code` **D** | no session | `{ code, newPassword }` → `()` |

## Home
| Command | Permission | Output |
|---|---|---|
| `home_principal` | reports.view | `PrincipalHomeDto` |
| `home_accountant` | fees.view | `AccountantHomeDto` |
| `home_teacher` | attendance.view (own) | `TeacherHomeDto` |

## Students
| Command | Permission | Input → Output |
|---|---|---|
| `list_students` | students.view | `StudentFilter { q?, sectionId?, status? }` → `[StudentListItemDto]` (role-shaped) |
| `get_student` | students.view | `{ studentId }` → `StudentDetailDto` (role-shaped) |
| `add_student` | students.add | `StudentInput { ..., confirmDuplicate? }` → `StudentDetailDto` or conflict `students.possible_duplicate` |
| `update_student` | students.edit (+ set_concession when concession changes) | `{ studentId, ...fields }` → `StudentDetailDto` |
| `mark_student_left` | students.mark_left | `{ studentId, leftOn, reason }` → `StudentDetailDto` |
| `export_students_xlsx` **D** | students.export | `{ path }` → `{ path }` |
| `import_students_preview` **D** | students.import | `{ path }` → `ImportPreviewDto { rows, errors }` |
| `import_students_commit` **D** | students.import | `{ previewId }` → `{ imported }` |
| `issue_tc` **D** | students.issue_tc | `TcInput` → `TcDto` |
| `get_tc` **D** | students.issue_tc | `{ tcId }` → `TcDto` |
| `export_import_template_xlsx` **D** | students.import | `{ path }` → `{ path }` |

## Attendance
| Command | Permission | Input → Output |
|---|---|---|
| `get_attendance` | attendance.view | `{ sectionId, date }` → `AttendanceSheetDto { students, marks, savedBy?, savedAt?, readOnlyReason? }` |
| `save_attendance` | attendance.mark_today or edit_past | `{ sectionId, date, marks: {studentId: "P"|"A"|"L"} }` → `AttendanceSheetDto` |
| `attendance_register` | attendance.print_register | `{ sectionId, month: "YYYY-MM" }` → `RegisterDto` |

## Marks and report cards
| Command | Permission | Input → Output |
|---|---|---|
| `get_marks_sheet` | marks.view | `{ examId, sectionId }` → `MarksSheetDto` |
| `save_marks` | marks.enter | `{ examId, sectionId, entries: [{studentId, subjectId, value: number|"AB"|null}] }` → `MarksSheetDto` or validation errors per cell |
| `get_report_card` | reportcard.view | `{ studentId }` → `ReportCardDto` |
| `get_class_report_cards` | reportcard.print | `{ sectionId }` → `[ReportCardDto]` |

## Fees
| Command | Permission | Input → Output |
|---|---|---|
| `fee_register` | fees.view | `FeeFilter { q?, sectionId?, state? }` → `FeeRegisterDto` |
| `get_fee_account` | fees.view | `{ studentId }` → `FeeAccountDto { due, paid, balance, termFee, receipts, previousSessionDues }` |
| `collect_fee` | fees.collect | `{ studentId, amount, mode, reference?, note? }` → `ReceiptDto` |
| `get_receipt` | fees.view | `{ receiptId }` → `ReceiptDto` |
| `cancel_receipt` | fees.cancel_receipt | `{ receiptId, reason }` → `ReceiptDto` |
| `day_book` | fees.daybook | `{ date }` → `DayBookDto` |
| `export_dues_xlsx` **D** | fees.export | `{ path }` → `{ path }` |
| `export_daybook_xlsx` **D** | fees.export | `{ date, path }` → `{ path }` |

## Reports, activity, alerts
| Command | Permission | Input → Output |
|---|---|---|
| `reports_summary` | reports.view | → `ReportsDto` |
| `export_class_summary_xlsx` **D** | reports.export | `{ path }` → `{ path }` |
| `list_activity` | activity.view | `{ kind?, userId?, beforeSeq?, limit }` → `[ActivityDto]` |
| `undo_field_change` **D** | activity.view + settings.edit | `{ historyId }` → `()` |
| `list_alerts` | alerts.view | → `[AlertDto]` |
| `resolve_alert` | alerts.view | `{ alertId }` → `()` |

## Staff logins
| Command | Permission | Input → Output |
|---|---|---|
| `list_users` | users.view | → `[UserDto]` |
| `create_user` | users.manage | `{ name, mobile, role: "teacher"|"accountant", sectionIds, username? }` → `CredentialSlipDto` |
| `update_user` | users.manage | `{ userId, name, mobile, sectionIds }` → `UserDto` |
| `reset_user_password` | users.manage | `{ userId }` → `CredentialSlipDto` |
| `unlock_user` | users.manage | `{ userId }` → `UserDto` |
| `set_user_active` | users.manage | `{ userId, active }` → `UserDto` |

## Settings and session
| Command | Permission | Input → Output |
|---|---|---|
| `get_settings` | settings.view | → `SettingsDto` |
| `save_school` | settings.edit | `SchoolInput` → `SettingsDto` |
| `save_classes` | settings.edit | `ClassesInput` → `SettingsDto` |
| `save_fee_plan` | settings.edit | `FeePlanInput` → `SettingsDto` |
| `save_subjects` | settings.edit | `SubjectsInput` → `SettingsDto` |
| `save_exams` | settings.edit | `ExamsInput` → `SettingsDto` |
| `save_grade_scale` | settings.edit | `GradeScaleInput` → `SettingsDto` |
| `save_app_settings` **D** | settings.edit | `AppSettingsInput` → `SettingsDto` |
| `save_device_code` **D** | settings.edit | `{ code }` → `SettingsDto` |
| `session_change_preview` **D** | session.change | `{ newSessionName }` → `PromotionPreviewDto` |
| `session_change_commit` **D** | session.change | `{ previewId, repeaters: [studentId], left: [studentId] }` → `{ newSessionId }` |
| `list_sessions` | settings.view | → `[SessionDto]` |

## Backup (desktop)
| Command | Permission | Input → Output |
|---|---|---|
| `backup_status` **D** | backup.manage | → `BackupStatusDto` |
| `backup_save_file` **D** | backup.manage | `{ backupPassword, path }` → `BackupResultDto` |
| `backup_list_removable` **D** | backup.manage | → `[RemovableDriveDto]` |
| `backup_to_pendrive` **D** | backup.manage | `{ driveId }` → `BackupResultDto` |
| `backup_enable_auto` **D** | backup.manage | `{ backupPassword }` → `BackupStatusDto` |
| `backup_change_password` **D** | backup.manage | `{ current, new }` → `()` |
| `restore_inspect` **D** | backup.manage or no school | `{ path, backupPassword }` → `RestorePreviewDto` |
| `restore_commit` **D** | backup.manage or no school | `{ previewId }` → `()` then app restarts |
| `backup_test_latest` **D** | backup.manage | → `BackupVerifyDto` |
| `drive_connect` **D** | backup.manage | → `{ email }` |
| `drive_disconnect` **D** | backup.manage | → `()` |
| `drive_list_backups` **D** | backup.manage | → `[DriveFileDto]` |
| `drive_restore_download` **D** | backup.manage or no school | `{ fileId }` → `{ path }` |

## Devices and LAN (desktop)
| Command | Permission | Input → Output |
|---|---|---|
| `server_status` **D** | devices.view | → `ServerStatusDto { running, port, addresses, fingerprintShort }` |
| `connection_check` **D** | devices.view | → `ConnectionCheckDto` |
| `list_devices` **D** | devices.view | → `[DeviceDto]` |
| `list_pending_approvals` **D** | devices.approve | → `[PendingApprovalDto]` |
| `decide_approval` **D** | devices.approve | `{ approvalId, allow, receiptPrefix? }` → `()` |
| `revoke_device` **D** | devices.manage | `{ deviceId }` → `()` |

## Phone (mobile)
| Command | Permission | Input → Output |
|---|---|---|
| `discover_server` **M** | no session | `{ schoolCode }` → `DiscoveryResultDto` |
| `remote_sign_in` **M** | no session | `{ username, password }` → `SignInResultDto` or `{ status: "waiting_for_approval", matchCode }` |
| `sync_now` **M** | any | → `SyncStatusDto` |
| `sync_status` **M** | any | → `SyncStatusDto { state, lastSyncAt, waiting, rejected: [RejectedChangeDto] }` |
| `export_receipt_pdf` | fees.view | `{ receiptId }` → `{ path }` (then share sheet on Android) |
| `export_report_card_pdf` | reportcard.print | `{ studentId }` → `{ path }` |
| `debug_storage_selftest` **M** | no session, debug builds only | → `SelfTestDto` |

## Events (Rust → frontend)
| Event | Payload | When |
|---|---|---|
| `session-expired` | `{}` | Idle timeout |
| `approval-requested` **D** | `PendingApprovalDto` | Phone waiting for approval |
| `sync-changed` | `SyncStatusDto` | Sync state changes |
| `data-changed` | `{ entities: [string] }` | Remote changes applied, so views can refresh |
| `backup-status` **D** | `BackupStatusDto` | Backup finished or failed |

## LAN HTTP API (office computer, `https://<ip>:47631/api/v1`)
All routes except discovery and sign-in require headers: `Authorization: Bearer <token>`, `X-Vidya-Device: <deviceId>`, `X-Vidya-Time: <unix ms>`, `X-Vidya-Nonce: <random>`, `X-Vidya-Signature: <base64 Ed25519 signature>`. See `SYNC_PROTOCOL.md` for the signature string.

| Method and path | Purpose | Service call |
|---|---|---|
| `POST /auth/sign-in` | Password check, approval flow | `AuthService::remote_sign_in` |
| `GET /auth/approval/{id}` | Poll approval result | `DeviceService::approval_status` |
| `POST /auth/first-password` | Set password with pending token | `AuthService::set_first_password` |
| `POST /auth/sign-out` | End session | `AuthService::sign_out` |
| `GET /sync/snapshot?page=` | Role-filtered snapshot | `SyncService::snapshot` |
| `POST /sync/push` | Send changes | `SyncService::apply_push` |
| `GET /sync/pull?since=` | Receive changes | `SyncService::pull` |
| `GET /sync/live` (WebSocket) | "new changes up to seq N" | broadcaster |
| `POST /students` | Add student (online-only, allocates admission number) | `StudentService::add` |
| `GET /time` | Server time for clock offset | — |
| `GET /health` | Returns school code and version, no session | — |
