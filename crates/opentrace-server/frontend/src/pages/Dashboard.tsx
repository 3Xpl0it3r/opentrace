import { useState, useEffect, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { statsApi, agentsApi, sinksApi, type Stats, type Agent, type Sink } from '@/api/client';
import { t } from '@/i18n';
import { usePageTitle } from '@/components/PageTitleContext';


function formatUptime(seconds?: number): string {
  if (!seconds) return '--';
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  if (d > 0) return `${d}d ${h}h`;
  return `${h}h`;
}

function formatRate(rate?: number): string {
  if (!rate) return '--';
  if (rate >= 1000) return `${(rate / 1000).toFixed(1)}K`;
  return `${rate.toFixed(0)}`;
}

/* ---- icon components (inline SVGs matching reference) ---- */
const ServerIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect x="2" y="6" width="20" height="12" rx="2" /><line x1="6" y1="10" x2="6" y2="14" /><line x1="10" y1="10" x2="10" y2="14" />
  </svg>
);
const CheckCircleIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" /><polyline points="22 4 12 14.01 9 11.01" />
  </svg>
);
const LayersIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 2L2 7l10 5 10-5-10-5z" /><path d="M2 17l10 5 10-5" /><path d="M2 12l10 5 10-5" />
  </svg>
);
const ActivityIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
  </svg>
);

/* ---- mini chart bar generator ---- */
function BarChart() {
  const bars = useMemo(() => {
    const arr: { h: number; o: number }[] = [];
    for (let i = 0; i < 48; i++) {
      arr.push({ h: 20 + Math.random() * 80, o: 0.4 + Math.random() * 0.6 });
    }
    return arr;
  }, []);

  return (
    <div style={{ height: 200, display: 'flex', alignItems: 'flex-end', gap: 3, padding: '16px 16px 0' }}>
      {bars.map((b, i) => (
        <div
          key={i}
          style={{
            flex: 1,
            borderRadius: '3px 3px 0 0',
            minHeight: 4,
            height: `${b.h}%`,
            background: 'linear-gradient(to top, var(--brand), var(--brand-light))',
            opacity: b.o,
            transition: 'opacity .2s',
          }}
        />
      ))}
    </div>
  );
}

