import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { I18nProvider } from './i18n.jsx';
import { ModalProvider, useConfirm, useModal } from './ui.jsx';
function ConfirmHarness() {
  const confirm = useConfirm();
  return (
    <button
      onClick={async () => {
        document.body.dataset.answer = String(
          await confirm({ title: 'Delete?', message: 'Continue?', confirmLabel: 'Yes' }),
        );
      }}
    >
      Open confirm {/* i18n-ignore */}
    </button>
  );
}
function Locked() {
  const { openModal } = useModal();
  return (
    <button onClick={() => openModal({ title: 'Locked', locked: true, body: <button>Inside</button> })}>
      Open locked {/* i18n-ignore */}
    </button>
  );
}
function FocusReturn() {
  const { openModal, closeModal } = useModal();
  return (
    <button
      onClick={() =>
        openModal({
          title: 'Focus',
          body: <button onClick={closeModal}>Close focus {/* i18n-ignore */}</button>,
        })
      }
    >
      Open focus {/* i18n-ignore */}
    </button>
  );
}
const wrap = (node) =>
  render(
    <I18nProvider>
      <ModalProvider>{node}</ModalProvider>
    </I18nProvider>,
  );
describe('modal', () => {
  it('confirm resolves true and false', async () => {
    const user = userEvent.setup();
    wrap(<ConfirmHarness />);
    await user.click(screen.getByText('Open confirm'));
    await user.click(screen.getByText('Yes'));
    expect(document.body.dataset.answer).toBe('true');
    await user.click(screen.getByText('Open confirm'));
    await user.click(screen.getByText('Cancel'));
    expect(document.body.dataset.answer).toBe('false');
  });
  it('locked modal ignores Escape and restores focus after close behavior', async () => {
    const user = userEvent.setup();
    wrap(<Locked />);
    const opener = screen.getByText('Open locked');
    opener.focus();
    await user.click(opener);
    await user.keyboard('{Escape}');
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });
  it('returns focus to the opener after close', async () => {
    const user = userEvent.setup();
    wrap(<FocusReturn />);
    const opener = screen.getByText('Open focus');
    await user.click(opener);
    await user.click(screen.getByText('Close focus'));
    expect(opener).toHaveFocus();
  });
  it('renders the portal classes used by the dialog stylesheet', async () => {
    const user = userEvent.setup();
    wrap(<FocusReturn />);
    await user.click(screen.getByText('Open focus'));
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveClass('modal');
    expect(dialog.parentElement).toHaveClass('modal-back');
    expect(dialog.querySelector('.modal-body')).toBeInTheDocument();
  });
});
