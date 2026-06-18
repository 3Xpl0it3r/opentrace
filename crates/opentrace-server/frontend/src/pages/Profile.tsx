import { useState, useEffect } from 'react';
import { t } from '@/i18n';
import { Shield, ShieldCheck, Eye, Save } from 'lucide-react';
import { useToast } from '@/components/Toast';

const roleConfig: Record<string, { icon: typeof Shield; color: string; labelKey: string }> = {
  admin: { icon: ShieldCheck, color: 'var(--brand)', labelKey: 'users.admin' },
  editor: { icon: Shield, color: 'var(--accent)', labelKey: 'users.editor' },
  viewer: { icon: Eye, color: 'var(--text-muted)', labelKey: 'users.viewer' },
};

export default function Profile() {

  const { toast } = useToast();

  const [username, setUsername] = useState('');
  const [role, setRole] = useState('');
  const [nickname, setNickname] = useState('');
  const [email, setEmail] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    try {
      const user = JSON.parse(localStorage.getItem('user') || '{}');
      setUsername(user.username || '');
      setRole(user.role || 'viewer');
      setNickname(user.nickname || user.username || '');
      setEmail(user.email || '');
    } catch {}
  }, []);

  const handleSave = async () => {
    if (newPassword && newPassword !== confirmPassword) {
      toast({ title: 'Passwords do not match', variant: 'error' });
      return;
    }
    setSaving(true);
    // TODO: API call to update profile
    setTimeout(() => {
      setSaving(false);
      setNewPassword('');
      setConfirmPassword('');
      toast({ title: t('profile.saved'), variant: 'success' });
    }, 500);
  };

  const roleInfo = roleConfig[role] ?? roleConfig.viewer;
  const RoleIcon = roleInfo.icon;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-[22px] font-bold" style={{ letterSpacing: '-0.3px' }}>{t('profile.title')}</h1>
        <p className="text-[13px] mt-1" style={{ color: 'var(--text-secondary)' }}>{t('profile.subtitle')}</p>
      </div>

      {/* Profile Card */}
      <div className="border rounded-[14px] overflow-hidden" style={{ background: 'var(--bg-card)', borderColor: 'var(--border)' }}>
        {/* User Info Header */}
        <div className="px-6 py-5 border-b" style={{ borderColor: 'var(--border)' }}>
          <div className="flex items-center gap-4">
            <div className="w-16 h-16 rounded-full flex items-center justify-center text-2xl font-bold text-white" style={{ background: 'linear-gradient(135deg, var(--brand), var(--accent))' }}>
              {username[0]?.toUpperCase() || 'U'}
            </div>
            <div>
              <h2 className="text-[18px] font-bold" style={{ color: 'var(--text)' }}>{username}</h2>
              <div className="flex items-center gap-2 mt-1">
                <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-[20px] text-[11px] font-semibold" style={{ background: `${roleInfo.color}15`, color: roleInfo.color }}>
                  <RoleIcon size={12} /> {t(roleInfo.labelKey)}
                </span>
              </div>
            </div>
          </div>
        </div>

        {/* Edit Form */}
        <div className="p-6 space-y-5">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('profile.nickname')}</label>
              <input
                value={nickname}
                onChange={(e) => setNickname(e.target.value)}
                className="w-full px-3 py-2.5 rounded-[6px] text-[13px]"
                style={{ background: 'var(--bg-elevated)', border: '1px solid var(--border)', color: 'var(--text)', outline: 'none' }}
              />
            </div>
            <div>
              <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('profile.email')}</label>
              <input
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="user@example.com"
                className="w-full px-3 py-2.5 rounded-[6px] text-[13px]"
                style={{ background: 'var(--bg-elevated)', border: '1px solid var(--border)', color: 'var(--text)', outline: 'none' }}
              />
            </div>
          </div>

          <div>
            <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('profile.role')}</label>
            <div className="px-3 py-2.5 rounded-[6px] text-[13px] flex items-center gap-2" style={{ background: 'var(--bg-elevated)', border: '1px solid var(--border)', color: 'var(--text-muted)' }}>
              <RoleIcon size={14} style={{ color: roleInfo.color }} />
              {t(roleInfo.labelKey)}
              <span className="text-[10px] ml-auto" style={{ color: 'var(--text-muted)' }}>(cannot be changed)</span>
            </div>
          </div>

          <div className="border-t pt-5" style={{ borderColor: 'var(--border)' }}>
            <h3 className="text-[14px] font-semibold mb-4" style={{ color: 'var(--text)' }}>{t('profile.changePassword')}</h3>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('profile.newPassword')}</label>
                <input
                  type="password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  placeholder="••••••••"
                  className="w-full px-3 py-2.5 rounded-[6px] text-[13px]"
                  style={{ background: 'var(--bg-elevated)', border: '1px solid var(--border)', color: 'var(--text)', outline: 'none' }}
                />
              </div>
              <div>
                <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('profile.confirmPassword')}</label>
                <input
                  type="password"
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  placeholder="••••••••"
                  className="w-full px-3 py-2.5 rounded-[6px] text-[13px]"
                  style={{ background: 'var(--bg-elevated)', border: '1px solid var(--border)', color: 'var(--text)', outline: 'none' }}
                />
              </div>
            </div>
          </div>
        </div>

        {/* Save Button */}
        <div className="px-6 py-4 border-t flex justify-end" style={{ borderColor: 'var(--border)', background: 'var(--bg-elevated)' }}>
          <button
            onClick={handleSave}
            disabled={saving}
            className="flex items-center gap-2 px-5 py-2.5 rounded-[6px] text-[13px] font-semibold text-white disabled:opacity-50"
            style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))', boxShadow: '0 2px 10px rgba(108,92,231,.3)' }}
          >
            <Save size={14} />
            {saving ? t('profile.saving') : t('profile.save')}
          </button>
        </div>
      </div>
    </div>
  );
}
