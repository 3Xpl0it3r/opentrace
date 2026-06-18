import { useState, useEffect } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import { ArrowLeft, Unlink, Edit3, Trash2 } from 'lucide-react';
import { sinksApi, agentsApi, type Sink, type Agent } from '@/api/client';
import { useToast } from '@/components/Toast';
import { t } from '@/i18n';
import Modal from '@/components/Modal';
import ConfirmDialog from '@/components/ConfirmDialog';

function getSinkIconInfo(type: string): { letter: string; bg: string; color: string } {
  switch (type) {
    case 'kafka': return { letter: 'K', bg: 'rgba(255,184,0,0.12)', color: '#FFB800' };
    case 'prometheus': return { letter: 'P', bg: 'rgba(0,206,201,0.1)', color: '#00CEC9' };
    case 'elasticsearch': return { letter: 'E', bg: 'rgba(108,92,231,0.1)', color: '#a29bfe' };
    default: return { letter: 'S', bg: 'rgba(108,92,231,0.1)', color: '#a29bfe' };
  }
}

function getSinkDescription(type: string): string {
  switch (type) {
    case 'kafka': return 'Apache Kafka';
    case 'prometheus': return 'Prometheus PushGateway';
    case 'elasticsearch': return 'Elasticsearch';
    default: return type;
  }
}

function formatEvents(count?: number): string {
  if (!count && count !== 0) return '--';
  if (count >= 1000000) return `${(count / 1000000).toFixed(1)}M`;
  if (count >= 1000) return `${(count / 1000).toFixed(1)}K`;
  return count.toString();
}

