import { useState, useEffect, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { agentsApi, groupsApi, parseTags, type Agent, type Group } from '@/api/client';
import { t } from '@/i18n';
import { useToast } from '@/components/Toast';
import { usePageTitle } from '@/components/PageTitleContext';
import Modal from '@/components/Modal';
import ConfirmDialog from '@/components/ConfirmDialog';

/* ---- helper functions ---- */
function formatUptime(seconds?: number): string {
  if (!seconds) return '--';
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  if (d > 0) return `${d}d ${h}h`;
  return `${h}h`;
}

function formatRate(rate?: number): string {
  if (!rate) return '--';
  if (rate >= 1000000) return `${(rate / 1000000).toFixed(1)}M`;
  if (rate >= 1000) return `${(rate / 1000).toFixed(1)}K`;
  return rate.toFixed(0);
}


/* ---- icon components ---- */
const PlusIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: 14, height: 14 }}>
    <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
  </svg>
);

/* ---- styles ---- */
const s = {
  btnPrimary: {
    display: 'inline-flex', alignItems: 'center', gap: 6, padding: '8px 16px',
    borderRadius: 6, fontSize: 12, fontWeight: 600, border: 'none', cursor: 'pointer',
    background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))',
    color: '#fff', boxShadow: '0 2px 10px rgba(108,92,231,.3)',
    transition: 'box-shadow .2s, transform .2s',
  } as React.CSSProperties,
  toolbar: { display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 16, gap: 12 } as React.CSSProperties,
  filterGroup: { display: 'flex', alignItems: 'center', gap: 8 } as React.CSSProperties,
  filterChip: (active: boolean): React.CSSProperties => ({
    padding: '6px 14px', borderRadius: 20, fontSize: 12, fontWeight: 500,
    cursor: 'pointer', border: '1px solid ' + (active ? 'rgba(108,92,231,.3)' : 'var(--border)'),
    background: active ? 'linear-gradient(135deg, rgba(108,92,231,.15), rgba(0,206,201,.08))' : 'var(--bg-card)',
    color: active ? 'var(--text-primary)' : 'var(--text-secondary)',
    transition: 'all .2s',
  }),
  groupLabel: { fontSize: 13, fontWeight: 600, color: 'var(--text-secondary)', margin: '20px 0 12px', display: 'flex', alignItems: 'center', gap: 8 } as React.CSSProperties,
  groupCount: { background: 'var(--bg-elevated)', border: '1px solid var(--border)', padding: '1px 8px', borderRadius: 10, fontSize: 10, color: 'var(--text-muted)' } as React.CSSProperties,
  agentGrid: { display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))', gap: 14 } as React.CSSProperties,
  agentCard: (offline: boolean): React.CSSProperties => ({
    background: 'var(--bg-card)', border: '1px solid var(--border)', borderRadius: 14,
    padding: 18, cursor: 'pointer', transition: 'all .2s', overflow: 'hidden',
    opacity: offline ? 0.5 : 1,
    textDecoration: 'none', color: 'inherit', display: 'block',
  }),
  cardTop: { display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 14 } as React.CSSProperties,
  agentName: { fontSize: 14, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 8 } as React.CSSProperties,
  statusDot: (status: string): React.CSSProperties => ({
    width: 8, height: 8, borderRadius: '50%', display: 'inline-block',
    background: status === 'online' ? 'var(--green)' : status === 'offline' ? 'var(--red)' : 'var(--yellow)',
    boxShadow: status === 'online' ? '0 0 8px rgba(0,214,143,.5)' : status === 'offline' ? '0 0 8px rgba(255,77,106,.4)' : '0 0 8px rgba(255,184,0,.4)',
  }),
  agentHost: { fontSize: 11, color: 'var(--text-muted)', fontFamily: "'JetBrains Mono', monospace", marginBottom: 12 } as React.CSSProperties,
  agentStats: { display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 8 } as React.CSSProperties,
  agentStat: { textAlign: 'center' as const, padding: '8px 4px', background: 'var(--bg-elevated)', borderRadius: 6 } as React.CSSProperties,
  statVal: { fontSize: 16, fontWeight: 700, color: 'var(--text-primary)' } as React.CSSProperties,
  statLbl: { fontSize: 10, color: 'var(--text-muted)', marginTop: 2 } as React.CSSProperties,
  cardTags: { display: 'flex', flexWrap: 'wrap' as const, gap: 4, marginTop: 12 } as React.CSSProperties,
  badgeTag: { background: 'var(--bg-elevated)', color: 'var(--text-secondary)', border: '1px solid var(--border)', padding: '2px 8px', borderRadius: 4, fontSize: 10, fontWeight: 500 } as React.CSSProperties,
};

