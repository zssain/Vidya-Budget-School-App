// Every export here maps 1:1 to a Tauri command in docs/API.md. Mock
// implementations are temporary (removed from P2.7 onward). Views call ONLY
// these functions, never invoke() or the mock directly.

import { invoke } from '@tauri-apps/api/core';
import { mock } from './mock/index.js';
import { toAppError } from './errors.js';
import { clearToken, getToken, setToken } from './session.js';

async function call(name, arg) {
  try {
    return await mock[name](arg);
  } catch (e) {
    throw toAppError(e);
  }
}

// A real Tauri command. `authed()` attaches the in-memory session token.
async function run(name, args) {
  try {
    return await invoke(name, args);
  } catch (e) {
    throw toAppError(e);
  }
}
function authed(args = {}) {
  return { token: getToken(), ...args };
}

// ---- App and setup (real Rust; P2.4/P2.7) ----
export async function appStatus() {
  return run('app_status');
}
export async function getDeviceId() {
  return call('get_device_id');
}
export async function activate(input) {
  return call('activate', input);
}
export async function wizardCreateSchool(input) {
  return call('wizard_create_school', input);
}
// Debug builds only (the button is hidden in release). Also seeds the mock DB so
// the still-mock feature screens have matching data during the P3.1–P4.4 transition.
export async function loadSampleSchool() {
  await run('load_sample_school');
  try {
    await mock.load_sample_school?.();
  } catch {
    // ignore: mock may already be seeded
  }
  return {};
}

// ---- Auth and account (real Rust; P2.7) ----
export async function signIn(input) {
  const r = await run('sign_in', input);
  if (r && r.token) setToken(r.token);
  // Bridge: mirror the session into the mock so mock feature screens work.
  if (r && r.status === 'ok') {
    try {
      await mock.sign_in?.(input);
    } catch {
      // ignore: features fall back to their own mock guards
    }
  }
  return r;
}
export async function setFirstPassword(input) {
  const r = await run('set_first_password', input);
  if (r && r.token) setToken(r.token);
  return r;
}
export async function signOut() {
  const r = await run('sign_out', authed());
  clearToken();
  try {
    await mock.sign_out?.();
  } catch {
    // ignore
  }
  return r;
}
export async function currentUser() {
  return run('current_user', authed());
}
export async function changePassword(input) {
  return run('change_password', authed(input));
}
export async function setLanguage(input) {
  return run('set_language', authed(input));
}

// ---- Home ----
export async function homePrincipal() {
  return call('home_principal');
}
export async function homeAccountant() {
  return call('home_accountant');
}
export async function homeTeacher() {
  return call('home_teacher');
}

// ---- Students ----
export async function listStudents(filter) {
  return call('list_students', filter);
}
export async function getStudent(input) {
  return call('get_student', input);
}
export async function addStudent(input) {
  return call('add_student', input);
}
export async function updateStudent(input) {
  return call('update_student', input);
}
export async function exportStudentsXlsx(input) {
  return call('export_students_xlsx', input);
}

// ---- Attendance ----
export async function getAttendance(input) {
  return call('get_attendance', input);
}
export async function saveAttendance(input) {
  return call('save_attendance', input);
}
export async function attendanceRegister(input) {
  return call('attendance_register', input);
}

// ---- Marks and report cards ----
export async function getMarksSheet(input) {
  return call('get_marks_sheet', input);
}
export async function saveMarks(input) {
  return call('save_marks', input);
}
export async function getReportCard(input) {
  return call('get_report_card', input);
}
export async function getClassReportCards(input) {
  return call('get_class_report_cards', input);
}

// ---- Fees ----
export async function feeRegister(filter) {
  return call('fee_register', filter);
}
export async function getFeeAccount(input) {
  return call('get_fee_account', input);
}
export async function collectFee(input) {
  return call('collect_fee', input);
}
export async function getReceipt(input) {
  return call('get_receipt', input);
}
export async function cancelReceipt(input) {
  return call('cancel_receipt', input);
}
export async function dayBook(input) {
  return call('day_book', input);
}
export async function exportDuesXlsx(input) {
  return call('export_dues_xlsx', input);
}
export async function exportDaybookXlsx(input) {
  return call('export_daybook_xlsx', input);
}

// ---- Reports and activity ----
export async function reportsSummary() {
  return call('reports_summary');
}
export async function exportClassSummaryXlsx(input) {
  return call('export_class_summary_xlsx', input);
}
export async function listActivity(input) {
  return call('list_activity', input);
}

// ---- Staff logins (real Rust; P3.1) ----
export async function listUsers() {
  return run('list_users', authed());
}
export async function createUser(input) {
  return run(
    'create_user',
    authed({
      input: {
        name: input.name,
        role: input.role,
        mobile: input.mobile,
        sections: input.sectionIds || [],
        username: input.username,
      },
    }),
  );
}
export async function updateUser(input) {
  return run(
    'update_user',
    authed({
      input: {
        userId: input.userId,
        name: input.name,
        mobile: input.mobile,
        sections: input.sectionIds || [],
      },
    }),
  );
}
export async function resetUserPassword(input) {
  return run('reset_user_password', authed({ userId: input.userId }));
}
export async function unlockUser(input) {
  return run('unlock_user', authed({ userId: input.userId }));
}
export async function setUserActive(input) {
  return run('set_user_active', authed({ userId: input.userId, active: input.active }));
}

// ---- Settings ----
export async function getSettings() {
  return run('get_settings', authed());
}
export async function saveSchool(input) {
  return call('save_school', input);
}
export async function saveClasses(input) {
  return call('save_classes', input);
}
export async function saveExams(input) {
  return call('save_exams', input);
}
export async function saveDeviceCode(input) {
  return call('save_device_code', input);
}

// ---- Backup ----
export async function backupStatus() {
  return call('backup_status');
}
export async function runBackupNow() {
  return call('run_backup_now');
}
export async function backupSaveFile(input) {
  return call('backup_save_file', input);
}
export async function restoreInspect(input) {
  return call('restore_inspect', input);
}
export async function restoreCommit(input) {
  return call('restore_commit', input);
}
export async function backupChangePassword(input) {
  return call('backup_change_password', input);
}
