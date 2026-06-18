interface StatusBadgeProps {
  status: string;
  variant?: 'agent' | 'sink';
}

const agentConfig: Record<string, { dot: string; glow: string; label: string }> = {
  online: { dot: '#00D68F', glow: '0 0 8px rgba(0,214,143,.5)', label: 'Online' },
  degraded: { dot: '#FFB800', glow: '0 0 8px rgba(255,184,0,.4)', label: 'Degraded' },
  offline: { dot: '#FF4D6A', glow: '0 0 8px rgba(255,77,106,.4)', label: 'Offline' },
};

const sinkConfig: Record<string, { dot: string; glow: string; label: string }> = {
  healthy: { dot: '#00D68F', glow: '0 0 8px rgba(0,214,143,.5)', label: 'Healthy' },
  degraded: { dot: '#FFB800', glow: '0 0 8px rgba(255,184,0,.4)', label: 'Degraded' },
  error: { dot: '#FF4D6A', glow: '0 0 8px rgba(255,77,106,.4)', label: 'Error' },
};

export default function StatusBadge({ status, variant = 'agent' }: StatusBadgeProps) {
  const config = (variant === 'sink' ? sinkConfig : agentConfig)[status];
  const dotColor = config?.dot ?? '#6B7280';
  const glow = config?.glow ?? 'none';
  const label = config?.label ?? status;

  return (
    <span className="inline-flex items-center gap-1.5 text-xs font-medium" style={{ color: 'var(--text-secondary)' }}>
      <span
        className="w-2 h-2 rounded-full"
        style={{ background: dotColor, boxShadow: glow }}
      />
      {label}
    </span>
  );
}
