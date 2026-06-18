import { useState, useEffect } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import { ArrowLeft, Pause, Play, Settings, Terminal } from 'lucide-react';
import { agentsApi, sinksApi, type Agent, type Tracepoint, type Sink } from '@/api/client';
import { t } from '@/i18n';
import { useToast } from '@/components/Toast';
import { usePageTitle } from '@/components/PageTitleContext';
import Modal from '@/components/Modal';
import ConfirmDialog from '@/components/ConfirmDialog';
import SearchableSelect from '@/components/SearchableSelect';

function formatUptime(seconds?: number): string {
  if (!seconds) return '--';
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatRate(rate?: number): string {
  if (!rate) return '--';
  if (rate >= 1000000) return `${(rate / 1000000).toFixed(1)}M`;
  if (rate >= 1000) return `${(rate / 1000).toFixed(1)}K`;
  return rate.toFixed(0);
}



export default function AgentDetail() {
  
  const { id } = useParams();
  const navigate = useNavigate();
  const agentId = Number(id);
  const { setPageTitle } = usePageTitle();
  const [agent, setAgent] = useState<Agent | null>(null);
  const [tracepoints, setTracepoints] = useState<Tracepoint[]>([]);
  const [sinks, setSinks] = useState<Sink[]>([]);
  const [allSinks, setAllSinks] = useState<Sink[]>([]);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const { toast } = useToast();

  // Create tracepoint modal
  const [showAddTP, setShowAddTP] = useState(false);
  const [tpName, setTpName] = useState('');
  const [tpDesc, setTpDesc] = useState('');
  const [creatingTP, setCreatingTP] = useState(false);

  // Tracer action loading
  const [tracerAction, setTracerAction] = useState<'starting' | 'stopping' | null>(null);
  const [tracerActionName, setTracerActionName] = useState('');

  // Delete agent
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  // Edit tracer modal
  const [showEditTP, setShowEditTP] = useState(false);
  const [editingTP, setEditingTP] = useState<Tracepoint | null>(null);
  const [editTPForm, setEditTPForm] = useState({
    name: '',
    description: '',
    schedule: 'always',
    sink_ids: [] as number[],
    metrics_config: '{}',
  });
  const [savingTP, setSavingTP] = useState(false);

  // Bind sink
  const [showBindSink, setShowBindSink] = useState(false);
  const [bindSinkId, setBindSinkId] = useState<number | ''>('');

  useEffect(() => {
    setPageTitle('节点管理', '管理和监控追踪节点');
    return () => setPageTitle('');
  }, [setPageTitle]);

  const load = async () => {
    try {
      const [a, tp, allS] = await Promise.all([
        agentsApi.get(agentId),
        agentsApi.listTracepoints(agentId),
        sinksApi.list(),
      ]);
      setAgent(a);
      setTracepoints(tp.items ?? []);
      setAllSinks(allS);

      // Find sinks bound to this agent
      const boundSinks: Sink[] = [];
      for (const s of allS) {
        try {
          const agentIds = await sinksApi.getAgents(s.id);
          if (agentIds.includes(agentId)) boundSinks.push(s);
        } catch { /* ignore */ }
      }
      setSinks(boundSinks);
    } catch (err: unknown) {
      toast({ title: 'Failed to load agent', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, [agentId]); // eslint-disable-line react-hooks/exhaustive-deps

  const refreshAgentSnapshot = async () => {
    try {
      const [freshAgent, tp] = await Promise.all([
        agentsApi.get(agentId),
        agentsApi.listTracepoints(agentId),
      ]);
      setAgent(freshAgent);
      setTracepoints(tp.items ?? []);
    } catch { /* keep current view if refresh fails */ }
  };

  const handleSync = async () => {
    setSyncing(true);
    try {
      const updated = await agentsApi.sync(agentId);
      setAgent(updated);
      // Reload tracepoints too
      const tp = await agentsApi.listTracepoints(agentId);
      setTracepoints(tp.items ?? []);
      toast({ title: t('agentDetail.syncSuccess'), variant: 'success' });
    } catch {
      await refreshAgentSnapshot();
      toast({ title: t('agentDetail.syncFailed'), variant: 'error' });
    } finally {
      setSyncing(false);
    }
  };

  const toggleTP = async (tp: Tracepoint) => {
    const action = tp.enabled ? 'stopping' : 'starting';
    setTracerAction(action);
    setTracerActionName(tp.name);
    try {
      const promise = tp.enabled
        ? agentsApi.stopTracer(agentId, tp.name)
        : agentsApi.startTracer(agentId, tp.name);
      // Wait for API + at least 2s
      const [,] = await Promise.all([promise, new Promise(r => setTimeout(r, 2000))]);
      setTracepoints((prev) => prev.map((t) => t.id === tp.id ? { ...t, enabled: !t.enabled } : t));
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Toggle failed';
      toast({ title: msg, variant: 'error' });
    } finally {
      setTracerAction(null);
    }
  };

  const handleAddTP = async () => {
    if (!tpName.trim()) return;
    setCreatingTP(true);
    try {
      const tp = await agentsApi.createTracepoint(agentId, { name: tpName.trim(), description: tpDesc.trim() || undefined });
      setTracepoints((prev) => [...prev, tp]);
      setShowAddTP(false);
      setTpName(''); setTpDesc('');
      toast({ title: 'Tracepoint added', variant: 'success' });
    } catch {
      toast({ title: 'Add failed', variant: 'error' });
    } finally {
      setCreatingTP(false);
    }
  };

  const handleDeleteAgent = async () => {
    try {
      await agentsApi.remove(agentId);
      toast({ title: 'Agent deleted', variant: 'success' });
      navigate('/agents');
    } catch {
      toast({ title: 'Delete failed', variant: 'error' });
    }
  };

  const handleBindSink = async () => {
    if (!bindSinkId) return;
    try {
      await sinksApi.bindAgent(Number(bindSinkId), agentId);
      const sink = allSinks.find((s) => s.id === Number(bindSinkId));
      if (sink) setSinks((prev) => [...prev, sink]);
      setShowBindSink(false);
      setBindSinkId('');
      toast({ title: 'Sink bound', variant: 'success' });
    } catch {
      toast({ title: 'Bind failed', variant: 'error' });
    }
  };

  const openEditTPModal = (tp: Tracepoint) => {
    setEditingTP(tp);
    setEditTPForm({
      name: tp.name,
      description: tp.description || '',
      schedule: (tp as any).schedule || 'always',
      sink_ids: tp.sink_id ? [tp.sink_id] : [],
      metrics_config: (tp as any).metrics_config || '{}',
    });
    setShowEditTP(true);
  };

  const handleSaveTP = async () => {
    if (!editingTP) return;
    setSavingTP(true);
    try {
      await agentsApi.updateTracepoint(agentId, editingTP.id, {
        enabled: editingTP.enabled,
        sink_id: editTPForm.sink_ids[0] ?? null,
      });
      toast({ title: 'Tracer updated', variant: 'success' });
      setShowEditTP(false);
      load(); // Reload data
    } catch (err: unknown) {
      toast({ title: 'Update failed', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally {
      setSavingTP(false);
    }
  };

  if (loading) {
    return (
      <div className="space-y-6">
        <div className="h-10 w-64 bg-[var(--bg-card)] rounded-[14px] animate-pulse" />
        <div className="h-32 bg-[var(--bg-card)] rounded-[14px] animate-pulse" />
      </div>
    );
  }

  if (!agent) {
    return (
      <div className="text-center py-20">
        <p className="text-[var(--text-muted)]">Agent not found</p>
        <Link to="/agents" className="text-[var(--accent)] text-sm mt-2 inline-block hover:underline">Back to agents</Link>
      </div>
    );
  }

  const agentOnline = agent.status === 'online';
  const enabledCount = agentOnline ? tracepoints.filter((t) => t.enabled).length : 0;
  const tags: string[] = (() => {
    if (!agent.tags) return [];
    if (Array.isArray(agent.tags)) return agent.tags.filter(Boolean);
    return String(agent.tags).split(',').filter(Boolean);
  })();

  return (
    <div className="space-y-6">
      {/* ── Header ── */}
      <div className="flex items-center gap-4">
        <button
          onClick={() => navigate('/agents')}
          className="w-9 h-9 rounded-[6px] bg-[var(--bg-card)] border border-[var(--border)] flex items-center justify-center text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)] transition-colors"
        >
          <ArrowLeft size={16} />
        </button>
        <div className="flex-1 min-w-0">
          <h1 className="text-[20px] font-bold text-[var(--text-primary)] flex items-center gap-2.5">
            <span className={`w-2.5 h-2.5 rounded-full ${agent.status === 'online' ? 'bg-[var(--green)] shadow-[0_0_8px_rgba(0,214,143,0.5)]' : 'bg-[var(--red)] shadow-[0_0_8px_rgba(255,77,106,0.4)]'}`} />
            {agent.name}
          </h1>
          <p className="text-[var(--text-secondary)] text-[13px] mt-0.5">
            {agent.host} / v{agent.version || '0.0.1'}
          </p>
        </div>

        <button
          onClick={() => navigate(`/agents/${agentId}/edit`)}
          className="px-3 py-1.5 text-[11px] font-semibold rounded-[6px] transition-colors"
          style={{
            background: 'var(--green-dim)',
            color: 'var(--green)',
            border: '1px solid rgba(0,214,143,0.3)',
          }}
        >
          ✎ 配置
        </button>
        <button
          onClick={handleSync}
          disabled={syncing}
          className="px-3 py-1.5 text-[11px] font-semibold rounded-[6px] transition-colors"
          style={{
            background: 'var(--brand-dim, rgba(108,92,231,0.1))',
            color: 'var(--brand)',
            border: '1px solid rgba(108,92,231,0.3)',
          }}
        >
          {syncing ? '⟳ Syncing...' : '⟳ Sync'}
        </button>
        <button
          onClick={() => setShowDeleteConfirm(true)}
          className="px-3 py-1.5 text-[11px] font-semibold rounded-[6px] transition-colors"
          style={{
            background: 'var(--red-dim)',
            color: 'var(--red)',
            border: '1px solid rgba(255,77,106,0.2)',
          }}
        >
          {t('agentDetail.delete')}
        </button>
      </div>

      {/* ── Meta Row ── */}
      <div className="flex items-center gap-5 flex-wrap text-[12px] text-[var(--text-secondary)]">
        <span>{t('agentDetail.uptime')}: <strong className="text-[var(--text-primary)]">{formatUptime(agent.uptime)}</strong></span>
        <span>{t('agentDetail.events')}: <strong className="text-[var(--green)]" style={{ fontFamily: "'JetBrains Mono', monospace" }}>{formatRate(agent.rate)}/s</strong></span>
        <span>{t('agentDetail.kernel')}: <strong className="text-[var(--text-primary)]">{agent.os && agent.arch ? `${agent.os}/${agent.arch}` : agent.host}</strong></span>
        {agent.group_name && (
          <span>{t('agentDetail.group')}: <strong className="text-[var(--text-primary)]">{agent.group_name}</strong></span>
        )}
        <div className="flex gap-1">
          {tags.map((tag) => (
            <span key={tag} className="inline-flex items-center gap-1 px-2 py-0.5 bg-[var(--bg-elevated)] text-[var(--text-secondary)] text-[10px] font-medium rounded-[4px] border border-[var(--border)]">
              {typeof tag === 'string' ? tag.trim() : tag}
            </span>
          ))}
        </div>
      </div>

      {/* ── Stats Grid ── */}
      <div className="grid grid-cols-4 gap-4">
        {[
          { label: t('agentDetail.tracers'), value: tracepoints.length.toString(), color: '#6C5CE7', gradient: 'linear-gradient(90deg, #6C5CE7, #a29bfe)' },
          { label: t('agentDetail.active'), value: enabledCount.toString(), color: '#00D68F', gradient: 'linear-gradient(90deg, #00D68F, #34d399)' },
          { label: t('agentDetail.connectedSinks'), value: sinks.length.toString(), color: '#3B82F6', gradient: 'linear-gradient(90deg, #3B82F6, #60a5fa)' },
          { label: t('agentDetail.totalEvents'), value: formatRate((agent.rate ?? 0) * (agent.uptime ?? 0)), color: '#FFB800', gradient: 'linear-gradient(90deg, #FFB800, #fcd34d)' },
        ].map((card) => (
          <div
            key={card.label}
            className="relative bg-[var(--bg-card)] border border-[var(--border)] rounded-[14px] p-4 overflow-hidden"
          >
            <div className="absolute top-0 left-0 right-0 h-[2px]" style={{ background: card.gradient }} />
            <div className="text-[22px] font-bold text-[var(--text-primary)] leading-none tracking-tight">{card.value}</div>
            <div className="text-[12px] text-[var(--text-secondary)] font-medium mt-1">{card.label}</div>
          </div>
        ))}
      </div>

      {/* ── Tracers ── */}
      <div className="space-y-4">
        <div className="bg-[var(--bg-card)] border border-[var(--border)] rounded-[14px] overflow-hidden">
          <div className="flex items-center justify-between px-5 py-4 border-b border-[var(--border)]">
            <h3 className="text-[14px] font-semibold text-[var(--text-primary)]">
              {t('agentDetail.tracers')}{' '}
              <span className="font-normal text-[var(--text-muted)] text-[12px]">
                ({tracepoints.length} {t('agentDetail.configured')}, {enabledCount} {t('agentDetail.active')})
              </span>
            </h3>
          </div>
          <div className="p-5">
            {tracepoints.length === 0 ? (
              <div className="text-center text-[var(--text-muted)] text-sm py-8">
                {t('agentDetail.noTracers')}
              </div>
            ) : (
              <div className="space-y-3">
                {tracepoints.map((tp) => {
                  const isRunning = agentOnline && tp.enabled;
                  const tracerStatus = !agentOnline ? 'Unknown' : isRunning ? 'Running' : 'Stopped';
                  return (
                    <div
                      key={tp.id}
                      className="flex flex-col overflow-hidden bg-[var(--bg-elevated)] border border-[var(--border)] rounded-[10px] hover:border-[var(--brand)] transition-colors md:flex-row"
                    >
                      <div className="min-w-0 flex-1 p-4">
                        <div className="flex items-center gap-2.5 mb-2">
                            <span className={`w-2.5 h-2.5 rounded-full flex-shrink-0 ${
                              !agentOnline
                                ? 'bg-[var(--text-muted)]'
                                : isRunning
                                ? 'bg-[var(--green)] shadow-[0_0_8px_rgba(0,214,143,0.4)]'
                                : 'bg-[var(--red)] shadow-[0_0_8px_rgba(255,77,106,0.4)]'
                            }`} />
                            <span className="text-[13px] font-semibold text-[var(--text-primary)]">{tp.name}</span>
                            <span className={`inline-flex items-center px-2 py-0.5 rounded-[20px] text-[10px] font-semibold ${
                              !agentOnline
                                ? 'bg-[var(--bg-hover)] text-[var(--text-muted)]'
                                : isRunning
                                ? 'bg-[var(--green-dim)] text-[var(--green)]'
                                : 'bg-[var(--red-dim)] text-[var(--red)]'
                            }`}>
                              {tracerStatus}
                            </span>
                          </div>

                        <div className="text-[11px] text-[var(--text-muted)] mb-3 pl-[18px]">
                          {tp.description || 'No description'}
                        </div>

                        <div className="flex items-center gap-4 pl-[18px]">
                          <div className="flex items-center gap-1.5">
                            <span className="text-[10px] text-[var(--text-muted)]">Sent:</span>
                            <span className="text-[12px] font-semibold text-[var(--green)]" style={{ fontFamily: "'JetBrains Mono', monospace" }}>
                              {(tp.events_sent ?? 0).toLocaleString()}
                            </span>
                          </div>
                          <div className="flex items-center gap-1.5">
                            <span className="text-[10px] text-[var(--text-muted)]">Failed:</span>
                            <span className={`text-[12px] font-semibold ${(tp.events_failed ?? 0) > 0 ? 'text-[var(--red)]' : 'text-[var(--text-muted)]'}`} style={{ fontFamily: "'JetBrains Mono', monospace" }}>
                              {(tp.events_failed ?? 0).toLocaleString()}
                            </span>
                          </div>
                        </div>
                      </div>

                      <div className="flex shrink-0 items-center justify-end gap-2 border-t border-[var(--border)] bg-[var(--bg-card)] px-2.5 py-2 md:w-[152px] md:border-l md:border-t-0">
                        <button
                          type="button"
                          disabled={!agentOnline}
                          onClick={(e) => { e.stopPropagation(); openEditTPModal(tp); }}
                          className="flex w-10 flex-col items-center gap-1 text-[var(--text-muted)] transition-colors hover:text-[var(--text-primary)] disabled:cursor-not-allowed disabled:opacity-35 disabled:hover:text-[var(--text-muted)]"
                          title="配置"
                        >
                          <Settings size={15} strokeWidth={1.8} />
                          <span className="text-[10px] leading-none">配置</span>
                        </button>
                        <button
                          type="button"
                          disabled={!agentOnline}
                          onClick={(e) => { e.stopPropagation(); toggleTP(tp); }}
                          className={`flex w-10 flex-col items-center gap-1 transition-colors ${
                            !agentOnline
                              ? 'cursor-not-allowed text-[var(--text-muted)] opacity-35'
                              : isRunning ? 'text-[var(--red)]' : 'text-[var(--green)]'
                          }`}
                          title={!agentOnline ? 'Unknown' : isRunning ? '暂停' : '启动'}
                        >
                          {isRunning ? <Pause size={15} strokeWidth={1.8} /> : <Play size={15} strokeWidth={1.8} />}
                          <span className="text-[10px] leading-none">{isRunning ? '暂停' : '启动'}</span>
                        </button>
                        <button
                          type="button"
                          disabled={!agentOnline}
                          onClick={(e) => { e.stopPropagation(); navigate('/agents/' + agentId + '/debug?tracer=' + encodeURIComponent(tp.name) + '&desc=' + encodeURIComponent(tp.description || '')); }}
                          className="flex w-10 flex-col items-center gap-1 text-[var(--text-muted)] transition-colors hover:text-[var(--brand)] disabled:cursor-not-allowed disabled:opacity-35 disabled:hover:text-[var(--text-muted)]"
                          title={t('agentDetail.debug')}
                        >
                          <Terminal size={15} strokeWidth={1.8} />
                          <span className="text-[10px] leading-none">debug</span>
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* ── Tracer Action Loading Modal ── */}
      {tracerAction && (
        <Modal isOpen={true} onClose={() => {}} title={tracerActionName} maxWidth="max-w-xs">
          <div className="flex flex-col items-center gap-4 py-4">
            <div className="w-8 h-8 border-2 border-[var(--brand)] border-t-transparent rounded-full animate-spin" />
            <p className="text-[13px] text-[var(--text-primary)] font-medium">
              {tracerAction === 'starting' ? '正在加载...' : '正在关闭...'}
            </p>
          </div>
        </Modal>
      )}

      {/* ── Edit Tracer Modal ── */}
      <Modal isOpen={showEditTP} onClose={() => setShowEditTP(false)} title={`${t('tracerEdit.title')}: ${editingTP?.name || ''}`} maxWidth="max-w-2xl">
        <div className="space-y-5">
          {/* Schedule Configuration */}
          <div>
            <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('tracerEdit.schedule')}</label>
            <div className="flex gap-2 mb-3">
              {['always', 'cron'].map((opt) => (
                <button
                  key={opt}
                  onClick={() => setEditTPForm({ ...editTPForm, schedule: opt })}
                  className={`px-3 py-1.5 text-[11px] font-semibold rounded-[6px] transition-colors ${
                    editTPForm.schedule === opt
                      ? 'bg-[var(--brand)] text-white'
                      : 'bg-[var(--bg-card)] text-[var(--text-secondary)] border border-[var(--border)]'
                  }`}
                >
                  {opt === 'always' ? t('tracerEdit.always') : t('tracerEdit.cron')}
                </button>
              ))}
            </div>
            {editTPForm.schedule === 'always' && (
              <div className="flex items-center gap-2">
                <label className="text-[12px] text-[var(--text-secondary)]">{t('tracerEdit.duration')}：</label>
                <input
                  type="number"
                  value={(editTPForm as any).duration || 0}
                  onChange={(e) => setEditTPForm({ ...editTPForm, duration: Number(e.target.value) } as any)}
                  min={0}
                  className="w-32 px-3 py-2 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] font-mono text-[var(--text-primary)] focus:outline-none focus:border-[var(--brand)] transition-colors"
                />
                <span className="text-[11px] text-[var(--text-muted)]">{t('tracerEdit.durationHint')}</span>
              </div>
            )}
            {editTPForm.schedule === 'cron' && (
              <input
                value={editTPForm.schedule}
                onChange={(e) => setEditTPForm({ ...editTPForm, schedule: e.target.value })}
                placeholder="*/5 * * * *"
                className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] font-mono text-[var(--text-primary)] focus:outline-none focus:border-[var(--brand)] transition-colors"
              />
            )}
          </div>

          {/* Sink Configuration - Searchable Dropdown */}
          <div>
            <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('tracerEdit.output')}</label>
            <SearchableSelect
              options={[
                { value: 0, label: t('tracerEdit.exposeMetrics'), sublabel: t('tracerEdit.default') },
                ...allSinks.map((sink) => ({
                  value: sink.id,
                  label: sink.name,
                  sublabel: sink.sink_type,
                })),
              ]}
              value={editTPForm.sink_ids.length > 0 ? editTPForm.sink_ids[0] : 0}
              onChange={(val) => setEditTPForm({ ...editTPForm, sink_ids: val > 0 ? [val] : [] })}
              placeholder={t('tracerEdit.exposeMetrics')}
              searchPlaceholder="Search sinks..."
            />
          </div>

          {/* Metrics Configuration */}
          <div>
            <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('tracerEdit.tracerConfig')}</label>
            <textarea
              value={editTPForm.metrics_config}
              onChange={(e) => setEditTPForm({ ...editTPForm, metrics_config: e.target.value })}
              rows={6}
              className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[12px] font-mono text-[var(--text-primary)] focus:outline-none focus:border-[var(--brand)] transition-colors resize-none"
              placeholder='{"endpoint": "/metrics", "port": 9090, "interval": "15s"}'
            />
            <p className="text-[10px] text-[var(--text-muted)] mt-1">
              {t('tracerEdit.configHint')}
            </p>
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-2 pt-2 border-t border-[var(--border)]">
            <button
              onClick={() => setShowEditTP(false)}
              className="px-4 py-2 text-[13px] text-[var(--text-secondary)] bg-[var(--bg-card)] border border-[var(--border)] hover:bg-[var(--bg-hover)] rounded-[6px] transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleSaveTP}
              disabled={savingTP}
              className="px-4 py-2 text-[13px] text-white rounded-[6px] transition-colors disabled:opacity-50"
              style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))', boxShadow: '0 2px 10px rgba(108,92,231,0.3)' }}
            >
              {savingTP ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        </div>
      </Modal>

      {/* ── Modals ── */}
      <Modal isOpen={showAddTP} onClose={() => setShowAddTP(false)} title="Add Tracepoint">
        <div className="space-y-4">
          <div>
            <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">Name</label>
            <input
              value={tpName}
              onChange={(e) => setTpName(e.target.value)}
              placeholder="tcp_connect"
              className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] font-mono placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all"
            />
          </div>
          <div>
            <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">Description</label>
            <input
              value={tpDesc}
              onChange={(e) => setTpDesc(e.target.value)}
              placeholder="kprobe / kernel"
              className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all"
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <button onClick={() => setShowAddTP(false)} className="px-4 py-2 text-[13px] text-[var(--text-secondary)] bg-[var(--bg-card)] border border-[var(--border)] hover:bg-[var(--bg-hover)] rounded-[6px] transition-colors">{t('tracerEdit.cancel')}</button>
            <button
              onClick={handleAddTP}
              disabled={creatingTP || !tpName.trim()}
              className="px-4 py-2 text-[13px] text-white rounded-[6px] transition-colors disabled:opacity-50"
              style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))', boxShadow: '0 2px 10px rgba(108,92,231,0.3)' }}
            >
              {creatingTP ? 'Adding...' : 'Add Tracepoint'}
            </button>
          </div>
        </div>
      </Modal>

      <Modal isOpen={showBindSink} onClose={() => setShowBindSink(false)} title="Assign Sink" maxWidth="max-w-sm">
        <div className="space-y-4">
          <div>
            <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">Select Sink</label>
            <select
              value={bindSinkId}
              onChange={(e) => setBindSinkId(e.target.value ? Number(e.target.value) : '')}
              className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all"
            >
              <option value="">Choose a sink...</option>
              {allSinks.filter((s) => !sinks.find((b) => b.id === s.id)).map((s) => (
                <option key={s.id} value={s.id}>{s.name} ({s.sink_type})</option>
              ))}
            </select>
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <button onClick={() => setShowBindSink(false)} className="px-4 py-2 text-[13px] text-[var(--text-secondary)] bg-[var(--bg-card)] border border-[var(--border)] hover:bg-[var(--bg-hover)] rounded-[6px] transition-colors">{t('tracerEdit.cancel')}</button>
            <button
              onClick={handleBindSink}
              disabled={!bindSinkId}
              className="px-4 py-2 text-[13px] text-white rounded-[6px] transition-colors disabled:opacity-50"
              style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))', boxShadow: '0 2px 10px rgba(108,92,231,0.3)' }}
            >
              Assign
            </button>
          </div>
        </div>
      </Modal>

      <ConfirmDialog
        isOpen={showDeleteConfirm}
        title={t('agentDetail.deleteTitle')}
        message={t('agentDetail.deleteMsg')}
        confirmLabel={t('agentDetail.delete')}
        onConfirm={handleDeleteAgent}
        onCancel={() => setShowDeleteConfirm(false)}
      />
    </div>
  );
}
