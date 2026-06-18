import { useState, useEffect } from 'react';
import { t } from '@/i18n';
import { Plus, Database, Edit3, Trash2 } from 'lucide-react';
import { sinksApi, type Sink } from '@/api/client';
import { useToast } from '@/components/Toast';
import { usePageTitle } from '@/components/PageTitleContext';
import Modal from '@/components/Modal';
import ConfirmDialog from '@/components/ConfirmDialog';
import EmptyState from '@/components/EmptyState';

const SINK_TYPES = [
  { value: 'kafka', label: 'Apache Kafka' },
  { value: 'prometheus', label: 'Prometheus PushGateway' },
  { value: 'elasticsearch', label: 'Elasticsearch' },
  { value: 'redis', label: 'Redis' },
  { value: 's3', label: 'Amazon S3' },
  { value: 'webhook', label: 'Webhook' },
];

function getSinkIconInfo(type: string): { letter: string; bg: string; color: string } {
  switch (type) {
    case 'kafka': return { letter: 'K', bg: 'rgba(255,184,0,0.12)', color: '#FFB800' };
    case 'prometheus': return { letter: 'P', bg: 'rgba(0,206,201,0.1)', color: '#00CEC9' };
    case 'elasticsearch': return { letter: 'E', bg: 'rgba(108,92,231,0.1)', color: '#a29bfe' };
    case 'redis':
    case 'cache': return { letter: 'C', bg: 'rgba(108,92,231,0.1)', color: '#a29bfe' };
    default: return { letter: 'S', bg: 'rgba(108,92,231,0.1)', color: '#a29bfe' };
  }
}

function getSinkBadge(type: string): string {
  switch (type) {
    case 'kafka': return 'bg-[rgba(255,184,0,0.12)] text-[#FFB800]';
    case 'prometheus': return 'bg-[rgba(0,206,201,0.1)] text-[#00CEC9]';
    default: return 'bg-[rgba(108,92,231,0.1)] text-[#a29bfe]';
  }
}

function getSinkDescription(type: string): string {
  switch (type) {
    case 'kafka': return 'Apache Kafka';
    case 'prometheus': return 'Prometheus PushGateway';
    case 'elasticsearch': return 'Elasticsearch';
    case 'redis': return 'Redis Cache';
    case 's3': return 'Amazon S3';
    case 'webhook': return 'Webhook';
    default: return type;
  }
}

function getDefaultConfig(type: string): string {
  switch (type) {
    case 'kafka': return JSON.stringify({ brokers: ['localhost:9092'], topic: 'opentrace-events', compression: 'none' }, null, 2);
    case 'prometheus': return JSON.stringify({ endpoint: 'http://localhost:9091', job: 'opentrace' }, null, 2);
    case 'elasticsearch': return JSON.stringify({ url: 'http://localhost:9200', index: 'opentrace' }, null, 2);
    case 'redis': return JSON.stringify({ url: 'redis://localhost:6379', channel: 'opentrace' }, null, 2);
    case 's3': return JSON.stringify({ bucket: 'opentrace-data', region: 'us-east-1' }, null, 2);
    case 'webhook': return JSON.stringify({ url: 'https://example.com/webhook', method: 'POST' }, null, 2);
    default: return '{}';
  }
}

function formatEvents(count?: number): string {
  if (!count && count !== 0) return '--';
  if (count >= 1000000) return `${(count / 1000000).toFixed(1)}M`;
  if (count >= 1000) return `${(count / 1000).toFixed(1)}K`;
  return count.toString();
}

