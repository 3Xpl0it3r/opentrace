import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Plus, Trash2, Zap, ZapOff } from 'lucide-react';
import { agentsApi, sinksApi, type Agent, type Sink } from '@/api/client';
import { t } from '@/i18n';
import { useToast } from '@/components/Toast';
import SearchableSelect from '@/components/SearchableSelect';
import ConfirmDialog from '@/components/ConfirmDialog';
import Modal from '@/components/Modal';

export default function AgentSinks() {
  const { id } = useParams();
  const navigate = useNavigate();
  const agentId = Number(id);
  const [agent, setAgent] = useState<Agent | null>(null);
  const [boundSinks, setBoundSinks] = useState<Sink[]>([]);
  const [allSinks, setAllSinks] = useState<Sink[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAdd, setShowAdd] = useState(false);
  const [addSinkId, setAddSinkId] = useState<number>(0);
  const [adding, setAdding] = useState(false);
  const [removingId, setRemovingId] = useState<number | null>(null);
  const [confirmRemove, setConfirmRemove] = useState<Sink | null>(null);
  const [connectedIds, setConnectedIds] = useState<Set<number>>(new Set());
  const [connectingId, setConnectingId] = useState<number | null>(null);
  const [disconnectingId, setDisconnectingId] = useState<number | null>(null);
  const { toast } = useToast();

  const load = async () => {
    try {
      const [a, allS] = await Promise.all([
        agentsApi.get(agentId),
        sinksApi.list(),
      ]);
      setAgent(a);
      setAllSinks(allS);

      const bound: Sink[] = [];
      for (const s of allS) {
        try {
          const agentIds = await sinksApi.getAgents(s.id);
          if (agentIds.includes(agentId)) bound.push(s);
        } catch { /* ignore */ }
      }
      setBoundSinks(bound);
      // Check which sinks are actually running on the agent
      try {
        const runningNames = await agentsApi.getSinkNames(agentId);
        const runningSet = new Set(runningNames);
        const connected = new Set<number>();
        for (const s of bound) {
          if (runningSet.has(s.name)) connected.add(s.id);
        }
        setConnectedIds(connected);
      } catch { /* agent offline or error — all disconnected */ }
    } catch (err: unknown) {
      toast({ title: 'Failed to load', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, [agentId]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleAdd = async () => {
    if (!addSinkId) return;
    setAdding(true);
    try {
      await sinksApi.bindAgent(addSinkId, agentId);
      const sink = allSinks.find((s) => s.id === addSinkId);
      if (sink) setBoundSinks((prev) => [...prev, sink]);
      setShowAdd(false);
      setAddSinkId(0);
      toast({ title: '已添加，请点击「连接」启动数据接收器', variant: 'success' });
    } catch (err: unknown) {
      toast({ title: '添加失败', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally {
      setAdding(false);
    }
  };

  const handleRemove = async () => {
    if (!confirmRemove) return;
    setRemovingId(confirmRemove.id);
    try {
      // Stop sink on agent first (ignore errors — agent may be offline)
      try { await sinksApi.disconnectAgent(confirmRemove.id, agentId); } catch { /* ignore */ }
      await sinksApi.unbindAgent(confirmRemove.id, agentId);
      setBoundSinks((prev) => prev.filter((s) => s.id !== confirmRemove.id));
      setConnectedIds((prev) => { const next = new Set(prev); next.delete(confirmRemove.id); return next; });
      toast({ title: '已删除', variant: 'success' });
    } catch {
      toast({ title: '删除失败', variant: 'error' });
    } finally {
      setRemovingId(null);
      setConfirmRemove(null);
    }
  };

  const handleConnect = async (sink: Sink) => {
    setConnectingId(sink.id);
    try {
      await sinksApi.connectAgent(sink.id, agentId);
      setConnectedIds((prev) => new Set(prev).add(sink.id));
      toast({ title: '已连接', variant: 'success' });
    } catch (err: unknown) {
      toast({ title: '连接失败', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally {
      setConnectingId(null);
    }
  };

  const handleDisconnect = async (sink: Sink) => {
    setDisconnectingId(sink.id);
    try {
      await sinksApi.disconnectAgent(sink.id, agentId);
      setConnectedIds((prev) => {
        const next = new Set(prev);
        next.delete(sink.id);
        return next;
      });
      toast({ title: '已断开', variant: 'success' });
    } catch (err: unknown) {
      toast({ title: '断开失败', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally {
      setDisconnectingId(null);
    }
  };

  const unboundSinks = allSinks.filter((s) => !boundSinks.find((b) => b.id === s.id));

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
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-4">
        <button
          onClick={() => navigate('/agents/' + agentId)}
          className="w-9 h-9 rounded-[6px] bg-[var(--bg-card)] border border-[var(--border)] flex items-center justify-center text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)] transition-colors"
        >
          <ArrowLeft size={16} />
        </button>
        <div className="flex-1 min-w-0">
          <h1 className="text-[20px] font-bold text-[var(--text-primary)]">
            {agent.name} — {t('nav.sinks')}
          </h1>
          <p className="text-[var(--text-secondary)] text-[13px] mt-0.5">
            {boundSinks.length} 个数据接收器已绑定
          </p>
        </div>
        <button
          onClick={() => setShowAdd(true)}
          disabled={unboundSinks.length === 0}
          className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-semibold rounded-[6px] transition-colors disabled:opacity-40"
          style={{
            background: 'var(--brand-dim, rgba(108,92,231,0.1))',
            color: 'var(--brand)',
            border: '1px solid rgba(108,92,231,0.3)',
          }}
        >
          <Plus size={12} />
          添加数据接收器
        </button>
      </div>

      {/* Sink List */}
      <div className="bg-[var(--bg-card)] border border-[var(--border)] rounded-[14px] overflow-hidden">
        <div className="p-5">
          {boundSinks.length === 0 ? (
            <div className="text-center text-[var(--text-muted)] text-[13px] py-8">
              此节点暂无数据接收器
              <button
                onClick={() => setShowAdd(true)}
                className="block mx-auto mt-2 text-[var(--brand)] text-[12px] font-semibold hover:underline"
              >
                添加数据接收器
              </button>
            </div>
          ) : (
            <div className="space-y-2">
              {boundSinks.map((sink) => {
                const iconInfo = (() => {
                  switch (sink.sink_type) {
                    case 'kafka': return { letter: 'K', bg: 'rgba(255,184,0,0.12)', color: '#FFB800' };
                    case 'prometheus': return { letter: 'P', bg: 'rgba(0,206,201,0.1)', color: '#00CEC9' };
                    default: return { letter: 'S', bg: 'rgba(108,92,231,0.1)', color: '#a29bfe' };
                  }
                })();
                return (
                  <div
                    key={sink.id}
                    className="flex items-center justify-between bg-[var(--bg-elevated)] border border-[var(--border)] rounded-[8px] px-4 py-3 hover:border-[var(--brand)] transition-colors"
                  >
                    <div className="flex items-center gap-3">
                      <div
                        className="w-8 h-8 rounded-[6px] flex items-center justify-center font-bold text-[11px] flex-shrink-0"
                        style={{ background: iconInfo.bg, color: iconInfo.color }}
                      >
                        {iconInfo.letter}
                      </div>
                      <div>
                        <div className="text-[13px] font-semibold text-[var(--text-primary)]">{sink.name}</div>
                        <div className="text-[11px] text-[var(--text-muted)]">{sink.sink_type}</div>
                      </div>
                    </div>
                    <div className="flex items-center gap-1.5">
                      {connectedIds.has(sink.id) ? (
                        <button
                          onClick={() => handleDisconnect(sink)}
                          disabled={disconnectingId === sink.id}
                          className="flex items-center gap-1 px-2.5 py-1.5 text-[11px] font-semibold rounded-[6px] transition-colors"
                          style={{
                            background: 'rgba(0,206,201,0.1)',
                            color: '#00CEC9',
                            border: '1px solid rgba(0,206,201,0.3)',
                          }}
                        >
                          <ZapOff size={11} />
                          断开
                        </button>
                      ) : (
                        <button
                          onClick={() => handleConnect(sink)}
                          disabled={connectingId === sink.id}
                          className="flex items-center gap-1 px-2.5 py-1.5 text-[11px] font-semibold rounded-[6px] transition-colors"
                          style={{
                            background: 'var(--brand-dim, rgba(108,92,231,0.1))',
                            color: 'var(--brand)',
                            border: '1px solid rgba(108,92,231,0.3)',
                          }}
                        >
                          <Zap size={11} />
                          连接
                        </button>
                      )}
                      <button
                        onClick={() => setConfirmRemove(sink)}
                        disabled={removingId === sink.id}
                        className="flex items-center gap-1 px-2.5 py-1.5 text-[11px] font-semibold rounded-[6px] bg-[var(--red-dim)] text-[var(--red)] border border-[rgba(255,77,106,0.2)] hover:bg-[rgba(255,77,106,0.2)] transition-colors"
                      >
                        <Trash2 size={11} />
                        删除
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      <Modal
        isOpen={showAdd}
        onClose={() => { setShowAdd(false); setAddSinkId(0); }}
        title="添加数据接收器"
        maxWidth="max-w-md"
      >
        <div className="space-y-4">
          <SearchableSelect
            options={unboundSinks.map((s) => ({
              value: s.id,
              label: s.name,
              sublabel: s.sink_type,
            }))}
            value={addSinkId}
            onChange={(val) => setAddSinkId(val)}
            placeholder="选择数据接收器..."
            searchPlaceholder="搜索..."
          />
          <div className="flex justify-end gap-2 pt-2">
            <button
              onClick={() => { setShowAdd(false); setAddSinkId(0); }}
              className="px-4 py-2 text-[13px] text-[var(--text-secondary)] bg-[var(--bg-card)] border border-[var(--border)] hover:bg-[var(--bg-hover)] rounded-[6px] transition-colors"
            >
              {t('common.cancel')}
            </button>
            <button
              onClick={handleAdd}
              disabled={adding || !addSinkId}
              className="px-4 py-2 text-[13px] text-white rounded-[6px] transition-colors disabled:opacity-50"
              style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))', boxShadow: '0 2px 10px rgba(108,92,231,0.3)' }}
            >
              {adding ? '...' : '绑定'}
            </button>
          </div>
        </div>
      </Modal>

      <ConfirmDialog
        isOpen={!!confirmRemove}
        title="删除数据接收器"
        message={`确定要从 ${agent.name} 删除「${confirmRemove?.name || ''}」吗？此操作将停止并移除该数据接收器。`}
        confirmLabel="删除"
        onConfirm={handleRemove}
        onCancel={() => setConfirmRemove(null)}
      />
    </div>
  );
}
