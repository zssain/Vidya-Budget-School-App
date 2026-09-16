// Assembles the mock command implementations, keyed by the snake_case command
// name from docs/API.md. TEMPORARY (see docs/prompts/P1.2).
import * as setup from './setup.js';
import * as sample from './sample.js';
import * as auth from './auth.js';
import * as students from './students.js';
import * as attendance from './attendance.js';
import * as marks from './marks.js';
import * as fees from './fees.js';
import * as reports from './reports.js';
import * as users from './users.js';
import * as activity from './activity.js';
import * as backup from './backup.js';
import * as settings from './settings.js';
import * as home from './home.js';

export const mock = {
  app_status: setup.appStatus,
  get_device_id: setup.getDeviceId,
  activate: setup.activate,
  wizard_create_school: setup.wizardCreateSchool,
  load_sample_school: sample.loadSampleSchool,

  sign_in: auth.signIn,
  set_first_password: auth.setFirstPassword,
  sign_out: auth.signOut,
  current_user: auth.currentUser,
  change_password: auth.changePassword,
  set_language: auth.setLanguage,

  home_principal: home.homePrincipal,
  home_accountant: home.homeAccountant,
  home_teacher: home.homeTeacher,

  list_students: students.listStudents,
  get_student: students.getStudent,
  add_student: students.addStudent,
  update_student: students.updateStudent,
  export_students_xlsx: students.exportStudentsXlsx,

  get_attendance: attendance.getAttendance,
  save_attendance: attendance.saveAttendance,
  attendance_register: attendance.attendanceRegister,

  get_marks_sheet: marks.getMarksSheet,
  save_marks: marks.saveMarks,
  get_report_card: marks.getReportCard,
  get_class_report_cards: marks.getClassReportCards,

  fee_register: fees.feeRegister,
  get_fee_account: fees.getFeeAccount,
  collect_fee: fees.collectFee,
  get_receipt: fees.getReceipt,
  cancel_receipt: fees.cancelReceipt,
  day_book: fees.dayBook,
  export_dues_xlsx: fees.exportDuesXlsx,
  export_daybook_xlsx: fees.exportDaybookXlsx,

  reports_summary: reports.reportsSummary,
  export_class_summary_xlsx: reports.exportClassSummaryXlsx,

  list_users: users.listUsers,
  create_user: users.createUser,
  update_user: users.updateUser,
  reset_user_password: users.resetUserPassword,
  unlock_user: users.unlockUser,
  set_user_active: users.setUserActive,

  list_activity: activity.listActivity,

  get_settings: settings.getSettings,
  save_school: settings.saveSchool,
  save_classes: settings.saveClasses,
  save_exams: settings.saveExams,
  save_device_code: settings.saveDeviceCode,

  backup_status: backup.backupStatus,
  backup_save_file: backup.backupSaveFile,
  restore_inspect: backup.restoreInspect,
  restore_commit: backup.restoreCommit,
  backup_change_password: backup.backupChangePassword,
};
