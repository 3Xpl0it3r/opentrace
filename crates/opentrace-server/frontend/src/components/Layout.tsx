import { useState, useEffect } from 'react';
import { Outlet, NavLink, Link, useLocation } from 'react-router-dom';
import { t, useLang } from '@/i18n';

import {
  LayoutDashboard,
  Server,
  Database,
  Users,
  Sun,
  Moon,
  LogOut,
  Search,
  Bell,
  Menu,
  Settings,
} from 'lucide-react';

interface NavSection {
  label: string;
  items: { to: string; label: string; icon: typeof LayoutDashboard }[];
}

function getNavSections(): NavSection[] {
  return [
    {
      label: t('nav.overview'),
      items: [{ to: '/', label: t('nav.dashboard'), icon: LayoutDashboard }],
    },
    {
      label: t('nav.management'),
      items: [
        { to: '/agents', label: t('nav.agents'), icon: Server },
        { to: '/sinks', label: t('nav.sinks'), icon: Database },
      ],
    },
    {
      label: t('nav.system'),
      items: [
        { to: '/users', label: t('nav.users'), icon: Users },
        { to: '/system-config', label: t('nav.systemConfig'), icon: Settings },
      ],
    },
  ];
}



function getStoredUser() {
  try {
    const raw = localStorage.getItem('user');
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function getTheme(): 'dark' | 'light' {
  return localStorage.getItem('ot-theme') === 'light' ? 'light' : 'dark';
}

export default function Layout() {

  
  useLang();
  const [user, setUser] = useState(getStoredUser);
  const [theme, setTheme] = useState<'dark' | 'light'>(getTheme);
  const [sidebarOpen, setSidebarOpen] = useState(false);


  const location = useLocation();
  const [pageTitle, setPageTitle] = useState('');
  const [pageSubtitle, setPageSubtitle] = useState('');

  useEffect(() => {
    const readTitle = () => {
      const el = document.getElementById('page-title-data');
      if (el) {
        setPageTitle(el.dataset.title || '');
        setPageSubtitle(el.dataset.subtitle || '');
      }
    };
    readTitle();
    const observer = new MutationObserver(readTitle);
    const target = document.getElementById('page-title-data');
    if (target) observer.observe(target, { attributes: true, attributeFilter: ['data-title', 'data-subtitle'] });
    return () => observer.disconnect();
  }, [location.pathname]);



  useEffect(() => {
    setUser(getStoredUser());
  }, [location]);

  useEffect(() => {
    const root = document.documentElement;
    root.setAttribute('data-theme', theme);
    localStorage.setItem('ot-theme', theme);
  }, [theme]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        // focus search
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, []);

  // Close sidebar on route change (mobile)
  useEffect(() => {
    setSidebarOpen(false);
  }, [location.pathname]);

  const toggleTheme = () =>
    setTheme((t) => (t === 'dark' ? 'light' : 'dark'));

  const logout = () => {
    localStorage.removeItem('token');
    localStorage.removeItem('user');
    window.location.href = '/login';
  };

  const navSections = getNavSections();
  const currentNav = navSections
    .flatMap((section) => section.items.map((item) => ({ ...item, section: section.label })))
    .find((item) => (item.to === '/' ? location.pathname === '/' : location.pathname.startsWith(item.to)));
  const topbarTitle = pageTitle || currentNav?.label || 'OpenTrace';
  const topbarSubtitle = pageSubtitle || currentNav?.section || '';

  return (
    <div className="flex h-screen overflow-hidden" style={{ background: 'var(--bg)' }}>
      {/* Mobile overlay */}
      {sidebarOpen && (
        <div
          className="fixed inset-0 bg-black/50 z-40 lg:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      {/* Sidebar */}
      <aside
        className={`fixed lg:static inset-y-0 left-0 z-50 w-60 flex flex-col transition-transform duration-200 lg:translate-x-0 ${
          sidebarOpen ? 'translate-x-0' : '-translate-x-full'
        }`}
        style={{
          background: 'var(--bg-surface)',
          borderRight: '1px solid var(--border)',
        }}
      >
        {/* Brand */}
        <div
          className="flex h-16 min-h-[64px] items-center gap-3 px-5"
          style={{ borderBottom: '1px solid var(--border)' }}
        >
          <div
            className="w-9 h-9 rounded-[10px] flex items-center justify-center text-sm font-bold text-white"
            style={{
              background: 'linear-gradient(135deg, var(--brand), var(--accent))',
              boxShadow: '0 0 20px rgba(108,92,231,.3)',
            }}
          >
            OT
          </div>
          <div className="flex flex-col">
            <span className="text-[15px] font-bold tracking-tight" style={{ color: 'var(--text)' }}>
              OpenTrace
            </span>
            <span
              className="text-[10px] tracking-[1.5px]"
              style={{ color: 'var(--text-muted)' }}
            >
              v0.0.0
            </span>
          </div>
        </div>

        {/* Nav */}
        <nav className="flex-1 overflow-y-auto px-2.5 py-3">
          {getNavSections().map((section) => (
            <div key={section.label}>
              <div
                className="text-[10px] font-semibold uppercase tracking-[1.5px] px-3 pt-4 pb-1.5"
                style={{ color: 'var(--text-muted)' }}
              >
                {section.label}
              </div>
              {section.items.map(({ to, label, icon: Icon }) => (
                <NavLink
                  key={to}
                  to={to}
                  end={to === '/'}
                  className={({ isActive }) =>
                    `flex items-center gap-2.5 px-3 py-2.5 rounded-md text-[13px] font-medium transition-all relative ${
                      isActive ? '' : ''
                    }`
                  }
                  style={({ isActive }) =>
                    isActive
                      ? {
                          background: 'linear-gradient(135deg, rgba(108,92,231,.15), rgba(0,206,201,.08))',
                          color: 'var(--text)',
                          border: '1px solid rgba(108,92,231,.2)',
                        }
                      : {
                          color: 'var(--text-secondary)',
                        }
                  }
                >
                  {({ isActive }) => (
                    <>
                      {isActive && (
                        <span
                          className="absolute -left-2.5 top-1/2 -translate-y-1/2 w-[3px] h-5 rounded-r"
                          style={{ background: 'var(--brand)' }}
                        />
                      )}
                      <Icon size={18} className={isActive ? 'opacity-100' : 'opacity-70'} />
                      {label}
                    </>
                  )}
                </NavLink>
              ))}
            </div>
          ))}
        </nav>

        {/* Sidebar footer */}
        <Link
          to="/profile"
          className="flex items-center gap-2.5 px-4 py-3 hover:bg-[var(--bg-hover)] transition-colors cursor-pointer"
          style={{ borderTop: '1px solid var(--border)', textDecoration: 'none', color: 'inherit' }}
        >
          <div
            className="w-8 h-8 rounded-full flex items-center justify-center text-xs font-semibold text-white"
            style={{ background: 'linear-gradient(135deg, var(--brand), var(--accent))' }}
          >
            {user?.username?.[0]?.toUpperCase() ?? '?'}
          </div>
          <div className="flex flex-col min-w-0">
            <span className="text-xs font-semibold truncate" style={{ color: 'var(--text)' }}>
              {user?.username}
            </span>
            <span className="text-[10px] capitalize" style={{ color: 'var(--text-muted)' }}>
              {user?.role}
            </span>
          </div>
          <button
            onClick={(e) => { e.preventDefault(); e.stopPropagation(); logout(); }}
            className="ml-auto p-1.5 rounded transition-colors hover:opacity-80"
            style={{ color: 'var(--text-muted)' }}
            title={t("menu.signOut")}
          >
            <LogOut size={14} />
          </button>
        </Link>
      </aside>

      {/* Main area */}
      <div className="flex-1 flex flex-col min-w-0 h-screen overflow-hidden">
        {/* Top bar */}
        <header
          className="h-16 min-h-[64px] flex items-center px-6 gap-4"
          style={{
            background: 'var(--bg-surface)',
            borderBottom: '1px solid var(--border)',
          }}
        >
          {/* Mobile menu button */}
          <button
            className="lg:hidden p-1.5 rounded"
            style={{ color: 'var(--text-secondary)' }}
            onClick={() => setSidebarOpen(true)}
          >
            <Menu size={20} />
          </button>

          <div className="min-w-0 flex-1">
            <div className="truncate text-[22px] font-bold leading-tight tracking-[-0.3px]" style={{ color: 'var(--text)' }}>
              {topbarTitle}
            </div>
            {topbarSubtitle && (
              <div className="mt-1 truncate text-[13px]" style={{ color: 'var(--text-secondary)' }}>
                {topbarSubtitle}
              </div>
            )}
          </div>

          {/* Search */}
          <div
            className="hidden sm:flex h-9 min-w-[240px] items-center gap-2 rounded-[6px] px-3 text-xs"
            style={{
              background: 'var(--bg-card)',
              border: '1px solid var(--border)',
              color: 'var(--text-muted)',
            }}
          >
            <Search size={14} />
            <input
              type="text"
              placeholder={t("common.search")}
              className="min-w-0 flex-1 bg-transparent border-none outline-none text-xs"
              style={{ color: 'var(--text)' }}
            />
            <kbd
              className="font-mono text-[10px] px-1.5 py-0.5 rounded-[4px]"
              style={{
                background: 'var(--bg-elevated)',
                border: '1px solid var(--border)',
                color: 'var(--text-muted)',
              }}
            >
              /
            </kbd>
          </div>


          {/* Theme toggle */}
          <button
            onClick={toggleTheme}
            className="h-9 w-9 flex items-center justify-center rounded-[6px] relative overflow-hidden transition-colors"
            style={{
              border: '1px solid var(--border)',
              background: 'var(--bg-card)',
              color: 'var(--text-secondary)',
            }}
            title="Toggle theme"
          >
            <Sun
              size={16}
              className="absolute transition-transform duration-300"
              style={{
                transform: theme === 'light' ? 'rotate(0) scale(1)' : 'rotate(90deg) scale(0)',
                opacity: theme === 'light' ? 1 : 0,
              }}
            />
            <Moon
              size={16}
              className="absolute transition-transform duration-300"
              style={{
                transform: theme === 'dark' ? 'rotate(0) scale(1)' : 'rotate(-90deg) scale(0)',
                opacity: theme === 'dark' ? 1 : 0,
              }}
            />
          </button>

          {/* Notification bell */}
          <button
            className="h-9 w-9 flex items-center justify-center rounded-[6px] transition-colors"
            style={{
              border: '1px solid var(--border)',
              background: 'var(--bg-card)',
              color: 'var(--text-secondary)',
            }}
            title="Notifications"
          >
            <Bell size={16} />
          </button>
        </header>

        {/* Content area */}
        <main className="flex-1 overflow-y-auto overflow-x-hidden p-6">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
