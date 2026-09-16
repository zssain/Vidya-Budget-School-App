// Every export here maps 1:1 to a Tauri command in docs/API.md. Mock
// implementations are temporary (removed from P2.7 onward). Views call ONLY
// these functions, never invoke() or the mock directly.

import { mock } from './mock/index.js';
import { toAppError } from './errors.js';
import { clearToken, setToken } from './session.js';

async function call(name, arg) {
  try {
    return await mock[name](arg);
  } catch (e) {
    throw toAppError(e);
  }
}

// ---- App and setup ----
export async function appStatus() {
  return call('app_status');
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
export async function loadSampleSchool() {
  return call('load_sample_school');
}

// ---- Auth and account ----
export async function signIn(input) {
  const r = await call('sign_in', input);
  if (r && r.token) setToken(r.token);
  return r;
}
export async function setFirstPassword(input) {
  const r = await call('set_first_password', input);
  if (r && r.token) setToken(r.token);
  return r;
}
export async function signOut() {
  const r = await call('sign_out');
  clearToken();
  return r;
}
export async function currentUser() {
  return call('current_user');
}
export async function changePassword(input) {
  return call('change_password', input);
}
export async function setLanguage(input) {
  return call('set_language', input);
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

// ---- Staff logins ----
export async function listUsers() {
  return call('list_users');
}
export async function createUser(input) {
  return call('create_user', input);
}
export async function updateUser(input) {
  return call('update_user', input);
}
export async function resetUserPassword(input) {
  return call('reset_user_password', input);
}
export async function unlockUser(input) {
  return call('unlock_user', input);
}
export async function setUserActive(input) {
  return call('set_user_active', input);
}

// ---- Settings ----
export async function getSettings() {
  return call('get_settings');
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
