import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Zap, ZapOff, Trash2, Plus } from 'lucide-react';
import { agentsApi, sinksApi, type Agent, type Sink } from '@/api/client';
import { t } from '@/i18n';
import { useToast } from '@/components/Toast';
import { usePageTitle } from '@/components/PageTitleContext';
import ConfirmDialog from '@/components/ConfirmDialog';
import SearchableSelect from '@/components/SearchableSelect';
import Modal from '@/components/Modal';

export default function AgentEdit() {
  const { id } = useParams();
  const navigate = useNavigate();
  const agentId = Number(id);
  const [agent, setAgent] = useState<Agent | null>(null);
  const [loading, setLoading] = useState(true);
  const { toast } = useToast();
  const { setPageTitle } = usePageTitle();

  // Token edit
  const [editToken, setEditToken] = useState('');
  const [savingToken, setSavingToken] = useState(false);
  const [editingToken, setEditingToken] = useState(false);
  const [syncing, setSyncing] = useState(false);

  function maskToken(token: string): string {
    if (!token || token.length <= 8) return token || '';
    return token.slice(0, 4) + '*'.repeat(token.length - 8) + token.slice(-4);
  }

  // Sink management
  const [boundSinks, setBoundSinks] = useState<Sink[]>([]);
  const [allSinks, setAllSinks] = useState<Sink[]>([]);
  const [connectedSinks, setConnectedSinks] = useState<Set<string>>(new Set());
  const [connectingSink, setConnectingSink] = useState<string | null>(null);
  const [showBindSink, setShowBindSink] = useState(false);
  const [bindSinkId, setBindSinkId] = useState('');
  const [bindingSink, setBindingSink] = useState(false);

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<Sink | null>(null);
  // deleting removed - not needed for ConfirmDialog

  useEffect(() => {
    setPageTitle('节点配置', '配置 Token 和数据接收器');
    return () => setPageTitle('');
  }, [setPageTitle]);

  useEffect(() => { load(); }, [agentId]);

  async function load() {
    try {
      setLoading(true);
      const [a, allS] = await Promise.all([agentsApi.get(agentId), sinksApi.list()]);
      setAgent(a);
      setEditToken(a.token || '');
      setAllSinks(allS);
      const bound: Sink[] = [];
      for (const s of allS) {
        try {
          const agentIds = await sinksApi.getAgents(s.id);
          if (agentIds.includes(agentId)) bound.push(s);
        } catch { /* ignore */ }
      }
      setBoundSinks(bound);
      try {
        const running = await agentsApi.getSinkNames(agentId);
        setConnectedSinks(new Set(running));
      } catch { setConnectedSinks(new Set()); }
    } catch (err: unknown) {
      toast({ title: 'Failed to load', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally { setLoading(false); }
  }

  async function handleSaveToken() {
    if (!editToken.trim()) return;
    setSavingToken(true);
    try {
      const updated = await agentsApi.update(agentId, { token: editToken.trim() });
      setAgent(updated);
      setEditToken(updated.token || editToken.trim());
      setEditingToken(false);
      toast({ title: '保存成功', variant: 'success' });
    } catch (err: unknown) {
      toast({ title: 'Failed to save token', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally { setSavingToken(false); }
  }

  // Manual sync
  async function handleSync() {
    setSyncing(true);
    try {
      await agentsApi.sync(agentId);
      toast({ title: '同步成功', variant: 'success' });
      load();
    } catch (err: unknown) {
      toast({ title: '同步失败', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally { setSyncing(false); }
  }

  async function handleBindSink() {
    if (!bindSinkId) return;
    setBindingSink(true);
    try {
      await sinksApi.bindAgent(Number(bindSinkId), agentId);
      const sink = allSinks.find((s) => s.id === Number(bindSinkId));
      if (sink) setBoundSinks((prev) => [...prev, sink]);
      setShowBindSink(false);
      setBindSinkId('');
      toast({ title: 'Sink bound', variant: 'success' });
    } catch (err: unknown) {
      toast({ title: 'Failed to bind sink', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally { setBindingSink(false); }
  }

  async function handleConnect(sink: Sink) {
    if (agent?.status !== 'online') {
      toast({ title: 'Agent 处于离线状态，请重新同步 Agent', variant: 'error' });
      return;
    }
    setConnectingSink(sink.name);
    try {
      await sinksApi.connectAgent(sink.id, agentId);
      setConnectedSinks((prev) => new Set([...prev, sink.name]));
      toast({ title: 'Sink connected', variant: 'success' });
    } catch (err: unknown) {
      toast({ title: 'Failed to connect', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally { setConnectingSink(null); }
  }

  async function handleDisconnect(sink: Sink) {
    if (agent?.status !== 'online') {
      toast({ title: 'Agent 处于离线状态，请重新同步 Agent', variant: 'error' });
      return;
    }
    setConnectingSink(sink.name);
    try {
      await sinksApi.disconnectAgent(sink.id, agentId);
      setConnectedSinks((prev) => { const n = new Set(prev); n.delete(sink.name); return n; });
      toast({ title: 'Sink disconnected', variant: 'success' });
    } catch (err: unknown) {
      toast({ title: 'Failed to disconnect', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally { setConnectingSink(null); }
  }

  // Test sink connectivity
  const [testingSink, setTestingSink] = useState<number | null>(null);
  async function handleTestConnectivity(sink: Sink) {
    setTestingSink(sink.id);
    try {
      await sinksApi.testConnectivity(sink.id);
      toast({ title: '连通性测试成功', variant: 'success' });
    } catch (err: unknown) {
      toast({ title: '连通性测试失败', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally { setTestingSink(null); }
  }

  async function handleDelete() {
    if (!deleteTarget) return;

    try {
      try { await sinksApi.disconnectAgent(deleteTarget.id, agentId); } catch { /* ignore */ }
      await sinksApi.unbindAgent(deleteTarget.id, agentId);
      setBoundSinks((prev) => prev.filter((s) => s.id !== deleteTarget.id));
      setConnectedSinks((prev) => { const n = new Set(prev); n.delete(deleteTarget.name); return n; });
      toast({ title: 'Sink removed', variant: 'success' });
    } catch (err: unknown) {
      toast({ title: 'Failed to remove', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally { setDeleteTarget(null); }
  }

  if (loading) {
    return <div className="flex items-center justify-center h-64"><div className="w-6 h-6 border-2 border-[var(--brand)] border-t-transparent rounded-full animate-spin" /></div>;
  }

  const agentOnline = agent?.status === 'online';

  const availableSinks = allSinks.filter((s) => !boundSinks.find((b) => b.id === s.id));

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3">
        <button onClick={() => navigate(`/agents/${agentId}`)} className="p-2 rounded-[8px] hover:bg-[var(--bg-hover)] transition-colors" style={{ color: 'var(--text-secondary)' }}>
          <ArrowLeft size={16} />
        </button>
        <h1 className="text-[18px] font-bold" style={{ color: 'var(--text)' }}>{agent?.name || 'Agent'} — 配置</h1>
        <div className="ml-auto">
          <button onClick={handleSync} disabled={syncing}
            className="flex items-center gap-1.5 px-4 py-2 text-[12px] font-semibold rounded-[6px] transition-colors disabled:opacity-50"
            style={{ background: 'var(--brand-dim, rgba(108,92,231,0.1))', color: 'var(--brand)', border: '1px solid rgba(108,92,231,0.3)' }}>
            {syncing ? '⟳ 同步中...' : '⟳ 同步'}
          </button>
        </div>
      </div>

      {/* Token */}
      <div className="rounded-[14px] border p-5" style={{ background: 'var(--bg-card)', borderColor: 'var(--border)' }}>
        <div className="flex flex-col gap-2 md:flex-row md:items-center">
          <label className="shrink-0 text-[14px] font-bold md:w-[92px]" style={{ color: 'var(--text)' }}>API Token</label>
          <input
            value={editingToken ? editToken : maskToken(editToken)}
            onChange={(e) => setEditToken(e.target.value)}
            readOnly={!editingToken}
            placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
            className="min-w-0 flex-1 px-3 py-2.5 bg-[var(--bg-elevated)] border border-[var(--border)] rounded-[8px] text-[13px] font-mono text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--brand)] transition-all"
            style={editingToken ? {} : { color: 'var(--text-muted)' }}
          />
          <div className="flex shrink-0 justify-end">
            {!editingToken ? (
              <button onClick={() => setEditingToken(true)}
                className="w-[72px] px-4 py-2 text-[13px] font-semibold rounded-[8px] transition-colors"
                style={{ background: 'var(--bg-elevated)', color: 'var(--text-secondary)', border: '1px solid var(--border)' }}>
                配置
              </button>
            ) : (
              <button onClick={handleSaveToken} disabled={savingToken || !editToken.trim()}
                className="w-[72px] px-4 py-2 text-[13px] font-semibold text-white rounded-[8px] transition-colors disabled:opacity-50"
                style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))', boxShadow: '0 2px 10px rgba(108,92,231,0.3)' }}>
                {savingToken ? '保存中...' : '保存'}
              </button>
            )}
          </div>
        </div>
        <p className="mt-2 text-[10px] md:pl-[92px]" style={{ color: 'var(--text-muted)' }}>{t('agentDetail.editTokenHint')}</p>
      </div>

      {/* Sinks */}
      <div className="rounded-[14px] border p-5" style={{ background: 'var(--bg-card)', borderColor: 'var(--border)' }}>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-[14px] font-bold" style={{ color: 'var(--text)' }}>{'数据接收器'}</h2>
          <button onClick={() => setShowBindSink(true)} disabled={availableSinks.length === 0}
            className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-semibold rounded-[6px] transition-colors disabled:opacity-40"
            style={{ background: 'var(--brand-dim, rgba(108,92,231,0.1))', color: 'var(--brand)', border: '1px solid rgba(108,92,231,0.3)' }}>
            <Plus size={12} /> {'添加'}
          </button>
        </div>
        {boundSinks.length === 0 ? (
          <p className="text-[12px] py-4 text-center" style={{ color: 'var(--text-muted)' }}>{'暂无数据接收器'}</p>
        ) : (
          <div className="space-y-2">
            {boundSinks.map((sink) => {
              const isRunning = connectedSinks.has(sink.name);
              const isBusy = connectingSink === sink.name;
              return (
                <div key={sink.id} className="flex items-center justify-between px-4 py-3 rounded-[8px] border transition-colors"
                  style={{ background: 'var(--bg-elevated)', borderColor: 'var(--border)' }}>
                  <div className="flex items-center gap-3">
                    <div className="w-2 h-2 rounded-full" style={{ background: isRunning ? 'var(--green)' : 'var(--text-muted)' }} />
                    <span className="text-[13px] font-semibold" style={{ color: 'var(--text)' }}>{sink.name}</span>
                    <span className="text-[11px] px-1.5 py-0.5 rounded" style={{ background: 'var(--bg-card)', color: 'var(--text-muted)' }}>{sink.sink_type}</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    {!isRunning ? (
                      <button onClick={() => handleConnect(sink)} disabled={isBusy || !agentOnline}
                        className={`flex items-center gap-1 px-2.5 py-1 text-[11px] font-semibold rounded-[6px] transition-colors ${!agentOnline ? 'opacity-40 cursor-not-allowed' : 'disabled:opacity-50'}`}
                        style={{ background: agentOnline ? 'rgba(0,214,143,0.1)' : 'var(--bg-card)', color: agentOnline ? 'var(--green)' : 'var(--text-muted)', border: `1px solid ${agentOnline ? 'rgba(0,214,143,0.3)' : 'var(--border)'}` }}>
                        <Zap size={11} /> {isBusy ? '...' : '连接'}
                      </button>
                    ) : (
                      <button onClick={() => handleDisconnect(sink)} disabled={isBusy || !agentOnline}
                        className={`flex items-center gap-1 px-2.5 py-1 text-[11px] font-semibold rounded-[6px] transition-colors ${!agentOnline ? 'opacity-40 cursor-not-allowed' : 'disabled:opacity-50'}`}
                        style={{ background: agentOnline ? 'rgba(255,184,0,0.1)' : 'var(--bg-card)', color: agentOnline ? 'var(--yellow)' : 'var(--text-muted)', border: `1px solid ${agentOnline ? 'rgba(255,184,0,0.3)' : 'var(--border)'}` }}>
                        <ZapOff size={11} /> {isBusy ? '...' : '断开'}
                      </button>
                    )}
                    <button onClick={() => handleTestConnectivity(sink)} disabled={testingSink === sink.id}
                      className="flex items-center gap-1 px-2.5 py-1 text-[11px] font-semibold rounded-[6px] transition-colors disabled:opacity-50"
                      style={{ background: 'var(--bg-card)', color: 'var(--text-secondary)', border: '1px solid var(--border)' }}>
                      {testingSink === sink.id ? '...' : '🔗 测试'}
                    </button>
                    <button onClick={() => setDeleteTarget(sink)}
                      className="flex items-center gap-1 px-2.5 py-1 text-[11px] font-semibold rounded-[6px] transition-colors"
                      style={{ background: 'rgba(255,71,87,0.08)', color: 'var(--red)', border: '1px solid rgba(255,71,87,0.2)' }}>
                      <Trash2 size={11} /> {'删除'}
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <Modal
        isOpen={showBindSink}
        onClose={() => { setShowBindSink(false); setBindSinkId(''); }}
        title="添加"
        maxWidth="max-w-md"
      >
        <div className="space-y-4">
          <SearchableSelect
            options={availableSinks.map((s) => ({ value: s.id, label: s.name, sublabel: s.sink_type }))}
            value={bindSinkId ? Number(bindSinkId) : 0}
            onChange={(val) => setBindSinkId(val > 0 ? String(val) : '')}
            placeholder={t('agentSinks.selectSink')}
            searchPlaceholder="Search sinks..."
          />
          <div className="flex justify-end gap-2 pt-2">
            <button onClick={() => { setShowBindSink(false); setBindSinkId(''); }}
              className="px-4 py-2 text-[13px] rounded-[6px] border transition-colors"
              style={{ color: 'var(--text-secondary)', background: 'var(--bg-card)', borderColor: 'var(--border)' }}>
              {t('tracerEdit.cancel')}
            </button>
            <button onClick={handleBindSink} disabled={!bindSinkId || bindingSink}
              className="px-4 py-2 text-[13px] font-semibold text-white rounded-[6px] transition-colors disabled:opacity-50"
              style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))' }}>
              {bindingSink ? '...' : '添加'}
            </button>
          </div>
        </div>
      </Modal>

      <ConfirmDialog isOpen={!!deleteTarget} onCancel={() => setDeleteTarget(null)} onConfirm={handleDelete}
        title={t('confirmDialogs.removeSinkTitle')} message={t('confirmDialogs.removeSinkMessage')}
        confirmLabel={t('confirmDialogs.remove')} />
    </div>
  );
}