/* ---- donut chart for sink distribution ---- */
function DonutChart({ sinks }: { sinks: Sink[] }) {
  const total = sinks.length || 1;
  const counts = useMemo(() => {
    const c = { kafka: 0, prometheus: 0, cache: 0 } as Record<string, number>;
    sinks.forEach((s) => {
      const t = s.sink_type.toLowerCase();
      if (t.includes('kafka')) c.kafka++;
      else if (t.includes('prom')) c.prometheus++;
      else c.cache++;
    });
    return c;
  }, [sinks]);

  const segments = [
    { key: 'kafka', label: 'Kafka', count: counts.kafka, color: 'var(--yellow)' },
    { key: 'prometheus', label: 'Prometheus', count: counts.prometheus, color: 'var(--accent)' },
    { key: 'cache', label: 'Cache', count: counts.cache, color: 'var(--brand-light)' },
  ];

  let offset = 0;
  const r = 15.915;


  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 16, paddingTop: 30 }}>
      <div style={{ position: 'relative', width: 140, height: 140 }}>
        <svg viewBox="0 0 36 36" style={{ width: 140, height: 140, transform: 'rotate(-90deg)' }}>
          <circle cx="18" cy="18" r={r} fill="none" stroke="var(--bg-elevated)" strokeWidth="3" />
          {segments.map((seg) => {
            const pct = seg.count / total;
            const dash = pct * 100;
            const el = (
              <circle
                key={seg.key}
                cx="18"
                cy="18"
                r={r}
                fill="none"
                stroke={seg.color}
                strokeWidth="3"
                strokeDasharray={`${dash} ${100 - dash}`}
                strokeDashoffset={-offset}
              />
            );
            offset += dash;
            return el;
          })}
        </svg>
        <div style={{ position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}>
          <span style={{ fontSize: 24, fontWeight: 700 }}>{total}</span>
          <span style={{ fontSize: 10, color: 'var(--text-muted)' }}>Sinks</span>
        </div>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8, width: '100%' }}>
        {segments.map((seg) => (
          <div key={seg.key} style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12 }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <span style={{ width: 8, height: 8, borderRadius: 2, background: seg.color }} />
              {seg.label}
            </span>
            <span style={{ color: 'var(--text-muted)' }}>
              {seg.count} ({total > 0 ? Math.round((seg.count / total) * 100) : 0}%)
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

/* ---- styles as JS objects to match reference ---- */
const s = {
  pageHeader: { display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: 24 } as React.CSSProperties,
  h1: { fontSize: 22, fontWeight: 700, letterSpacing: -0.3 } as React.CSSProperties,
  subtitle: { fontSize: 13, color: 'var(--text-secondary)', marginTop: 4 } as React.CSSProperties,
  statsGrid: { display: 'grid', gridTemplateColumns: 'repeat(4,1fr)', gap: 16, marginBottom: 24 } as React.CSSProperties,
  statCard: (_color: string): React.CSSProperties => ({
    background: 'var(--bg-card)',
    border: '1px solid var(--border)',
    borderRadius: 14,
    padding: 20,
    position: 'relative',
    overflow: 'hidden',
    borderTop: 'none',
  }),
  statCardBefore: (color: string): React.CSSProperties => ({
    position: 'absolute', top: 0, left: 0, right: 0, height: 2,
    background: {
      purple: 'linear-gradient(90deg, var(--brand), var(--brand-light))',
      green: 'linear-gradient(90deg, var(--green), #34d399)',
      blue: 'linear-gradient(90deg, var(--blue), #60a5fa)',
      yellow: 'linear-gradient(90deg, var(--yellow), #fcd34d)',
    }[color],
  }),
  statIcon: (color: string): React.CSSProperties => ({
    width: 40, height: 40, borderRadius: 10,
    display: 'flex', alignItems: 'center', justifyContent: 'center',
    marginBottom: 14,
    background: {
      purple: 'rgba(108,92,231,.12)',
      green: 'rgba(0,214,143,.12)',
      blue: 'rgba(59,130,246,.12)',
      yellow: 'rgba(255,184,0,.12)',
    }[color],
    color: {
      purple: 'var(--brand-light)',
      green: 'var(--green)',
      blue: 'var(--blue)',
      yellow: 'var(--yellow)',
    }[color],
  }),
  statValue: { fontSize: 28, fontWeight: 700, letterSpacing: -1, lineHeight: 1, marginBottom: 4, fontFamily: "'JetBrains Mono', monospace" } as React.CSSProperties,
  statLabel: { fontSize: 12, color: 'var(--text-secondary)', fontWeight: 500 } as React.CSSProperties,
  statChange: (type: 'up' | 'down'): React.CSSProperties => ({
    display: 'inline-flex', alignItems: 'center', gap: 3, fontSize: 11, fontWeight: 600,
    marginTop: 8, padding: '2px 8px', borderRadius: 10,
    background: type === 'up' ? 'rgba(0,214,143,.12)' : 'rgba(255,77,106,.12)',
    color: type === 'up' ? 'var(--green)' : 'var(--red)',
  }),
  panel: { background: 'var(--bg-card)', border: '1px solid var(--border)', borderRadius: 14, overflow: 'hidden' } as React.CSSProperties,
  panelHeader: { display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '16px 20px', borderBottom: '1px solid var(--border)' } as React.CSSProperties,
  panelTitle: { fontSize: 14, fontWeight: 600 } as React.CSSProperties,
  panelBody: (noPad?: boolean): React.CSSProperties => ({ padding: noPad ? 0 : 20 }),
  dashGrid: { display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 16, marginBottom: 16 } as React.CSSProperties,
  dashGridEqual: { display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16, marginBottom: 16 } as React.CSSProperties,
  table: { width: '100%', borderCollapse: 'collapse' } as React.CSSProperties,
  th: { textAlign: 'left' as const, padding: '10px 16px', fontSize: 11, fontWeight: 600, textTransform: 'uppercase' as const, letterSpacing: 0.8, color: 'var(--text-muted)', borderBottom: '1px solid var(--border)', background: 'var(--bg-elevated)' } as React.CSSProperties,
  td: { padding: '12px 16px', fontSize: 13, borderBottom: '1px solid var(--border)', color: 'var(--text-secondary)' } as React.CSSProperties,
  statusDot: (status: string): React.CSSProperties => ({
    width: 8, height: 8, borderRadius: '50%', display: 'inline-block', marginRight: 6,
    background: status === 'online' ? 'var(--green)' : status === 'offline' ? 'var(--red)' : 'var(--yellow)',
    boxShadow: status === 'online' ? '0 0 8px rgba(0,214,143,.5)' : status === 'offline' ? '0 0 8px rgba(255,77,106,.4)' : '0 0 8px rgba(255,184,0,.4)',
  }),
  badge: (status: string): React.CSSProperties => {
    const map: Record<string, { bg: string; color: string }> = {
      online: { bg: 'rgba(0,214,143,.12)', color: 'var(--green)' },
      offline: { bg: 'rgba(255,77,106,.12)', color: 'var(--red)' },
      degraded: { bg: 'rgba(255,184,0,.12)', color: 'var(--yellow)' },
    };
    const c = map[status] || map.online;
    return { display: 'inline-flex', alignItems: 'center', gap: 4, padding: '3px 10px', borderRadius: 20, fontSize: 11, fontWeight: 600, background: c.bg, color: c.color };
  },
  btnSecondary: { padding: '5px 10px', fontSize: 11, fontWeight: 600, borderRadius: 6, border: '1px solid var(--border)', background: 'var(--bg-card)', color: 'var(--text-secondary)', cursor: 'pointer' } as React.CSSProperties,
  activityList: { listStyle: 'none', padding: 0, margin: 0 } as React.CSSProperties,
  activityItem: { display: 'flex', gap: 12, padding: '12px 0', borderBottom: '1px solid var(--border)' } as React.CSSProperties,
  activityDot: (color: string): React.CSSProperties => ({
    width: 8, height: 8, borderRadius: '50%', marginTop: 5, flexShrink: 0, background: color,
  }),
  activityText: { fontSize: 12, color: 'var(--text-secondary)', lineHeight: 1.5 } as React.CSSProperties,
  activityTime: { fontSize: 10, color: 'var(--text-muted)', marginTop: 2 } as React.CSSProperties,
};

const activities = [
  { color: 'var(--green)', text: <><strong>prod-web-01</strong> connected to <strong>kafka-prod</strong></>, time: '2 min ago' },
  { color: 'var(--brand)', text: <>New agent <strong>prod-web-03</strong> registered</>, time: '15 min ago' },
  { color: 'var(--yellow)', text: <>Sink <strong>prometheus-metrics</strong> updated</>, time: '1h ago' },
  { color: 'var(--red)', text: <><strong>dev-worker-03</strong> went offline</>, time: '3h ago' },
];

export default function Dashboard() {
  
  
  const { setPageTitle } = usePageTitle();
  const [stats, setStats] = useState<Stats | null>(null);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [sinks, setSinks] = useState<Sink[]>([]);

  useEffect(() => {
    setPageTitle(t('dashboard.title'), t('dashboard.subtitle'));
    return () => setPageTitle('');
  }, [setPageTitle]);

  useEffect(() => {
    statsApi.get().then(setStats).catch(() => {});
    agentsApi.list().then(setAgents).catch(() => {});
    sinksApi.list().then(setSinks).catch(() => {});
  }, []);

  const onlinePct = stats ? Math.round((stats.online_agents / (stats.total_agents || 1)) * 100) : 0;

  const statusLabel = (status: string) => {
    if (status === 'online') return 'Online';
    if (status === 'offline') return 'Offline';
    return 'Degraded';
  };

  return (
    <div>
      {/* Stats grid */}
      <div style={s.statsGrid}>
        {[
          { color: 'purple', icon: <ServerIcon />, value: stats?.total_agents ?? '--', label: t('dashboard.totalAgents'), change: '+3 this week', type: 'up' as const },
          { color: 'green', icon: <CheckCircleIcon />, value: stats?.online_agents ?? '--', label: t('dashboard.onlineAgents'), change: `${onlinePct}% uptime`, type: 'up' as const },
          { color: 'blue', icon: <LayersIcon />, value: stats?.total_sinks ?? '--', label: t('dashboard.activeSinks'), change: '+2 new', type: 'up' as const },
          { color: 'yellow', icon: <ActivityIcon />, value: '2.4M', label: t('dashboard.eventsPerHour'), change: '-5% vs yesterday', type: 'down' as const },
        ].map((card) => (
          <div key={card.label} style={{ ...s.statCard(card.color), borderTop: 'none' }}>
            <div style={s.statCardBefore(card.color)} />
            <div style={s.statIcon(card.color)}>
              <div style={{ width: 20, height: 20 }}>{card.icon}</div>
            </div>
            <div style={s.statValue}>{card.value}</div>
            <div style={s.statLabel}>{card.label}</div>
            <span style={s.statChange(card.type)}>{card.change}</span>
          </div>
        ))}
      </div>

      {/* Two-column grid: 2fr 1fr */}
      <div style={s.dashGrid}>
        {/* Throughput Overview */}
        <div style={s.panel}>
          <div style={s.panelHeader}>
            <h3 style={s.panelTitle}>{t('dashboard.throughput')}</h3>
          </div>
          <div style={s.panelBody()}>
            <BarChart />
          </div>
        </div>

        {/* Sink Distribution */}
        <div style={s.panel}>
          <div style={s.panelHeader}>
            <h3 style={s.panelTitle}>{t('dashboard.sinkDistribution')}</h3>
          </div>
          <div style={{ ...s.panelBody(), display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
            <DonutChart sinks={sinks} />
          </div>
        </div>
      </div>

      {/* Two-column equal grid */}
      <div style={s.dashGridEqual}>
        {/* Agent Health */}
        <div style={s.panel}>
          <div style={s.panelHeader}>
            <h3 style={s.panelTitle}>{t('dashboard.agentHealth')}</h3>
            <Link to="/agents" style={s.btnSecondary}>{t('dashboard.viewAll')}</Link>
          </div>
          <div style={s.panelBody(true)}>
            <table style={s.table}>
              <thead>
                <tr>
                  <th style={s.th}>{t('agents.title')}</th>
                  <th style={s.th}>{t('common.status')}</th>
                  <th style={s.th}>{t('agents.uptime')}</th>
                  <th style={s.th}>{t('agents.eventsPerSec')}</th>
                </tr>
              </thead>
              <tbody>
                {agents.slice(0, 5).map((agent) => (
                  <tr key={agent.id} style={{ transition: 'background .15s' }} onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')} onMouseLeave={(e) => (e.currentTarget.style.background = '')}>
                    <td style={s.td}>
                      <strong style={{ color: 'var(--text-primary)' }}>{agent.name}</strong>
                      <br />
                      <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>{agent.host}</span>
                    </td>
                    <td style={s.td}>
                      <span style={s.badge(agent.status)}>
                        <span style={s.statusDot(agent.status)} />
                        {statusLabel(agent.status)}
                      </span>
                    </td>
                    <td style={s.td}>{formatUptime(agent.uptime)}</td>
                    <td style={{ ...s.td, fontFamily: "'JetBrains Mono', monospace", fontWeight: 600, color: agent.status === 'online' ? 'var(--green)' : 'var(--text-muted)' }}>
                      {formatRate(agent.rate)}
                    </td>
                  </tr>
                ))}
                {agents.length === 0 && (
                  <tr><td style={s.td} colSpan={4}>No agents registered</td></tr>
                )}
              </tbody>
            </table>
          </div>
        </div>

        {/* Recent Activity */}
        <div style={s.panel}>
          <div style={s.panelHeader}>
            <h3 style={s.panelTitle}>{t('dashboard.recentActivity')}</h3>
          </div>
          <div style={s.panelBody()}>
            <ul style={s.activityList}>
              {activities.map((item, i) => (
                <li key={i} style={{ ...s.activityItem, ...(i === activities.length - 1 ? { borderBottom: 'none' } : {}) }}>
                  <div style={s.activityDot(item.color)} />
                  <div>
                    <div style={s.activityText}>{item.text}</div>
                    <div style={s.activityTime}>{item.time}</div>
                  </div>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
