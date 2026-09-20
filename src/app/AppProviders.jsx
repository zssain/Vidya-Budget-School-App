import { I18nProvider } from '../core/i18n.jsx';
import { PrintProvider } from '../core/print.jsx';
import { RouterProvider } from '../core/router.jsx';
import { SessionProvider } from '../core/session.jsx';
import { ModalProvider, ToastProvider } from '../core/ui.jsx';
export function AppProviders({ children, initialUser }) {
  return (
    <I18nProvider>
      <ToastProvider>
        <ModalProvider>
          <PrintProvider>
            <SessionProvider initialUser={initialUser}>
              <RouterProvider>{children}</RouterProvider>
            </SessionProvider>
          </PrintProvider>
        </ModalProvider>
      </ToastProvider>
    </I18nProvider>
  );
}
