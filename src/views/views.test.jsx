import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { renderApp } from '../test/render.jsx';
vi.mock('../api/commands.js', () => {
  const ok = () => Promise.resolve({ id: 'one', name: "Sierra D'Souza <b>x</b>" });
  const office = {
    shape: 'office',
    id: 'one',
    admNo: 'ADM/0001',
    name: "Sierra D'Souza <b>x</b>",
    gender: 'Female',
    dob: '2015-01-01',
    father: 'Parent',
    mother: '',
    mobile: '9876543210',
    locality: '',
    className: 'V',
    sectionName: 'A',
    sectionId: 'sec',
    roll: 1,
    status: 'active',
    category: 'General',
    rte: false,
    transport: false,
    aadhaarCollected: false,
    apaarCreated: false,
    admittedOn: '2026-04-01',
    concession: 0,
    fee: { due: 100, paid: 0, balance: 100, state: 'due' },
  };
  return {
    homePrincipal: () =>
      Promise.resolve({
        name: 'Principal',
        students: 0,
        classes: 0,
        sections: 0,
        pending: [],
        attendance: { done: 0, p: 0, a: 0, l: 0 },
        fees: { due: 0, paid: 0, pending: 0, unpaid: 0, part: 0, pctCollected: 0 },
        recentActivity: [],
      }),
    homeAccountant: () =>
      Promise.resolve({
        name: 'Accountant',
        receiptsToday: 0,
        collectedToday: 0,
        devicePrefix: 'PC',
        byMode: { Cash: 0, UPI: 0, Cheque: 0 },
        fees: { pending: 0, paid: 0, unpaid: 0, part: 0 },
        topDue: [],
      }),
    homeTeacher: () => Promise.resolve({ name: 'Teacher', classes: [], pending: [] }),
    listStudents: () => Promise.resolve({ items: [office], truncated: false }),
    getStudent: () => Promise.resolve({ ...office, receipts: [] }),
    addStudent: vi.fn(),
    updateStudent: vi.fn(),
    markStudentLeft: vi.fn(),
    getAttendance: () =>
      Promise.resolve({
        sectionId: 'sec',
        sectionLabel: 'V-A',
        date: '2026-09-20',
        students: [
          { id: 'one', adm: 'ADM/0001', roll: 1, name: 'Aman' },
          { id: 'two', adm: 'ADM/0002', roll: 2, name: 'Bina' },
        ],
        marks: {},
        savedBy: undefined,
        savedAt: undefined,
        readOnlyReason: undefined,
      }),
    saveAttendance: vi.fn(),
    attendanceRegister: vi.fn(),
    getMarksSheet: () =>
      Promise.resolve({
        exam: { id: 'ex1', name: 'Half Yearly', max: 100 },
        exams: [{ id: 'ex1', name: 'Half Yearly' }],
        sectionId: 'sec',
        subjects: [{ id: 'sub1', name: 'Hindi' }],
        students: [
          {
            id: 'one',
            adm: 'ADM/0001',
            roll: 1,
            name: 'Aman',
            marks: {},
            total: '—',
            grade: '—',
            percent: '—',
          },
        ],
        savedBy: undefined,
        savedAt: undefined,
        editable: true,
      }),
    saveMarks: vi.fn(),
    getReportCard: () =>
      Promise.resolve({
        school: { name: 'School', session: '2026-27' },
        student: {
          name: 'Student',
          ck: 'V-A',
          roll: 1,
          adm: 'one',
          father: 'Parent',
          dob: '2015-01-01',
          sectionId: 'sec',
        },
        exams: [],
        rows: [],
        totals: [],
        attendance: { p: 0, t: 0, pct: null },
      }),
    getClassReportCards: () => Promise.resolve([]),
    feeRegister: () =>
      Promise.resolve({
        totals: { due: 0, paid: 0, pending: 0, payingCount: 0, unpaid: 0, part: 0 },
        collectedToday: 0,
        receiptsToday: 0,
        session: '2026-27',
        terms: 3,
        devicePrefix: 'PC',
        list: [],
      }),
    getFeeAccount: () =>
      Promise.resolve({
        studentId: 'one',
        name: 'Student',
        ck: 'V-A',
        roll: 1,
        due: 100,
        paid: 0,
        balance: 100,
        terms: 3,
        oneTerm: 50,
        rte: false,
      }),
    collectFee: vi.fn(),
    cancelReceipt: vi.fn(),
    getReceipt: ok,
    dayBook: () =>
      Promise.resolve({
        date: '2026-09-17',
        modes: [],
        total: 0,
        count: 0,
        receipts: [],
        school: { name: 'School' },
      }),
    reportsSummary: () =>
      Promise.resolve({
        enrolment: 0,
        boys: 0,
        girls: 0,
        rte: 0,
        aadhaarPct: null,
        aadhaarPending: 0,
        attendance30: null,
        attendanceMarks: 0,
        byClass: [],
        cats: [],
        feeStatus: [],
      }),
    exportClassSummaryXlsx: vi.fn(),
    listUsers: () => Promise.resolve([]),
    listActivity: () => Promise.resolve([]),
    backupStatus: ok,
    backupSaveFile: vi.fn(),
    backupChangePassword: vi.fn(),
    restoreInspect: vi.fn(),
    restoreCommit: vi.fn(),
    getSettings: () =>
      Promise.resolve({
        school: { name: 'School', addr: '', udise: '', board: '', session: '2026-27', phone: '' },
        classes: [{ id: 'cv', name: 'V', sortOrder: 5, sections: [{ id: 'sec', name: 'A' }], subjects: [] }],
        transportFee: 0,
        terms: 3,
        exams: [],
        license: { schoolCode: 'school', activationCode: 'DEMO', maxUsers: 40 },
        deviceId: 'device',
        deviceCode: 'PC',
      }),
    saveSchool: vi.fn(),
    saveClasses: vi.fn(),
    saveExams: vi.fn(),
    saveDeviceCode: vi.fn(),
    currentUser: ok,
    signIn: ok,
    signOut: ok,
    setFirstPassword: ok,
    getDeviceId: () => Promise.resolve({ deviceId: 'VD-TEST' }),
    wizardCreateSchool: vi.fn(),
    loadSampleSchool: vi.fn(),
    changePassword: vi.fn(),
  };
});
import { Home } from './Home.jsx';
import { StudentList } from './students/StudentList.jsx';
import { StudentDetail } from './students/StudentDetail.jsx';
import { StudentForm } from './students/StudentForm.jsx';
import { Attendance } from './Attendance.jsx';
import { Marks } from './Marks.jsx';
import { ReportCard } from './ReportCard.jsx';
import { FeeRegister } from './fees/FeeRegister.jsx';
import { CollectFee } from './fees/CollectFee.jsx';
import { Receipt } from './fees/Receipt.jsx';
import { DayBook } from './fees/DayBook.jsx';
import { Reports } from './Reports.jsx';
import { Users } from './Users.jsx';
import { Activity } from './Activity.jsx';
import { Backup } from './Backup.jsx';
import { Settings } from './Settings.jsx';
import { Welcome } from './setup/Welcome.jsx';
import { Wizard } from './setup/Wizard.jsx';
import { Credentials } from './setup/Credentials.jsx';
import { Login } from './Login.jsx';
import { Password } from './Password.jsx';
import { Shell } from './Shell.jsx';
describe('feature views', () => {
  it.each([
    ['home', <Home key="home" />],
    ['student list', <StudentList key="student-list" />],
    ['student detail', <StudentDetail key="student-detail" studentId="one" />],
    ['student form', <StudentForm key="student-form" />],
    ['attendance', <Attendance key="attendance" />],
    ['marks', <Marks key="marks" />],
    ['report card', <ReportCard key="report-card" />],
    ['fee register', <FeeRegister key="fee-register" />],
    ['collect fee', <CollectFee key="collect-fee" studentId="one" />],
    ['receipt', <Receipt key="receipt" receiptId="one" />],
    ['day book', <DayBook key="day-book" />],
    ['reports', <Reports key="reports" />],
    ['users', <Users key="users" />],
    ['activity', <Activity key="activity" />],
    ['backup', <Backup key="backup" />],
    ['settings', <Settings key="settings" />],
    ['welcome', <Welcome key="welcome" onReady={() => {}} onSetup={() => {}} />],
    ['wizard', <Wizard key="wizard" onBack={() => {}} onComplete={() => {}} />],
    [
      'credentials',
      <Credentials
        key="credentials"
        result={{ principalUsername: 'principal@school', credentials: [] }}
        onSignIn={() => {}}
      />,
    ],
    ['login', <Login key="login" />],
    ['password', <Password key="password" pendingToken="pending" />],
    ['shell', <Shell key="shell" />],
  ])('%s renders without throwing', async (_name, node) => {
    renderApp(node);
    expect((await screen.findAllByRole('heading')).length).toBeGreaterThan(0);
  });
  it('renders hostile student data as literal text', async () => {
    const { container } = renderApp(<StudentList />);
    expect(await screen.findByText("Sierra D'Souza <b>x</b>")).toBeInTheDocument();
    expect(container.querySelector('b')).toBeNull();
  });
  it('shows a validation error beside its field', async () => {
    const commands = await import('../api/commands.js');
    commands.collectFee.mockRejectedValueOnce({
      kind: 'validation',
      message: 'Enter a valid amount.',
      field: 'amount',
    });
    const user = userEvent.setup();
    renderApp(<CollectFee studentId="one" />);
    await user.clear(await screen.findByLabelText(/Amount received/));
    await user.type(screen.getByLabelText(/Amount received/), '1');
    await user.click(screen.getByRole('button', { name: 'Save and print receipt' }));
    expect(await screen.findByText('Enter a valid amount.')).toBeInTheDocument();
  });
  it('cycles an attendance card P → A → L and marks all present', async () => {
    const user = userEvent.setup();
    renderApp(<Attendance />);
    const card = await screen.findByRole('button', { name: /Aman/ });
    await user.click(card);
    expect(await screen.findByRole('button', { name: 'Aman P' })).toBeInTheDocument();
    await user.click(card);
    expect(await screen.findByRole('button', { name: 'Aman A' })).toBeInTheDocument();
    await user.click(card);
    expect(await screen.findByRole('button', { name: 'Aman L' })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /Mark all/ }));
    expect(await screen.findByRole('button', { name: 'Aman P' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Bina P' })).toBeInTheDocument();
  });
  it('flags the cells returned in a marks validation error', async () => {
    const commands = await import('../api/commands.js');
    commands.saveMarks.mockRejectedValueOnce({
      kind: 'validation',
      messageKey: 'marks.error.cells',
      params: { cells: JSON.stringify([{ studentId: 'one', subjectId: 'sub1' }]) },
      message: 'Fix these marks.',
    });
    const user = userEvent.setup();
    renderApp(<Marks />);
    const cell = await screen.findByLabelText('Aman Hindi');
    await user.type(cell, '150');
    await user.click(screen.getByRole('button', { name: /Save marks/ }));
    expect(await screen.findByText('Fix these marks.')).toBeInTheDocument();
    expect(cell.className).toMatch(/bad/);
  });
  it('keeps the dashboard in the shell scroll surface', async () => {
    const { container } = renderApp(<Home />);
    expect(await screen.findByText(/Principal/)).toBeInTheDocument();
    expect(container.querySelector('main.view > .inner.stack')).toBeInTheDocument();
  });
});