export default function Agents() {
  const { setPageTitle } = usePageTitle();
  useEffect(() => {
    setPageTitle('节点管理', '管理和监控追踪节点');
    return () => setPageTitle('');
  }, []);
  
  const [agents, setAgents] = useState<Agent[]>([]);
  const [groups, setGroups] = useState<Group[]>([]);
  const [filter, setFilter] = useState<'all' | 'online' | 'offline'>('all');
  const [showCreate, setShowCreate] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Agent | null>(null);
  const [loading, setLoading] = useState(true);
  const { toast } = useToast();

  // Create form state
  const [newName, setNewName] = useState('');
  const [newHost, setNewHost] = useState('');
  const [newGroupId, setNewGroupId] = useState<number | ''>('');
  const [newTags, setNewTags] = useState('');
  const [newToken, setNewToken] = useState('');
  const [creating, setCreating] = useState(false);

  const load = () => {
    setLoading(true);
    Promise.all([agentsApi.list(), groupsApi.list()])
      .then(([a, g]) => { setAgents(a); setGroups(g); })
      .catch(() => toast({ title: t('agents.toast.loadFailed'), variant: 'error' }))
      .finally(() => setLoading(false));
  };

  useEffect(load, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Filter counts
  const onlineCount = useMemo(() => agents.filter((a) => a.status === 'online').length, [agents]);
  const offlineCount = useMemo(() => agents.filter((a) => a.status === 'offline').length, [agents]);

  // Filtered agents
  const filtered = useMemo(() => {
    if (filter === 'online') return agents.filter((a) => a.status === 'online');
    if (filter === 'offline') return agents.filter((a) => a.status === 'offline');
    return agents;
  }, [agents, filter]);

  // Group agents by group_name
  const grouped = useMemo(() => {
    const map = new Map<string, Agent[]>();
    filtered.forEach((agent) => {
      const key = agent.group_name || 'Ungrouped';
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(agent);
    });
    return map;
  }, [filtered]);

  const handleCreate = async () => {
    if (!newName.trim() || !newHost.trim()) return;
    setCreating(true);
    try {
await agentsApi.create({
        name: newName.trim(),
        host: newHost.trim(),
        group_id: newGroupId || undefined,
        tags: newTags ? newTags.split(',').map((t) => t.trim()).filter(Boolean) : undefined,
        token: newToken.trim() || undefined,
      });
      toast({ title: t('agents.toast.created'), variant: 'success' });
      setShowCreate(false);
      setNewName(''); setNewHost(''); setNewGroupId(''); setNewTags(''); setNewToken('');
      load();
    } catch (err: unknown) {
      toast({ title: t('agents.toast.createFailed'), description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally {
      setCreating(false);
    }
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    try {
      await agentsApi.remove(deleteTarget.id);
      toast({ title: t('agents.toast.deleted'), variant: 'success' });
      setDeleteTarget(null);
      load();
    } catch (err: unknown) {
      toast({ title: t('agents.toast.deleteFailed'), description: err instanceof Error ? err.message : '', variant: 'error' });
    }
  };

  if (loading) {
    return <div style={{ padding: 40, textAlign: 'center', color: 'var(--text-muted)' }}>Loading...</div>;
  }

  return (
    <div>
      {/* Toolbar with filter chips */}
      <div style={s.toolbar}>
        <div style={s.filterGroup}>
          {[
            { key: 'all' as const, label: t('agents.all') + ' ' + agents.length },
            { key: 'online' as const, label: t('agents.online') + ' ' + onlineCount },
            { key: 'offline' as const, label: t('agents.offline') + ' ' + offlineCount },
          ].map((chip) => (
            <div
              key={chip.key}
              style={s.filterChip(filter === chip.key)}
              onClick={() => setFilter(chip.key)}
            >
              {chip.label}
            </div>
          ))}
        </div>
        <button style={s.btnPrimary} onClick={() => setShowCreate(true)}>
          <PlusIcon /> {t('agents.addAgent')}
        </button>
      </div>

      {/* Grouped agent cards */}
      {Array.from(grouped.entries()).map(([groupName, groupAgents]) => (
        <div key={groupName}>
          <div style={s.groupLabel}>
            {groupName} <span style={s.groupCount}>{groupAgents.length}</span>
          </div>
          <div style={s.agentGrid}>
            {groupAgents.map((agent) => {
              const isOffline = agent.status === 'offline';
              const tags = parseTags(agent.tags);
              return (
                <Link
                  key={agent.id}
                  to={`/agents/${agent.id}`}
                  style={s.agentCard(isOffline)}
                  onMouseEnter={(e) => {
                    const el = e.currentTarget;
                    el.style.borderColor = 'var(--brand)';
                    el.style.boxShadow = '0 0 20px rgba(108,92,231,.1)';
                    el.style.transform = 'translateY(-2px)';
                  }}
                  onMouseLeave={(e) => {
                    const el = e.currentTarget;
                    el.style.borderColor = 'var(--border)';
                    el.style.boxShadow = 'none';
                    el.style.transform = 'translateY(0)';
                  }}
                >
                  {/* Line 1: Agent Name + Status */}
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 6 }}>
                    <div style={s.agentName}>
                      <span style={s.statusDot(agent.status)} />
                      {agent.name}
                    </div>
                    <span style={{ 
                      display: 'flex', alignItems: 'center', gap: 4, 
                      fontSize: 10, fontWeight: 600,
                      color: agent.status === 'online' ? 'var(--green)' : 'var(--red)',
                      background: agent.status === 'online' ? 'var(--green-dim)' : 'var(--red-dim)',
                      padding: '2px 8px', borderRadius: 10
                    }}>
                      <span style={{ width: 5, height: 5, borderRadius: '50%', background: 'currentColor' }} />
                      {agent.status === 'online' ? t('agents.connected') : t('agents.disconnected')}
                    </span>
                  </div>

                  {/* Line 2: IP + Version */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
                    <span style={{ fontSize: 11, color: 'var(--text-muted)', fontFamily: "'JetBrains Mono', monospace" }}>
                      {agent.host}
                    </span>
                    <span style={{ fontSize: 10, color: 'var(--text-muted)' }}>
                      v{agent.version || '0.0.1'}
                    </span>
                  </div>

                  {/* Line 3: Tracers + Events + Uptime (main body) */}
                  <div style={s.agentStats}>
                    <div style={s.agentStat}>
                      <div style={s.statVal}>{agent.tracers?.length ?? 0}</div>
                      <div style={s.statLbl}>Tracers</div>
                    </div>
                    <div style={s.agentStat}>
                      <div style={s.statVal}>{formatRate(agent.rate)}</div>
                      <div style={s.statLbl}>Events/s</div>
                    </div>
                    <div style={s.agentStat}>
                      <div style={s.statVal}>{formatUptime(agent.uptime)}</div>
                      <div style={s.statLbl}>Uptime</div>
                    </div>
                  </div>

                  {/* Line 4: Tags + Group */}
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: 10, paddingTop: 10, borderTop: '1px solid var(--border)' }}>
                    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
                      {tags.map((tag) => (
                        <span key={tag} style={s.badgeTag}>{tag}</span>
                      ))}
                    </div>
                    {agent.group_name && (
                      <span style={{ fontSize: 10, color: 'var(--brand-light)', background: 'rgba(108,92,231,.1)', padding: '2px 8px', borderRadius: 4, flexShrink: 0 }}>
                        {agent.group_name}
                      </span>
                    )}
                  </div>
                </Link>
              );
            })}
          </div>
        </div>
      ))}

      {filtered.length === 0 && (
        <div style={{ padding: 40, textAlign: 'center', color: 'var(--text-muted)', fontSize: 13 }}>
          {t('agents.noAgents')}
        </div>
      )}

      {/* Create Modal */}
      <Modal isOpen={showCreate} onClose={() => setShowCreate(false)} title={t('agents.addNewAgent')}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div>
            <label style={{ display: 'block', fontSize: 12, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 6 }}>{t('agents.agentName')}</label>
            <input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder={t('agents.agentNamePlaceholder')}
              style={{ width: '100%', padding: '9px 12px', background: 'var(--bg-card)', border: '1px solid var(--border)', borderRadius: 6, color: 'var(--text-primary)', fontSize: 13, outline: 'none' }}
            />
          </div>
          <div>
            <label style={{ display: 'block', fontSize: 12, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 6 }}>{t('agents.hostAddress')}</label>
            <input
              value={newHost}
              onChange={(e) => setNewHost(e.target.value)}
              placeholder={t('agents.hostPlaceholder')}
              style={{ width: '100%', padding: '9px 12px', background: 'var(--bg-card)', border: '1px solid var(--border)', borderRadius: 6, color: 'var(--text-primary)', fontSize: 13, outline: 'none' }}
            />
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <div>
              <label style={{ display: 'block', fontSize: 12, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 6 }}>{t('agents.group')}</label>
              <select
                value={newGroupId}
                onChange={(e) => setNewGroupId(e.target.value ? Number(e.target.value) : '')}
                style={{ width: '100%', padding: '9px 12px', background: 'var(--bg-card)', border: '1px solid var(--border)', borderRadius: 6, color: 'var(--text-primary)', fontSize: 13, outline: 'none' }}
              >
                <option value="">{t('agents.noGroup')}</option>
                {groups.map((g) => <option key={g.id} value={g.id}>{g.name}</option>)}
              </select>
            </div>
            <div>
              <label style={{ display: 'block', fontSize: 12, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 6 }}>{t('agents.tags')}</label>
              <input
                value={newTags}
                onChange={(e) => setNewTags(e.target.value)}
                placeholder={t('agents.tagsPlaceholder')}
                style={{ width: '100%', padding: '9px 12px', background: 'var(--bg-card)', border: '1px solid var(--border)', borderRadius: 6, color: 'var(--text-primary)', fontSize: 13, outline: 'none' }}
              />
            </div>
          </div>
          <div>
            <label style={{ display: 'block', fontSize: 12, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 6 }}>{t('agents.apiToken')}</label>
            <input
              value={newToken}
              onChange={(e) => setNewToken(e.target.value)}
              placeholder={t('agents.apiTokenPlaceholder')}
              style={{ width: '100%', padding: '9px 12px', background: 'var(--bg-card)', border: '1px solid var(--border)', borderRadius: 6, color: 'var(--text-primary)', fontSize: 13, outline: 'none' }}
            />
          </div>
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, paddingTop: 8 }}>
            <button
              onClick={() => setShowCreate(false)}
              style={{ padding: '8px 16px', borderRadius: 6, fontSize: 12, fontWeight: 600, border: '1px solid var(--border)', background: 'var(--bg-card)', color: 'var(--text-secondary)', cursor: 'pointer' }}
            >
              {t('common.cancel')}
            </button>
            <button
              onClick={handleCreate}
              disabled={creating || !newName.trim() || !newHost.trim()}
              style={{ ...s.btnPrimary, opacity: creating || !newName.trim() || !newHost.trim() ? 0.5 : 1 }}
            >
              {creating ? t('agents.creating') : t('agents.create')}
            </button>
          </div>
        </div>
      </Modal>

      {/* Delete Confirmation */}
      <ConfirmDialog
        isOpen={!!deleteTarget}
        title="Delete Agent"
        message={`Are you sure you want to delete "${deleteTarget?.name}"? This action cannot be undone.`}
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
}
