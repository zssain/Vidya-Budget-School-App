// Assembles the mock command implementations, keyed by the snake_case command
// name from docs/API.md. TEMPORARY (see docs/prompts/P1.2).
import * as setup from './setup.js';
import * as sample from './sample.js';
import * as auth from './auth.js';
import * as students from './students.js';
import * as fees from './fees.js';
import * as reports from './reports.js';
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

  // list/get/add/update/mark_left students are real Rust commands (P3.2). Only
  // the Excel export is still mock, until its own prompt.
  export_students_xlsx: students.exportStudentsXlsx,

  // get/save/register attendance are real Rust commands (P3.4).

  // marks sheet/save + report cards are real Rust commands (P3.5).

  // register/account/collect/receipt/cancel/day_book are real Rust commands
  // (P3.3). Only the Excel exports are still mock, until their own prompt.
  export_dues_xlsx: fees.exportDuesXlsx,
  export_daybook_xlsx: fees.exportDaybookXlsx,

  reports_summary: reports.reportsSummary,
  export_class_summary_xlsx: reports.exportClassSummaryXlsx,

  list_activity: activity.listActivity,

  get_settings: settings.getSettings,
  save_school: settings.saveSchool,
  save_classes: settings.saveClasses,
  save_exams: settings.saveExams,
  save_device_code: settings.saveDeviceCode,

  backup_status: backup.backupStatus,
  run_backup_now: backup.runBackupNow,
  backup_save_file: backup.backupSaveFile,
  restore_inspect: backup.restoreInspect,
  restore_commit: backup.restoreCommit,
  backup_change_password: backup.backupChangePassword,
};
