import { useState, useRef, useEffect } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { t } from '@/i18n';
import { ArrowLeft, Play, Square, Trash2, Terminal } from 'lucide-react';
import { agentsApi, authHeaders, type Agent } from '@/api/client';
import { useToast } from '@/components/Toast';

interface OutputLine {
  id: number;
  time: string;
  text: string;
  type: 'info' | 'data' | 'error' | 'system';
}

export default function Debug() {
  const { id } = useParams();
  const navigate = useNavigate();

  const { toast } = useToast();
  const agentId = Number(id);

  const [agent, setAgent] = useState<Agent | null>(null);
  const [loading, setLoading] = useState(true);
  const [param, setParam] = useState('');
  const [status, setStatus] = useState<'idle' | 'running' | 'stopped'>('idle');
  const statusRef = useRef(status);
  statusRef.current = status;
  const [output, setOutput] = useState<OutputLine[]>([]);
  const [wsConnected, setWsConnected] = useState(false);
  const watchAbortRef = useRef<AbortController | null>(null);

  // Get tracer info from URL params
  const searchParams = new URLSearchParams(window.location.search);
  const tracerName = searchParams.get('tracer') || '';
  const tracerDesc = searchParams.get('desc') || '';
  const outputRef = useRef<HTMLDivElement>(null);
  
  let lineId = useRef(0);

  // Load agent info
  useEffect(() => {
    agentsApi.get(agentId)
      .then(setAgent)
      .catch(() => toast({ title: 'Failed to load agent', variant: 'error' }))
      .finally(() => setLoading(false));
  }, [agentId]);

  // Auto scroll output
  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [output]);

  const addOutput = (text: string, type: OutputLine['type'] = 'data') => {
    lineId.current += 1;
    const now = new Date();
    const time = now.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
    setOutput(prev => [...prev, { id: lineId.current, time, text, type }]);
  };

  const cleanup = () => {
    if (watchAbortRef.current) {
      watchAbortRef.current.abort();
      watchAbortRef.current = null;
    }
  };

  const handleRun = async () => {
    cleanup();

    setStatus('running');
    addOutput('[' + t('debug.running') + ']', 'system');
    addOutput('[System] Connecting to ' + agent?.host + '...', 'system');

    // Build SSE URL
    const params = new URLSearchParams({ tracer: tracerName });
    if (param) params.set('param', param);
    const sseUrl = `/api/agents/${agentId}/debug/watch?${params.toString()}`;
    const controller = new AbortController();
    watchAbortRef.current = controller;

    try {
      const response = await fetch(sseUrl, {
        method: 'POST',
        headers: authHeaders(),
        signal: controller.signal,
      });

      if (response.status === 401) {
        window.location.href = '/login';
        return;
      }
      if (!response.ok || !response.body) {
        throw new Error(response.statusText || `HTTP ${response.status}`);
      }

      setWsConnected(true);
      addOutput('[System] ' + t('debug.connected'), 'system');

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';
      let eventData = '';

      const flushEvent = () => {
        if (!eventData || statusRef.current === 'stopped') return;
        addOutput(eventData, 'data');
        eventData = '';
      };

      while (true) {
        const { value, done } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        let newlineIndex = buffer.indexOf('\n');
        while (newlineIndex >= 0) {
          const line = buffer.slice(0, newlineIndex).replace(/\r$/, '');
          buffer = buffer.slice(newlineIndex + 1);

          if (line === '') {
            flushEvent();
          } else if (line.startsWith('data:')) {
            if (eventData) eventData += '\n';
            eventData += line.slice(5).trimStart();
          }

          newlineIndex = buffer.indexOf('\n');
        }
      }

      flushEvent();
    } catch (err) {
      if (!(err instanceof DOMException && err.name === 'AbortError')) {
        addOutput(
          '[System] Connection lost. Check server logs for details.',
          'error'
        );
      }
    } finally {
      if (watchAbortRef.current === controller) {
        watchAbortRef.current = null;
        setWsConnected(false);
        setStatus('stopped');
      }
    }
  };

  const handleStop = async () => {
    setStatus('stopped');
    setWsConnected(false);

    // Send stop to agent first, then close SSE
    try {
      await agentsApi.stopDebug(agentId, tracerName);
      addOutput('[System] ' + t('debug.stopped'), 'system');
    } catch (err: any) {
      addOutput('[System] Stop failed: ' + err.message, 'error');
    }
    cleanup();
  };

  const handleClear = () => {
    setOutput([]);
    lineId.current = 0;
  };

  // Cleanup on unmount
  useEffect(() => {
    return () => cleanup();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="w-6 h-6 border-2 border-[var(--brand)] border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  if (!agent) {
    return (
      <div className="text-center py-20">
        <p style={{ color: 'var(--text-muted)' }}>Agent not found</p>
        <Link to="/agents" className="text-[var(--accent)] text-sm mt-2 inline-block hover:underline">
          Back to agents
        </Link>
      </div>
    );
  }

  const getStatusColor = () => {
    switch (status) {
      case 'running': return 'var(--green)';
      case 'stopped': return 'var(--red)';
      default: return 'var(--text-muted)';
    }
  };

  const getStatusText = () => {
    switch (status) {
      case 'running': return 'Running';
      case 'stopped': return 'Stopped';
      default: return 'Idle';
    }
  };

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center gap-4">
        <button
          onClick={() => navigate('/agents/' + agentId)}
          className="w-9 h-9 rounded-[6px] border flex items-center justify-center transition-colors"
          style={{ background: 'var(--bg-card)', borderColor: 'var(--border)', color: 'var(--text-secondary)' }}
        >
          <ArrowLeft size={16} />
        </button>
        <div className="flex-1">
          <h1 className="text-[20px] font-bold flex items-center gap-2.5" style={{ color: 'var(--text)' }}>
            <Terminal size={20} style={{ color: 'var(--brand)' }} />
            {tracerName || t('debug.title')}
            <span className="text-[14px] font-normal" style={{ color: 'var(--text-muted)' }}>
              ({agent.name})
            </span>
          </h1>
          <p className="text-[13px] mt-0.5" style={{ color: 'var(--text-secondary)' }}>
            {tracerDesc || t('debug.subtitle')}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span className="flex items-center gap-1.5 text-[11px] font-semibold" style={{ color: getStatusColor() }}>
            <span className="w-2 h-2 rounded-full" style={{ background: getStatusColor(), boxShadow: `0 0 6px ${getStatusColor()}40` }} />
            {getStatusText()}
          </span>
          <span className="flex items-center gap-1.5 text-[11px]" style={{ color: wsConnected ? 'var(--green)' : 'var(--text-muted)' }}>
            <span className="w-2 h-2 rounded-full" style={{ background: wsConnected ? 'var(--green)' : 'var(--text-muted)' }} />
            {wsConnected ? t('debug.connected') : t('debug.disconnected')}
          </span>
        </div>
      </div>

      {/* Control Panel */}
      <div
        className="border rounded-[14px] p-4"
        style={{ background: 'var(--bg-card)', borderColor: 'var(--border)' }}
      >
        <div className="flex items-end gap-4">
          {/* Parameter Input */}
          <div className="flex-1">
            <label className="block text-[12px] font-semibold mb-1.5" style={{ color: 'var(--text-secondary)' }}>
              {t('debug.argument')}
            </label>
            <input
              value={param}
              onChange={(e) => setParam(e.target.value)}
              placeholder={t('debug.argumentPlaceholder')}
              className="w-full px-3 py-2.5 rounded-[6px] text-[13px] font-mono"
              style={{
                background: 'var(--bg-elevated)',
                border: '1px solid var(--border)',
                color: 'var(--text)',
                outline: 'none',
              }}
              disabled={status === 'running'}
            />
          </div>

          {/* Action Buttons */}
          <div className="flex gap-2">
            {status === 'idle' || status === 'stopped' ? (
              <button
                onClick={handleRun}
                className="flex items-center gap-2 px-4 py-2.5 rounded-[6px] text-[13px] font-semibold text-white"
                style={{
                  background: 'linear-gradient(135deg, var(--green), #059669)',
                  boxShadow: '0 2px 10px rgba(0,214,143,.3)',
                }}
              >
                <Play size={14} />
                Run
              </button>
            ) : null}

            {status === 'running' ? (
              <button
                onClick={handleStop}
                className="flex items-center gap-2 px-4 py-2.5 rounded-[6px] text-[13px] font-semibold text-white"
                style={{
                  background: 'linear-gradient(135deg, var(--red), #DC2626)',
                  boxShadow: '0 2px 10px rgba(255,77,106,.3)',
                }}
              >
                <Square size={14} />
                Stop
              </button>
            ) : null}

            <button
              onClick={handleClear}
              className="flex items-center gap-2 px-3 py-2.5 rounded-[6px] text-[13px] font-semibold border"
              style={{
                background: 'var(--bg-card)',
                borderColor: 'var(--border)',
                color: 'var(--text-secondary)',
              }}
            >
              <Trash2 size={14} />
              Clear
            </button>
          </div>
        </div>
      </div>

      {/* Output Console */}
      <div
        className="border rounded-[14px] overflow-hidden"
        style={{ background: 'var(--bg-card)', borderColor: 'var(--border)' }}
      >
        <div className="px-4 py-3 border-b flex items-center justify-between" style={{ borderColor: 'var(--border)' }}>
          <h3 className="text-[13px] font-semibold" style={{ color: 'var(--text)' }}>
            {t('debug.output')}
          </h3>
          <span className="text-[11px] font-mono" style={{ color: 'var(--text-muted)' }}>
            {output.length} lines
          </span>
        </div>

        <div
          ref={outputRef}
          className="p-4 overflow-y-auto font-mono text-[12px] leading-5"
          style={{
            height: '400px',
            background: 'var(--bg-surface)',
            color: 'var(--text-secondary)',
          }}
        >
          {output.length === 0 ? (
            <div className="text-center py-8" style={{ color: 'var(--text-muted)' }}>
              {t('debug.noOutput')}
            </div>
          ) : (
            output.map((line) => (
              <div key={line.id} className="flex gap-3 py-0.5">
                <span className="text-[10px] shrink-0 w-16 text-right" style={{ color: 'var(--text-muted)' }}>
                  {line.time}
                </span>
                <span
                  className="flex-1 whitespace-pre-wrap break-all"
                  style={{
                    color: line.type === 'error' ? 'var(--red)' :
                           line.type === 'system' ? 'var(--brand-light)' :
                           line.type === 'data' ? 'var(--green)' :
                           'var(--text-secondary)',
                  }}
                >
                  {line.text}
                </span>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
