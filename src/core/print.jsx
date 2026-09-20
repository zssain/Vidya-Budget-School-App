import { createContext, useCallback, useContext, useState } from 'react';
import { createPortal } from 'react-dom';
const PrintContext = createContext(null);
export function PrintProvider({ children }) {
  const [node, setNode] = useState(null);
  const printElement = useCallback(async (content) => {
    setNode(content);
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    await (document.fonts?.ready || Promise.resolve());
    window.print();
    setNode(null);
  }, []);
  return (
    <PrintContext.Provider value={{ printElement }}>
      {children}
      {node && createPortal(node, document.getElementById('print-root'))}
    </PrintContext.Provider>
  );
}
export function usePrint() {
  return useContext(PrintContext);
}