export default function Sinks() {
  
  const { setPageTitle } = usePageTitle();
  const [sinks, setSinks] = useState<Sink[]>([]);
  const [loading, setLoading] = useState(true);
  const [showCreate, setShowCreate] = useState(false);
  const [editTarget, setEditTarget] = useState<Sink | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Sink | null>(null);
  const { toast } = useToast();

  // Form state
  const [formName, setFormName] = useState('');
  const [formType, setFormType] = useState('kafka');
  const [formConfig, setFormConfig] = useState(getDefaultConfig('kafka'));
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setPageTitle(t('nav.sinks'), t('sinks.subtitle'));
    return () => setPageTitle('');
  }, [setPageTitle]);

  const load = () => {
    setLoading(true);
    sinksApi.list()
      .then(setSinks)
      .catch(() => toast({ title: 'Failed to load sinks', variant: 'error' }))
      .finally(() => setLoading(false));
  };

  useEffect(load, []); // eslint-disable-line react-hooks/exhaustive-deps

  const openCreate = () => {
    setEditTarget(null);
    setFormName('');
    setFormType('kafka');
    setFormConfig(getDefaultConfig('kafka'));
    setShowCreate(true);
  };

  const openEdit = (sink: Sink) => {
    setEditTarget(sink);
    setFormName(sink.name);
    setFormType(sink.sink_type);
    setFormConfig(sink.config);
    setShowCreate(true);
  };

  const handleSave = async () => {
    if (!formName.trim()) return;
    setSaving(true);
    try {
      if (editTarget) {
        await sinksApi.update(editTarget.id, { name: formName.trim(), sink_type: formType, config: formConfig });
        toast({ title: 'Sink updated', variant: 'success' });
        setShowCreate(false);
      } else {
        await sinksApi.create({
          name: formName.trim(),
          sink_type: formType,
          config: formConfig,
        });
        toast({ title: 'Sink created', variant: 'success' });
        setShowCreate(false);
      }
      load();
    } catch (err: unknown) {
      toast({ title: 'Save failed', description: err instanceof Error ? err.message : '', variant: 'error' });
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    try {
      await sinksApi.remove(deleteTarget.id);
      toast({ title: 'Sink deleted', variant: 'success' });
      setDeleteTarget(null);
      load();
    } catch {
      toast({ title: 'Delete failed', variant: 'error' });
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-end">
        <button
          onClick={openCreate}
          className="flex items-center gap-1.5 px-4 py-2 text-[12px] font-semibold text-white rounded-[6px] transition-all"
          style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))', boxShadow: '0 2px 10px rgba(108,92,231,0.3)' }}
        >
          <Plus size={14} />
          {t('sinks.createSink')}
        </button>
      </div>

      {loading ? (
        <div className="space-y-3">
          {[1, 2, 3].map((i) => <div key={i} className="h-16 bg-[var(--bg-card)] rounded-[14px] animate-pulse" />)}
        </div>
      ) : sinks.length === 0 ? (
        <EmptyState icon={Database} title={t('sinks.noSinks')} description={t('sinks.noSinksDesc')} actionLabel={t('sinks.createSink')} onAction={openCreate} />
      ) : (
        <div className="bg-[var(--bg-card)] border border-[var(--border)] rounded-[14px] overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full border-collapse">
              <thead>
                <tr>
                  {[t('sinks.name'), t('sinks.type'), t('sinks.endpoint'), t('sinks.agents'), t('sinks.eventsSent'), t('sinks.status'), t('sinks.actions')].map((h) => (
                    <th key={h} className="text-left px-4 py-2.5 text-[11px] font-semibold uppercase tracking-[0.8px] text-[var(--text-muted)] bg-[var(--bg-elevated)] border-b border-[var(--border)]">{h}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {sinks.map((sink) => {
                  const icon = getSinkIconInfo(sink.sink_type);
                  const agentCount = sink.agent_count;
                  let endpoint = sink.config;
                  try {
                    const cfg = JSON.parse(sink.config);
                    endpoint = cfg.brokers?.[0] || cfg.endpoint || cfg.url || sink.config;
                  } catch { /* keep as-is */ }
                  return (
                    <tr key={sink.id} className="hover:bg-[var(--bg-hover)] transition-colors cursor-pointer" onClick={() => window.location.href = `/sinks/${sink.id}`}>
                      <td className="px-4 py-3 border-b border-[var(--border)]">
                        <div className="flex items-center gap-2.5">
                          <div
                            className="w-9 h-9 rounded-[6px] flex items-center justify-center font-bold text-[12px] flex-shrink-0"
                            style={{ background: icon.bg, color: icon.color }}
                          >
                            {icon.letter}
                          </div>
                          <div>
                            <strong className="text-[var(--text-primary)] text-[13px]">{sink.name}</strong>
                            <div className="text-[11px] text-[var(--text-muted)]">{getSinkDescription(sink.sink_type)}</div>
                          </div>
                        </div>
                      </td>
                      <td className="px-4 py-3 border-b border-[var(--border)]">
                        <span className={`inline-flex items-center px-2.5 py-0.5 rounded-[20px] text-[11px] font-semibold ${getSinkBadge(sink.sink_type)}`}>
                          {sink.sink_type === 'kafka' ? 'Kafka' : sink.sink_type === 'prometheus' ? 'Prom' : sink.sink_type === 'cache' ? 'Cache' : sink.sink_type}
                        </span>
                      </td>
                      <td className="px-4 py-3 border-b border-[var(--border)] text-[11px]" style={{ fontFamily: "'JetBrains Mono', monospace" }}>
                        {endpoint}
                      </td>
                      <td className="px-4 py-3 border-b border-[var(--border)]">
                        <strong className="text-[var(--text-primary)]">{agentCount ?? '--'}</strong>
                      </td>
                      <td className="px-4 py-3 border-b border-[var(--border)] font-semibold text-[var(--green)]" style={{ fontFamily: "'JetBrains Mono', monospace" }}>
                        {formatEvents(sink.events_sent)}
                      </td>
                      <td className="px-4 py-3 border-b border-[var(--border)]">
                        <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-[20px] text-[11px] font-semibold bg-[var(--green-dim)] text-[var(--green)]">
                          <span className="w-2 h-2 rounded-full bg-[var(--green)] shadow-[0_0_6px_1px_rgba(0,214,143,0.5)]" />
                          Active
                        </span>
                      </td>
                      <td className="px-4 py-3 border-b border-[var(--border)]">
                        <div className="flex items-center gap-3">
                          <button
                            type="button"
                            onClick={(e) => { e.stopPropagation(); openEdit(sink); }}
                            className="flex w-10 flex-col items-center gap-1 text-[var(--text-muted)] transition-colors hover:text-[var(--text-primary)]"
                            title="配置"
                          >
                            <Edit3 size={15} strokeWidth={1.8} />
                            <span className="text-[10px] leading-none">配置</span>
                          </button>
                          <button
                            type="button"
                            onClick={(e) => { e.stopPropagation(); setDeleteTarget(sink); }}
                            className="flex w-10 flex-col items-center gap-1 text-[var(--text-muted)] transition-colors hover:text-[var(--red)]"
                            title="删除"
                          >
                            <Trash2 size={15} strokeWidth={1.8} />
                            <span className="text-[10px] leading-none">删除</span>
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Create/Edit Modal */}
      <Modal isOpen={showCreate} onClose={() => setShowCreate(false)} title={editTarget ? t('sinks.editSink') : t('sinks.createSink')}>
        <div className="space-y-4">
          <div>
            <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('sinks.sinkName')} *</label>
            <input
              value={formName}
              onChange={(e) => setFormName(e.target.value)}
              placeholder="kafka-prod-2"
              className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all"
            />
          </div>
          <div>
            <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('sinks.sinkType')} *</label>
            <select
              value={formType}
              onChange={(e) => { setFormType(e.target.value); if (!editTarget) setFormConfig(getDefaultConfig(e.target.value)); }}
              className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all appearance-none cursor-pointer"
            >
              {SINK_TYPES.map((t) => (
                <option key={t.value} value={t.value}>{t.label}</option>
              ))}
            </select>
          </div>

          {/* Type-specific fields */}
          {formType === 'kafka' && (
            <>
              <div>
                <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('sinks.bootstrapServers')} *</label>
                <input
                  value={(() => { try { return JSON.parse(formConfig).brokers?.[0] || ''; } catch { return ''; } })()}
                  onChange={(e) => {
                    try {
                      const cfg = JSON.parse(formConfig);
                      cfg.brokers = [e.target.value];
                      setFormConfig(JSON.stringify(cfg, null, 2));
                    } catch { /* keep as-is */ }
                  }}
                  placeholder="kafka-01:9092"
                  className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] font-mono placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all"
                />
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('sinks.topic')} *</label>
                  <input
                    value={(() => { try { return JSON.parse(formConfig).topic || ''; } catch { return ''; } })()}
                    onChange={(e) => {
                      try {
                        const cfg = JSON.parse(formConfig);
                        cfg.topic = e.target.value;
                        setFormConfig(JSON.stringify(cfg, null, 2));
                      } catch { /* keep as-is */ }
                    }}
                    placeholder="opentrace-events"
                    className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all"
                  />
                </div>
                <div>
                  <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('sinks.compression')}</label>
                  <select
                    value={(() => { try { return JSON.parse(formConfig).compression || 'none'; } catch { return 'none'; } })()}
                    onChange={(e) => {
                      try {
                        const cfg = JSON.parse(formConfig);
                        cfg.compression = e.target.value;
                        setFormConfig(JSON.stringify(cfg, null, 2));
                      } catch { /* keep as-is */ }
                    }}
                    className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all appearance-none cursor-pointer"
                  >
                    <option value="none">none</option>
                    <option value="gzip">gzip</option>
                    <option value="snappy">snappy</option>
                  </select>
                </div>
              </div>
            </>
          )}
          {formType === 'prometheus' && (
            <>
              <div>
                <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">PushGateway URL *</label>
                <input
                  value={(() => { try { return JSON.parse(formConfig).endpoint || ''; } catch { return ''; } })()}
                  onChange={(e) => {
                    try {
                      const cfg = JSON.parse(formConfig);
                      cfg.endpoint = e.target.value;
                      setFormConfig(JSON.stringify(cfg, null, 2));
                    } catch { /* keep as-is */ }
                  }}
                  placeholder="http://pushgateway:9091"
                  className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] font-mono placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all"
                />
              </div>
              <div>
                <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">Job Name</label>
                <input
                  value={(() => { try { return JSON.parse(formConfig).job || ''; } catch { return ''; } })()}
                  onChange={(e) => {
                    try {
                      const cfg = JSON.parse(formConfig);
                      cfg.job = e.target.value;
                      setFormConfig(JSON.stringify(cfg, null, 2));
                    } catch { /* keep as-is */ }
                  }}
                  placeholder="opentrace"
                  className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all"
                />
              </div>
            </>
          )}
          {formType === 'redis' && (
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">Max Entries</label>
                <input
                  type="number"
                  defaultValue={10000}
                  className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all"
                />
              </div>
              <div>
                <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">TTL (seconds)</label>
                <input
                  type="number"
                  defaultValue={3600}
                  className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all"
                />
              </div>
            </div>
          )}
          {!['kafka', 'prometheus', 'redis'].includes(formType) && (
            <div>
              <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">Configuration <span className="text-[var(--text-muted)]">(JSON)</span></label>
              <textarea
                value={formConfig}
                onChange={(e) => setFormConfig(e.target.value)}
                rows={6}
                className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] font-mono focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all resize-none"
              />
            </div>
          )}
          <div>
            <label className="block text-[12px] font-semibold text-[var(--text-secondary)] mb-1.5">{t('sinks.description')}</label>
            <textarea
              placeholder="Optional description..."
              rows={3}
              className="w-full px-3 py-2.5 bg-[var(--bg-card)] border border-[var(--border)] rounded-[6px] text-[13px] text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--brand)] focus:shadow-[0_0_0_3px_rgba(108,92,231,0.15)] transition-all resize-none"
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <button onClick={() => setShowCreate(false)} className="px-4 py-2 text-[13px] text-[var(--text-secondary)] bg-[var(--bg-card)] border border-[var(--border)] hover:bg-[var(--bg-hover)] rounded-[6px] transition-colors">{t('sinks.cancel')}</button>
            <button
              onClick={handleSave}
              disabled={saving || !formName.trim()}
              className="px-4 py-2 text-[13px] text-white rounded-[6px] transition-colors disabled:opacity-50"
              style={{ background: 'linear-gradient(135deg, var(--brand), var(--brand-dark))', boxShadow: '0 2px 10px rgba(108,92,231,0.3)' }}
            >
              {saving ? 'Saving...' : editTarget ? 'Update' : 'Create'}
            </button>
          </div>
        </div>
      </Modal>

      <ConfirmDialog isOpen={!!deleteTarget} title="Delete Sink" message={`Delete sink "${deleteTarget?.name}"? All agent bindings will be removed.`} onConfirm={handleDelete} onCancel={() => setDeleteTarget(null)} />
    </div>
  );
}
