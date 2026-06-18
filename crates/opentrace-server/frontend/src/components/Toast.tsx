import { useState, useEffect, useCallback, createContext, useContext, useRef } from 'react';
import { X, CheckCircle, XCircle, Info } from 'lucide-react';

interface ToastItem {
  id: number;
  title: string;
  description?: string;
  variant: 'success' | 'error' | 'info';
}

interface ToastOptions {
  title: string;
  description?: string;
  variant?: 'success' | 'error' | 'info';
}

const ToastContext = createContext<{ toast: (opts: ToastOptions) => void }>({
  toast: () => {},
});

let _globalId = 0;

export function useToast() {
  return useContext(ToastContext);
}

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const timers = useRef<Map<number, ReturnType<typeof setTimeout>>>(new Map());

  const remove = useCallback((id: number) => {
    const t = timers.current.get(id);
    if (t) {
      clearTimeout(t);
      timers.current.delete(id);
    }
    setToasts((prev) => prev.filter((x) => x.id !== id));
  }, []);

  const toast = useCallback(
    (opts: ToastOptions) => {
      const id = ++_globalId;
      const item: ToastItem = {
        id,
        title: opts.title,
        description: opts.description,
        variant: opts.variant ?? 'info',
      };
      setToasts((prev) => [...prev, item]);
      const timer = setTimeout(() => remove(id), 3000);
      timers.current.set(id, timer);
    },
    [remove]
  );

  useEffect(() => {
    return () => {
      timers.current.forEach((t) => clearTimeout(t));
    };
  }, []);

  const iconFor = (variant: string) => {
    switch (variant) {
      case 'success':
        return <CheckCircle size={18} style={{ color: 'var(--green)' }} />;
      case 'error':
        return <XCircle size={18} style={{ color: 'var(--red)' }} />;
      default:
        return <Info size={18} style={{ color: 'var(--blue)' }} />;
    }
  };

  const borderLeftFor = (variant: string) => {
    switch (variant) {
      case 'success':
        return '3px solid var(--green)';
      case 'error':
        return '3px solid var(--red)';
      default:
        return '3px solid var(--blue)';
    }
  };

  return (
    <ToastContext.Provider value={{ toast }}>
      {children}
      <div className="fixed top-5 right-5 z-[100] flex flex-col gap-2 pointer-events-none">
        {toasts.map((t) => (
          <div
            key={t.id}
            className="pointer-events-auto w-80 p-3 flex items-start gap-3"
            style={{
              background: 'var(--bg-surface)',
              border: '1px solid var(--border)',
              borderLeft: borderLeftFor(t.variant),
              borderRadius: '10px',
              boxShadow: 'var(--shadow-modal, 0 8px 32px rgba(0,0,0,.5))',
              animation: 'toastSlideIn 0.3s ease-out',
            }}
          >
            <div className="shrink-0 mt-0.5">{iconFor(t.variant)}</div>
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium" style={{ color: 'var(--text)' }}>
                {t.title}
              </p>
              {t.description && (
                <p className="text-xs mt-0.5" style={{ color: 'var(--text-muted)' }}>
                  {t.description}
                </p>
              )}
            </div>
            <button
              onClick={() => remove(t.id)}
              className="shrink-0 p-0.5 transition-colors hover:opacity-80"
              style={{ color: 'var(--text-muted)' }}
            >
              <X size={14} />
            </button>
          </div>
        ))}
      </div>
      <style>{`
        @keyframes toastSlideIn {
          from { opacity: 0; transform: translateX(40px); }
          to { opacity: 1; transform: translateX(0); }
        }
      `}</style>
    </ToastContext.Provider>
  );
}
