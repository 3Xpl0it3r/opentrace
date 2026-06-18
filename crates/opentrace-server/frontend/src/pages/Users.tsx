import { useState, useEffect } from 'react';
import { t } from '@/i18n';
import { Plus, Trash2, Edit3, Shield, ShieldCheck, Eye } from 'lucide-react';
import { usersApi, type User } from '@/api/client';
import { useToast } from '@/components/Toast';
import { usePageTitle } from '@/components/PageTitleContext';
import Modal from '@/components/Modal';
import ConfirmDialog from '@/components/ConfirmDialog';

const roleConfig: Record<string, { icon: typeof Shield; color: string; labelKey: string; descKey: string }> = {
  admin: { icon: ShieldCheck, color: 'var(--brand)', labelKey: 'users.admin', descKey: 'users.adminDesc' },
  editor: { icon: Shield, color: 'var(--accent)', labelKey: 'users.editor', descKey: '' },
  viewer: { icon: Eye, color: 'var(--text-muted)', labelKey: 'users.viewer', descKey: 'users.viewerDesc' },
};

export default function Users() {

  const { toast } = useToast();
  const { setPageTitle } = usePageTitle();
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);
  const [showCreate, setShowCreate] = useState(false);
  const [editTarget, setEditTarget] = useState<User | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<User | null>(null);

  // Create form
  const [newUsername, setNewUsername] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [newRole, setNewRole] = useState('viewer');
  const [creating, setCreating] = useState(false);

  // Edit form
  const [editPassword, setEditPassword] = useState('');
  const [editConfirmPassword, setEditConfirmPassword] = useState('');
  const [editRole, setEditRole] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setPageTitle(t('users.title'), t('users.subtitle'));
    return () => setPageTitle('');
  }, [setPageTitle]);

  const load = () => {
    setLoading(true);
    usersApi.list()
      .then(setUsers)
      .catch(() => toast({ title: 'Failed to load users', variant: 'error' }))
      .finally(() => setLoading(false));
  };

  useEffect(load, []);

  const handleCreate = async () => {
    if (!newUsername.trim() || !newPassword.trim()) return;
    setCreating(true);
    try {
      await usersApi.create({ username: newUsername.trim(), password: newPassword, role: newRole });
      toast({ title: 'User created', variant: 'success' });
      setShowCreate(false);
      setNewUsername(''); setNewPassword(''); setNewRole('viewer');
      load();
    } catch (err: unknown) {
      toast({ title: 'Create failed', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally {
      setCreating(false);
    }
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    try {
      await usersApi.remove(deleteTarget.id);
      toast({ title: 'User deleted', variant: 'success' });
      setDeleteTarget(null);
      load();
    } catch {
      toast({ title: 'Delete failed', variant: 'error' });
    }
  };

  const openEdit = (user: User) => {
    setEditTarget(user);
    setEditRole(user.role);
    setEditPassword('');
    setEditConfirmPassword('');
  };

  const handleSave = async () => {
    if (!editTarget) return;
    if (editPassword && editPassword !== editConfirmPassword) {
      toast({ title: 'Passwords do not match', variant: 'error' });
      return;
    }
    setSaving(true);
    // TODO: API call to update user role and password
    setTimeout(() => {
      setSaving(false);
      setEditTarget(null);
      toast({ title: 'User updated', variant: 'success' });
      load();
    }, 500);
  };

  // Check if current user is admin
  const currentUser = (() => {
    try { return JSON.parse(localStorage.getItem('user') || '{}'); } catch { return {}; }
  })();
  const isAdmin = currentUser?.role === 'admin';

  return (
    <div className="space-y-6">
      <div className="flex justify-end">
        {isAdmin && (
          <button
            onClick={() => setShowCreate(true)}
            className="flex items-center gap-2 px-4 py-2 rounded-[6px] text-[13px] font-semibold text-white"
            style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))', boxShadow: '0 2px 10px rgba(108,92,231,.3)' }}
          >
            <Plus size={14} /> {t('users.addUser')}
          </button>
        )}
      </div>

      {/* Users Table */}
      {loading ? (
        <div className="space-y-3">
          {[1, 2, 3].map((i) => <div key={i} className="h-16 rounded-[14px] animate-pulse" style={{ background: 'var(--bg-card)' }} />)}
        </div>
      ) : users.length === 0 ? (
        <div className="text-center py-12" style={{ color: 'var(--text-muted)' }}>{t('users.noUsers')}</div>
      ) : (
        <div className="border rounded-[14px] overflow-hidden" style={{ background: 'var(--bg-card)', borderColor: 'var(--border)' }}>
          <table className="w-full">
            <thead>
              <tr style={{ borderBottom: '1px solid var(--border)' }}>
                <th className="text-left px-5 py-3 text-[11px] font-semibold uppercase tracking-[0.8px]" style={{ color: 'var(--text-muted)', background: 'var(--bg-elevated)' }}>{t('users.user')}</th>
                <th className="text-left px-5 py-3 text-[11px] font-semibold uppercase tracking-[0.8px]" style={{ color: 'var(--text-muted)', background: 'var(--bg-elevated)' }}>{t('users.role')}</th>
                <th className="text-left px-5 py-3 text-[11px] font-semibold uppercase tracking-[0.8px]" style={{ color: 'var(--text-muted)', background: 'var(--bg-elevated)' }}>{t('users.lastActive')}</th>
                {isAdmin && (
                  <th className="text-right px-5 py-3 text-[11px] font-semibold uppercase tracking-[0.8px]" style={{ color: 'var(--text-muted)', background: 'var(--bg-elevated)' }}>{t('common.actions')}</th>
                )}
              </tr>
            </thead>
            <tbody>
              {users.map((user) => {
                const role = roleConfig[user.role] ?? roleConfig.viewer;
                const RoleIcon = role.icon;
                return (
                  <tr key={user.id} className="transition-colors" style={{ borderBottom: '1px solid var(--border)' }} onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')} onMouseLeave={(e) => (e.currentTarget.style.background = '')}>
                    <td className="px-5 py-3">
                      <div className="flex items-center gap-3">
                        <div className="w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold text-white" style={{ background: 'linear-gradient(135deg, var(--brand), var(--accent))' }}>
                          {user.username[0].toUpperCase()}
                        </div>
                        <span className="text-[13px] font-medium" style={{ color: 'var(--text-primary)' }}>{user.username}</span>
                      </div>
                    </td>
                    <td className="px-5 py-3">
                      <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-[20px] text-[11px] font-semibold" style={{ background: `${role.color}15`, color: role.color }}>
                        <RoleIcon size={12} /> {t(role.labelKey)}
                      </span>
                    </td>
                    <td className="px-5 py-3 text-[12px]" style={{ color: 'var(--text-muted)' }}>
                      {new Date(user.created_at).toLocaleDateString()}
                    </td>
                    {isAdmin && (
                      <td className="px-5 py-3">
                        <div className="flex items-center justify-end gap-3">
                          <button
                            type="button"
                            onClick={() => openEdit(user)}
                            className="flex w-10 flex-col items-center gap-1 text-[var(--text-muted)] transition-colors hover:text-[var(--text-primary)]"
                            title="配置"
                          >
                            <Edit3 size={15} strokeWidth={1.8} />
                            <span className="text-[10px] leading-none">配置</span>
                          </button>
                          <button
                            type="button"
                            onClick={() => setDeleteTarget(user)}
                            className="flex w-10 flex-col items-center gap-1 text-[var(--text-muted)] transition-colors hover:text-[var(--red)]"
                            title="删除"
                          >
                            <Trash2 size={15} strokeWidth={1.8} />
                            <span className="text-[10px] leading-none">删除</span>
                          </button>
                        </div>
                      </td>
                    )}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {/* Create User Modal */}
      <Modal isOpen={showCreate} onClose={() => setShowCreate(false)} title={t('users.addUser')}>
        <div className="space-y-4">
          <div>
            <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('users.user')} *</label>
            <input
              value={newUsername}
              onChange={(e) => setNewUsername(e.target.value)}
              placeholder="username"
              className="w-full px-3 py-2.5 rounded-[6px] text-[13px]"
              style={{ background: 'var(--bg-elevated)', border: '1px solid var(--border)', color: 'var(--text)', outline: 'none' }}
            />
          </div>
          <div>
            <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('users.password')} *</label>
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
            <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('users.role')}</label>
            <div className="grid grid-cols-3 gap-2">
              {Object.entries(roleConfig).map(([value, config]) => {
                const Icon = config.icon;
                return (
                  <button
                    key={value}
                    onClick={() => setNewRole(value)}
                    className="flex flex-col items-center gap-1 p-3 rounded-[8px] border transition-all"
                    style={{
                      background: newRole === value ? 'var(--bg-elevated)' : 'var(--bg-surface)',
                      borderColor: newRole === value ? 'var(--brand)' : 'var(--border)',
                    }}
                  >
                    <Icon size={16} style={{ color: newRole === value ? 'var(--brand)' : 'var(--text-muted)' }} />
                    <span className="text-[11px] font-semibold" style={{ color: newRole === value ? 'var(--text)' : 'var(--text-secondary)' }}>{t(config.labelKey)}</span>
                  </button>
                );
              })}
            </div>
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <button onClick={() => setShowCreate(false)} className="px-4 py-2 text-[13px] rounded-[6px] border" style={{ color: 'var(--text-secondary)', background: 'var(--bg-card)', borderColor: 'var(--border)' }}>{t('common.cancel')}</button>
            <button
              onClick={handleCreate}
              disabled={creating || !newUsername.trim() || !newPassword.trim()}
              className="px-4 py-2 text-[13px] text-white rounded-[6px] disabled:opacity-50"
              style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))' }}
            >
              {creating ? '...' : t('users.addUser')}
            </button>
          </div>
        </div>
      </Modal>

      {/* Edit User Modal */}
      <Modal isOpen={!!editTarget} onClose={() => setEditTarget(null)} title={t('users.editUser')}>
        <div className="space-y-4">
          <div>
            <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('users.user')}</label>
            <div className="px-3 py-2.5 rounded-[6px] text-[13px]" style={{ background: 'var(--bg-elevated)', border: '1px solid var(--border)', color: 'var(--text)' }}>
              {editTarget?.username}
            </div>
          </div>
          <div>
            <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('users.role')}</label>
            <div className="grid grid-cols-3 gap-2">
              {Object.entries(roleConfig).map(([value, config]) => {
                const Icon = config.icon;
                return (
                  <button
                    key={value}
                    onClick={() => setEditRole(value)}
                    className="flex flex-col items-center gap-1 p-3 rounded-[8px] border transition-all"
                    style={{
                      background: editRole === value ? 'var(--bg-elevated)' : 'var(--bg-surface)',
                      borderColor: editRole === value ? 'var(--brand)' : 'var(--border)',
                    }}
                  >
                    <Icon size={16} style={{ color: editRole === value ? 'var(--brand)' : 'var(--text-muted)' }} />
                    <span className="text-[11px] font-semibold" style={{ color: editRole === value ? 'var(--text)' : 'var(--text-secondary)' }}>{t(config.labelKey)}</span>
                  </button>
                );
              })}
            </div>
          </div>
          <div>
            <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('users.changePassword')} <span className="text-[10px]" style={{ color: 'var(--text-muted)' }}>(leave empty to keep current)</span></label>
            <input
              type="password"
              value={editPassword}
              onChange={(e) => setEditPassword(e.target.value)}
              placeholder="••••••••"
              className="w-full px-3 py-2.5 rounded-[6px] text-[13px]"
              style={{ background: 'var(--bg-elevated)', border: '1px solid var(--border)', color: 'var(--text)', outline: 'none' }}
            />
          </div>
          {editPassword && (
            <div>
              <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>{t('users.confirmPassword')}</label>
              <input
                type="password"
                value={editConfirmPassword}
                onChange={(e) => setEditConfirmPassword(e.target.value)}
                placeholder="••••••••"
                className="w-full px-3 py-2.5 rounded-[6px] text-[13px]"
                style={{ background: 'var(--bg-elevated)', border: '1px solid var(--border)', color: 'var(--text)', outline: 'none' }}
              />
            </div>
          )}
          <div className="flex justify-end gap-2 pt-2">
            <button onClick={() => setEditTarget(null)} className="px-4 py-2 text-[13px] rounded-[6px] border" style={{ color: 'var(--text-secondary)', background: 'var(--bg-card)', borderColor: 'var(--border)' }}>{t('common.cancel')}</button>
            <button
              onClick={handleSave}
              disabled={saving}
              className="px-4 py-2 text-[13px] text-white rounded-[6px] disabled:opacity-50"
              style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))' }}
            >
              {saving ? '...' : t('common.save')}
            </button>
          </div>
        </div>
      </Modal>

      {/* Delete Confirmation */}
      <ConfirmDialog
        isOpen={!!deleteTarget}
        title={t('users.deleteTitle')}
        message={t('users.deleteMsg')}
        confirmLabel={t('common.delete')}
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
}
