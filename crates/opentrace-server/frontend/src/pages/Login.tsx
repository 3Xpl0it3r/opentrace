import { useState, type FormEvent } from 'react';
import { setLang, t, useLang } from '@/i18n';
import { useNavigate } from 'react-router-dom';
import { authApi, setToken } from '@/api/client';
import { Languages, Loader2 } from 'lucide-react';

export default function Login() {
  
  const currentLang = useLang();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      const res = await authApi.login(username, password);
      setToken(res.token);
      localStorage.setItem('user', JSON.stringify(res.user));
      navigate('/');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : t('login.failed'));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      className="min-h-screen flex items-center justify-center p-5 relative"
      style={{ background: 'var(--bg)' }}
    >
      <button
        type="button"
        onClick={() => setLang(currentLang === 'zh' ? 'en' : 'zh')}
        aria-label={t('menu.language')}
        title={currentLang === 'zh' ? 'English' : '中文'}
        className="absolute right-5 top-5 inline-flex h-9 w-9 items-center justify-center rounded-[6px] transition-colors hover:bg-[var(--bg-hover)]"
        style={{
          background: 'var(--bg-card)',
          border: '1px solid var(--border)',
          color: 'var(--text-secondary)',
        }}
      >
        <Languages size={17} />
      </button>

      <div className="w-full max-w-sm">
        {/* Logo */}
        <div className="text-center mb-8">
          <div
            className="inline-flex items-center justify-center w-14 h-14 rounded-2xl mb-4"
            style={{
              background: 'linear-gradient(135deg, var(--brand, #6C5CE7), var(--accent, #00CEC9))',
              boxShadow: '0 0 30px rgba(108,92,231,.3)',
            }}
          >
            <svg width="28" height="28" viewBox="0 0 32 32" fill="none">
              <rect x="4" y="4" width="24" height="24" rx="4" stroke="white" strokeWidth="2" fill="none" />
              <rect x="9" y="9" width="14" height="14" rx="2" fill="white" opacity="0.6" />
              <rect x="13" y="13" width="6" height="6" rx="1" fill="white" />
            </svg>
          </div>
          <h1 className="text-xl font-bold" style={{ color: 'var(--text)' }}>OpenTrace</h1>
          <p className="text-sm mt-1" style={{ color: 'var(--text-muted)' }}>{t('login.subtitle')}</p>
        </div>

        {/* Form */}
        <form
          onSubmit={handleSubmit}
          className="p-6 space-y-4"
          style={{
            background: 'var(--bg-surface)',
            border: '1px solid var(--border)',
            borderRadius: '20px',
          }}
        >
          {error && (
            <div
              className="px-3 py-2.5 rounded-lg text-sm"
              style={{
                background: 'var(--red-dim)',
                border: '1px solid rgba(255,77,106,.2)',
                color: 'var(--red)',
              }}
            >
              {error}
            </div>
          )}

          <div>
            <label className="block text-sm font-medium mb-1.5" style={{ color: 'var(--text-secondary)' }}>
              {t('login.username')}
            </label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              className="w-full px-3 py-2.5 rounded-lg text-sm outline-none transition-colors"
              style={{
                background: 'var(--bg-card)',
                border: '1px solid var(--border)',
                color: 'var(--text)',
              }}
              placeholder={t('login.username')}
              autoFocus
              required
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-1.5" style={{ color: 'var(--text-secondary)' }}>
              {t('login.password')}
            </label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="w-full px-3 py-2.5 rounded-lg text-sm outline-none transition-colors"
              style={{
                background: 'var(--bg-card)',
                border: '1px solid var(--border)',
                color: 'var(--text)',
              }}
              placeholder="••••••••"
              required
            />
          </div>

          <button
            type="submit"
            disabled={loading}
            className="w-full py-2.5 text-white text-sm font-medium rounded-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            style={{
              background: 'linear-gradient(135deg, var(--brand, #6C5CE7), var(--brand-dark, #4834d4))',
              boxShadow: '0 2px 10px rgba(108,92,231,.3)',
            }}
          >
            {loading ? <Loader2 size={16} className="animate-spin" /> : null}
            {loading ? t('login.signingIn') : t('login.signIn')}
          </button>
        </form>

        <p className="text-center text-xs mt-4" style={{ color: 'var(--text-muted)' }}>
          {t('login.defaultCreds')}
        </p>
      </div>
    </div>
  );
}
