import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../api/commands.js', () => ({
  listStudents: vi.fn(),
  exportStudentsXlsx: vi.fn(),
}));

import * as commands from '../../api/commands.js';
import { view } from './list.js';

const principal = {
  role: 'principal',
  permissions: ['students.view', 'students.add', 'fees.view'],
  sections: [],
};

const sampleStudent = {
  id: 'ADM/0001',
  adm: 'ADM/0001',
  roll: 1,
  name: "Sierra D'Souza <b>x</b>",
  ck: 'V-A',
  cls: 'V',
  sec: 'A',
  father: 'Ramesh Sharma',
  mobile: '9876543210',
  rte: false,
  transport: false,
  status: 'active',
  feeState: 'due',
  balance: 3600,
};

beforeEach(() => {
  vi.clearAllMocks();
});

describe('students list view', () => {
  it('renders the list and escapes a student name containing HTML', async () => {
    commands.listStudents.mockResolvedValue([sampleStudent]);
    const root = document.createElement('div');
    await view(root, { me: principal });
    // The name is shown as literal text (textContent keeps the <b> characters),
    // not injected as real markup.
    expect(root.textContent).toContain("Sierra D'Souza <b>x</b>");
    expect(root.querySelector('b > b')).toBeNull();
  });

  it('shows the empty state when there are no students', async () => {
    commands.listStudents.mockResolvedValue([]);
    const root = document.createElement('div');
    await view(root, { me: principal });
    expect(root.querySelector('.empty')).not.toBeNull();
  });
});