export default function SinkDetail() {
  const { id } = useParams();
  const navigate = useNavigate();
  const sinkId = Number(id);
  const [sink, setSink] = useState<Sink | null>(null);
  const [boundAgents, setBoundAgents] = useState<Agent[]>([]);
  const [allAgents, setAllAgents] = useState<Agent[]>([]);
  const [loading, setLoading] = useState(true);
  const { toast } = useToast();
  const [showEdit, setShowEdit] = useState(false);
  const [showBind, setShowBind] = useState(false);
  const [bindAgentId, setBindAgentId] = useState<number | ''>('');
  const [deleteTarget, setDeleteTarget] = useState(false);
  const [formName, setFormName] = useState('');
  const [formType, setFormType] = useState('');
  const [formConfig, setFormConfig] = useState('');
  const [saving, setSaving] = useState(false);

  const load = async () => {
    try {
      const [s, agentIds, allA] = await Promise.all([
        sinksApi.get(sinkId), sinksApi.getAgents(sinkId), agentsApi.list(),
      ]);
      setSink(s);
      setAllAgents(allA);
      setBoundAgents(allA.filter((a) => agentIds.includes(a.id)));
    } catch {
      toast({ title: t('sinkDetail.toast.loadFailed'), variant: 'error' });
    } finally { setLoading(false); }
  };

  useEffect(() => { load(); }, [sinkId]);

  const openEdit = () => {
    if (!sink) return;
    setFormName(sink.name); setFormType(sink.sink_type); setFormConfig(sink.config);
    setShowEdit(true);
  };

  const handleSave = async () => {
    if (!formName.trim()) return;
    setSaving(true);
    try {
      const updated = await sinksApi.update(sinkId, { name: formName.trim(), sink_type: formType, config: formConfig });
      setSink(updated); setShowEdit(false);
      toast({ title: t('sinkDetail.toast.updated'), variant: 'success' });
    } catch {
      toast({ title: t('sinkDetail.toast.updateFailed'), variant: 'error' });
    } finally { setSaving(false); }
  };

  const handleDelete = async () => {
    try {
      await sinksApi.remove(sinkId);
      toast({ title: t('sinkDetail.toast.deleted'), variant: 'success' });
      navigate('/sinks');
    } catch {
      toast({ title: t('sinkDetail.toast.deleteFailed'), variant: 'error' });
    }
  };

  const handleBind = async () => {
    if (!bindAgentId) return;
    try {
      await sinksApi.bindAgent(sinkId, Number(bindAgentId));
      const agent = allAgents.find((a) => a.id === Number(bindAgentId));
      if (agent) setBoundAgents((prev) => [...prev, agent]);
      setShowBind(false); setBindAgentId('');
      toast({ title: t('sinkDetail.toast.bound'), variant: 'success' });
    } catch {
      toast({ title: t('sinkDetail.toast.bindFailed'), variant: 'error' });
    }
  };

  const handleUnbind = async (agentId: number) => {
    try {
      await sinksApi.unbindAgent(sinkId, agentId);
      setBoundAgents((prev) => prev.filter((a) => a.id !== agentId));
      toast({ title: t('sinkDetail.toast.disconnected'), variant: 'success' });
    } catch {
      toast({ title: t('sinkDetail.toast.disconnectFailed'), variant: 'error' });
    }
  };

  if (loading) return <div className="space-y-6"><div className="h-10 w-64 bg-[var(--bg-card)] rounded-[14px] animate-pulse" /><div className="h-32 bg-[var(--bg-card)] rounded-[14px] animate-pulse" /></div>;

  if (!sink) return <div className="text-center py-20"><p className="text-[var(--text-muted)]">{t('sinkDetail.notFound')}</p><Link to="/sinks" className="text-[var(--accent)] text-sm mt-2 inline-block hover:underline">{t('sinkDetail.backToSinks')}</Link></div>;

  const icon = getSinkIconInfo(sink.sink_type);
  let endpoint = sink.config; let topic = '';
  try { const cfg = JSON.parse(sink.config); endpoint = cfg.brokers?.[0] || cfg.endpoint || cfg.url || sink.config; topic = cfg.topic || ''; } catch {}
  const eventsPerSec = sink.events_per_sec;
  const deliveryRate = sink.delivery_rate;

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <button onClick={() => navigate('/sinks')} className="w-9 h-9 rounded-[6px] bg-[var(--bg-card)] border border-[var(--border)] flex items-center justify-center text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)] transition-colors"><ArrowLeft size={16} /></button>
        <div className="w-11 h-11 rounded-[6px] flex items-center justify-center font-bold text-[16px] flex-shrink-0" style={{ background: icon.bg, color: icon.color }}>{icon.letter}</div>
        <div className="flex-1 min-w-0">
          <h1 className="text-[20px] font-bold text-[var(--text-primary)]">{sink.name}</h1>
          <p className="text-[var(--text-secondary)] text-[13px] mt-0.5">{getSinkDescription(sink.sink_type)}</p>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={openEdit} className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-semibold rounded-[6px] bg-[var(--bg-card)] text-[var(--text-secondary)] border border-[var(--border)] hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)] transition-colors"><Edit3 size={13} /> {t('sinkDetail.edit')}</button>
          <button onClick={() => setDeleteTarget(true)} className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-semibold rounded-[6px] bg-[var(--red-dim)] text-[var(--red)] border border-[rgba(255,77,106,0.2)] hover:bg-[rgba(255,77,106,0.2)] transition-colors"><Trash2 size={13} /> {t('sinkDetail.delete')}</button>
        </div>
      </div>

      <div className="flex items-center gap-5 flex-wrap text-[12px] text-[var(--text-secondary)]">
        <span>{t('sinkDetail.endpoint')}: <strong className="text-[var(--text-primary)]" style={{ fontFamily: "'JetBrains Mono', monospace" }}>{endpoint}</strong></span>
        {topic && <span>{t('sinkDetail.topic')}: <strong className="text-[var(--text-primary)]">{topic}</strong></span>}
        <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-[20px] text-[11px] font-semibold bg-[var(--green-dim)] text-[var(--green)]">
          <span className="w-2 h-2 rounded-full bg-[var(--green)] shadow-[0_0_6px_1px_rgba(0,214,143,0.5)]" />{t('sinkDetail.active')}
        </span>
      </div>

      <div className="grid grid-cols-4 gap-4">
        {[{ label: t('sinkDetail.connectedAgents'), value: boundAgents.length.toString(), gradient: 'linear-gradient(90deg, #6C5CE7, #a29bfe)' },
          { label: t('sinkDetail.eventsSent'), value: formatEvents(sink.events_sent), gradient: 'linear-gradient(90deg, #00D68F, #34d399)' },
          { label: t('sinkDetail.eventsPerSec'), value: eventsPerSec === undefined ? '--' : eventsPerSec.toLocaleString(), gradient: 'linear-gradient(90deg, #3B82F6, #60a5fa)' },
          { label: t('sinkDetail.deliveryRate'), value: deliveryRate === undefined ? '--' : `${deliveryRate.toFixed(1)}%`, gradient: 'linear-gradient(90deg, #FFB800, #fcd34d)' },
        ].map((card) => (
          <div key={card.label} className="relative bg-[var(--bg-card)] border border-[var(--border)] rounded-[14px] p-4 overflow-hidden">
            <div className="absolute top-0 left-0 right-0 h-[2px]" style={{ background: card.gradient }} />
            <div className="text-[22px] font-bold text-[var(--text-primary)] leading-none tracking-tight">{card.value}</div>
            <div className="text-[12px] text-[var(--text-secondary)] font-medium mt-1">{card.label}</div>
          </div>
        ))}
      </div>

      <div className="bg-[var(--bg-card)] border border-[var(--border)] rounded-[14px] overflow-hidden">
        <div className="px-5 py-4 border-b border-[var(--border)]">
          <h3 className="text-[14px] font-semibold text-[var(--text-primary)]">{t('sinkDetail.connectedAgents')}</h3>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full border-collapse">
            <thead><tr>{[t('sinkDetail.table.agent'), t('sinkDetail.table.group'), t('sinkDetail.table.status'), t('sinkDetail.table.eventsSent'), t('sinkDetail.table.since'), t('sinkDetail.table.actions')].map((h) => (
              <th key={h} className="text-left px-4 py-2.5 text-[11px] font-semibold uppercase tracking-[0.8px] text-[var(--text-muted)] bg-[var(--bg-elevated)] border-b border-[var(--border)]">{h}</th>
            ))}</tr></thead>
            <tbody>
              {boundAgents.length === 0 ? (
                <tr><td colSpan={6} className="text-center py-10 text-[var(--text-muted)] text-sm">{t('sinkDetail.table.noAgents')}</td></tr>
              ) : boundAgents.map((agent) => (
                <tr key={agent.id} className="hover:bg-[var(--bg-hover)] transition-colors">
                  <td className="px-4 py-3 border-b border-[var(--border)]"><strong className="text-[var(--text-primary)] text-[13px]">{agent.name}</strong><div className="text-[11px] text-[var(--text-muted)] font-mono">{agent.host}</div></td>
                  <td className="px-4 py-3 border-b border-[var(--border)]"><span className="inline-flex items-center px-2 py-0.5 bg-[var(--bg-elevated)] text-[var(--text-secondary)] text-[10px] font-medium rounded-[4px] border border-[var(--border)]">{agent.group_name || t('sinkDetail.table.noGroup')}</span></td>
                  <td className="px-4 py-3 border-b border-[var(--border)]"><span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-[20px] text-[11px] font-semibold bg-[var(--green-dim)] text-[var(--green)]"><span className="w-2 h-2 rounded-full bg-[var(--green)] shadow-[0_0_6px_1px_rgba(0,214,143,0.5)]" />{t('sinkDetail.table.streaming')}</span></td>
                  <td className="px-4 py-3 border-b border-[var(--border)]" style={{ fontFamily: "'JetBrains Mono', monospace" }}>--</td>
                  <td className="px-4 py-3 border-b border-[var(--border)] text-[var(--text-secondary)]">{new Date(agent.created_at).toLocaleDateString()}</td>
                  <td className="px-4 py-3 border-b border-[var(--border)]"><button onClick={() => handleUnbind(agent.id)} className="flex items-center gap-1 px-2.5 py-1 text-[11px] font-semibold rounded-[6px] bg-[var(--red-dim)] text-[var(--red)] border border-[rgba(255,77,106,0.2)] hover:bg-[rgba(255,77,106,0.2)] transition-colors"><Unlink size={12} /> {t('sinkDetail.table.disconnect')}</button></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      <Modal isOpen={showEdit} onClose={() => setShowEdit(false)} title={t('sinkDetail.editModal.title')}>
        <div className="space-y-4">
          <div><label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('sinkDetail.editModal.name')}</label><input value={formName} onChange={(e) => setFormName(e.target.value)} className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all" /></div>
          <div><label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('sinkDetail.editModal.type')}</label><input value={formType} onChange={(e) => setFormType(e.target.value)} className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all" /></div>
          <div><label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('sinkDetail.editModal.config')}</label><textarea value={formConfig} onChange={(e) => setFormConfig(e.target.value)} rows={6} className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] font-mono focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all resize-none" /></div>
          <div className="flex justify-end gap-2 pt-2">
            <button onClick={() => setShowEdit(false)} className="px-4 py-2 text-[13px] text-[var(--text-secondary)] bg-[var(--bg-card)] border border-[var(--border)] hover:bg-[var(--bg-hover)] rounded-[6px] transition-colors">{t('common.cancel')}</button>
            <button onClick={handleSave} disabled={saving} className="px-4 py-2 text-[13px] text-white rounded-[6px] transition-colors disabled:opacity-50" style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))', boxShadow: '0 2px 10px rgba(108,92,231,0.3)' }}>{saving ? t('tracerEdit.saving') : t('common.save')}</button>
          </div>
        </div>
      </Modal>

      <Modal isOpen={showBind} onClose={() => setShowBind(false)} title={t('sinkDetail.bindModal.title')} maxWidth="max-w-sm">
        <div className="space-y-4">
          <div><label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('sinkDetail.bindModal.select')}</label>
            <select value={bindAgentId} onChange={(e) => setBindAgentId(e.target.value ? Number(e.target.value) : '')} className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all">
              <option value="">{t('sinkDetail.bindModal.placeholder')}</option>
              {allAgents.filter((a) => !boundAgents.find((b) => b.id === a.id)).map((a) => (<option key={a.id} value={a.id}>{a.name} ({a.host})</option>))}
            </select>
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <button onClick={() => setShowBind(false)} className="px-4 py-2 text-[13px] text-[var(--text-secondary)] bg-[var(--bg-card)] border border-[var(--border)] hover:bg-[var(--bg-hover)] rounded-[6px] transition-colors">{t('common.cancel')}</button>
            <button onClick={handleBind} disabled={!bindAgentId} className="px-4 py-2 text-[13px] text-white rounded-[6px] transition-colors disabled:opacity-50" style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))', boxShadow: '0 2px 10px rgba(108,92,231,0.3)' }}>{t('sinkDetail.bindModal.bind')}</button>
          </div>
        </div>
      </Modal>

      <ConfirmDialog isOpen={deleteTarget} title={t('sinkDetail.deleteTitle')} message={t('sinkDetail.deleteMsg')} onConfirm={handleDelete} onCancel={() => setDeleteTarget(false)} />
    </div>
  );
}
