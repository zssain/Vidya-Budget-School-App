import { render, screen } from '@testing-library/react';
import { expect, it } from 'vitest';
import { AppProviders } from '../app/AppProviders.jsx';
import { Shell } from '../views/Shell.jsx';
it("a teacher's navigation hides administration and finance", () => {
  const teacher = {
    name: 'Teacher',
    username: 'teacher@school',
    role: 'teacher',
    permissions: ['students.view', 'attendance.mark', 'marks.enter'],
  };
  render(
    <AppProviders initialUser={teacher}>
      <Shell />
    </AppProviders>,
  );
  for (const label of ['Fees', 'Reports', 'Staff logins', 'Backup', 'Settings'])
    expect(screen.queryByText(label)).not.toBeInTheDocument();
});
