import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useT } from './i18n.jsx';

const ToastContext = createContext(null);
const ModalContext = createContext(null);

export function ToastProvider({ children }) {
  const [item, setItem] = useState(null);
  const timer = useRef();
  const toast = useCallback((message, { kind = 'info' } = {}) => {
    clearTimeout(timer.current);
    setItem({ message, kind });
    timer.current = setTimeout(() => setItem(null), 3500);
  }, []);
  useEffect(() => () => clearTimeout(timer.current), []);
  return (
    <ToastContext.Provider value={toast}>
      {children}
      {item &&
        createPortal(
          <div className={`toast ${item.kind}`} role="status" aria-live="polite">
            {item.message}
          </div>,
          document.body,
        )}
    </ToastContext.Provider>
  );
}
export function useToast() {
  const toast = useContext(ToastContext);
  return { toast };
}

function Dialog({ modal, close }) {
  const box = useRef(null);
  useEffect(() => {
    const opener = document.activeElement;
    const focusable = () =>
      [
        ...box.current.querySelectorAll('button,input,select,textarea,[tabindex]:not([tabindex="-1"])'),
      ].filter((node) => !node.disabled);
    focusable()[0]?.focus();
    const keydown = (event) => {
      if (event.key === 'Escape' && !modal.locked) close();
      if (event.key === 'Tab') {
        const nodes = focusable();
        if (!nodes.length) return;
        const first = nodes[0];
        const last = nodes.at(-1);
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener('keydown', keydown);
    return () => {
      document.removeEventListener('keydown', keydown);
      opener?.focus();
    };
  }, [close, modal.locked]);
  return createPortal(
    <div className="modal-back" role="presentation">
      <section
        ref={box}
        className={`modal ${modal.wide ? 'wide' : ''}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby="modal-title"
      >
        <header>
          <h2 id="modal-title">{modal.title}</h2>
          {modal.subtitle && <p>{modal.subtitle}</p>}
        </header>
        <div className="modal-body">{modal.body}</div>
        {modal.footer && <footer>{modal.footer}</footer>}
      </section>
    </div>,
    document.body,
  );
}
export function ModalProvider({ children }) {
  const [modal, setModal] = useState(null);
  const closeModal = useCallback(() => setModal(null), []);
  const openModal = useCallback(
    (options) => {
      setModal(options);
      return { close: closeModal };
    },
    [closeModal],
  );
  return (
    <ModalContext.Provider value={{ openModal, closeModal }}>
      {children}
      {modal && <Dialog modal={modal} close={closeModal} />}
    </ModalContext.Provider>
  );
}
export function useModal() {
  return useContext(ModalContext);
}
export function useConfirm() {
  const { openModal, closeModal } = useModal();
  const t = useT();
  return useCallback(
    ({ title, message, confirmLabel = t('common.ok'), danger = false }) =>
      new Promise((resolve) => {
        const finish = (answer) => {
          closeModal();
          resolve(answer);
        };
        openModal({
          title,
          body: <p>{message}</p>,
          footer: (
            <div className="row g8 end">
              <button className="btn outline" onClick={() => finish(false)}>
                {t('common.cancel')}
              </button>
              <button className={`btn ${danger ? 'warn' : ''}`} onClick={() => finish(true)}>
                {confirmLabel}
              </button>
            </div>
          ),
        });
      }),
    [closeModal, openModal, t],
  );
}
