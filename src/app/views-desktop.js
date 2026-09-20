import { Home } from '../views/Home.jsx';
import { StudentList } from '../views/students/StudentList.jsx';
import { StudentDetail } from '../views/students/StudentDetail.jsx';
import { StudentForm } from '../views/students/StudentForm.jsx';
import { Attendance } from '../views/Attendance.jsx';
import { Marks } from '../views/Marks.jsx';
import { FeeRegister } from '../views/fees/FeeRegister.jsx';
import { CollectFee } from '../views/fees/CollectFee.jsx';
import { Receipt } from '../views/fees/Receipt.jsx';
import { DayBook } from '../views/fees/DayBook.jsx';
import { Reports } from '../views/Reports.jsx';
import { Users } from '../views/Users.jsx';
import { Activity } from '../views/Activity.jsx';
import { Backup } from '../views/Backup.jsx';
import { Settings } from '../views/Settings.jsx';
import { ReportCard } from '../views/ReportCard.jsx';
export const desktopViews = [
  { id: 'home', titleKey: 'nav.home', Component: Home, navIcon: 'home' },
  {
    id: 'students',
    titleKey: 'nav.students',
    Component: StudentList,
    navIcon: 'students',
    permission: 'students.view',
  },
  {
    id: 'student',
    titleKey: 'students.details',
    Component: StudentDetail,
    permission: 'students.view',
    nav: false,
  },
  {
    id: 'student-form',
    titleKey: 'students.newAdmission',
    Component: StudentForm,
    permission: 'students.add',
    nav: false,
  },
  {
    id: 'reportcard',
    titleKey: 'students.reportCard',
    Component: ReportCard,
    permission: 'reportcard.view',
    nav: false,
  },
  {
    id: 'attendance',
    titleKey: 'nav.attendance',
    Component: Attendance,
    navIcon: 'attendance',
    permission: 'attendance.mark',
  },
  { id: 'marks', titleKey: 'nav.marks', Component: Marks, navIcon: 'marks', permission: 'marks.enter' },
  { id: 'fees', titleKey: 'nav.fees', Component: FeeRegister, navIcon: 'fees', permission: 'fees.view' },
  {
    id: 'collect-fee',
    titleKey: 'common.collectFee',
    Component: CollectFee,
    permission: 'fees.collect',
    nav: false,
  },
  { id: 'receipt', titleKey: 'doc.feeReceipt', Component: Receipt, permission: 'fees.view', nav: false },
  { id: 'daybook', titleKey: 'fees.dayBook', Component: DayBook, permission: 'fees.view', nav: false },
  {
    id: 'reports',
    titleKey: 'nav.reports',
    Component: Reports,
    navIcon: 'reports',
    permission: 'reports.view',
  },
  { id: 'users', titleKey: 'nav.users', Component: Users, navIcon: 'users', permission: 'users.manage' },
  {
    id: 'activity',
    titleKey: 'nav.activity',
    Component: Activity,
    navIcon: 'activity',
    permission: 'activity.view',
  },
  { id: 'backup', titleKey: 'nav.backup', Component: Backup, navIcon: 'backup', permission: 'backup.run' },
  {
    id: 'settings',
    titleKey: 'nav.settings',
    Component: Settings,
    navIcon: 'settings',
    permission: 'settings.edit',
  },
];
