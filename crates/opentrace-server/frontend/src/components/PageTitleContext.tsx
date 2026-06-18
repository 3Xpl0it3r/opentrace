import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from 'react';

interface PageTitleState {
  title: string;
  subtitle?: string;
}

const PageTitleContext = createContext<{
  setPageTitle: (title: string, subtitle?: string) => void;
}>({ setPageTitle: () => {} });

export function PageTitleProvider({ children }: { children: ReactNode }) {
  const [titleState, setTitleState] = useState<PageTitleState>({ title: '' });
  const setPageTitle = useCallback((title: string, subtitle?: string) => {
    setTitleState((prev) => {
      if (prev.title === title && (prev.subtitle || '') === (subtitle || '')) {
        return prev;
      }
      return { title, subtitle };
    });
  }, []);
  const value = useMemo(() => ({ setPageTitle }), [setPageTitle]);

  return (
    <PageTitleContext.Provider value={value}>
      {children}
      <div id="page-title-data" data-title={titleState.title} data-subtitle={titleState.subtitle || ''} style={{ display: 'none' }} />
    </PageTitleContext.Provider>
  );
}

export function usePageTitle() {
  return useContext(PageTitleContext);
}
