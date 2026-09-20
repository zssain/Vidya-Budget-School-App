import { render } from '@testing-library/react';
import { AppProviders } from '../app/AppProviders.jsx';

export const principal = {
  id: 'u1',
  name: 'Test Principal',
  username: 'test@school',
  role: 'principal',
  permissions: [
    'students.view',
    'students.add',
    'students.edit',
    'attendance.mark',
    'marks.enter',
    'fees.view',
    'reports.view',
    'users.manage',
    'activity.view',
    'backup.manage',
    'settings.edit',
  ],
};
export function renderApp(node, user = principal) {
  return render(<AppProviders initialUser={user}>{node}</AppProviders>);
}
