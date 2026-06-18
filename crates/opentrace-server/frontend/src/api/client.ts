export class ApiError extends Error {
  status: number;
  body: unknown;
  constructor(status: number, message: string, body?: unknown) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.body = body;
  }
}

// ---- Models ----
export interface User {
  id: number;
  username: string;
  role: string;
  created_at: string;
}

export interface Tracer {
  name: string;
  description: string;
}

export interface Agent {
  id: number;
  name: string;
  host: string;
  group_id?: number;
  group_name?: string;
  status: string;
  tags?: string;
  version?: string;
  tracers?: Tracer[];
  token?: string;
  os?: string;
  arch?: string;
  cpu?: number;
  memory?: number;
  rate?: number;
  uptime?: number;
  created_at: string;
}

export interface Group {
  id: number;
  name: string;
  description?: string;
  created_at: string;
}

export interface Sink {
  id: number;
  name: string;
  sink_type: string;
  config: string;
  status: string;
  agent_count?: number;
  events_sent?: number;
  events_per_sec?: number;
  delivery_rate?: number;
  created_at: string;
}

export interface Tracepoint {
  id: number;
  agent_id: number;
  name: string;
  description?: string;
  enabled: boolean;
  sink_id?: number | null;
  events_sent?: number;   // TODO: backend API 待添加
  events_failed?: number; // TODO: backend API 待添加
  created_at: string;
}

export interface PaginatedTracepoints {
  items: Tracepoint[];
  total: number;
  page: number;
  page_size: number;
}

export interface Stats {
  total_agents: number;
  online_agents: number;
  total_sinks: number;
  healthy_sinks: number;
}

// ---- Token management ----
const TOKEN_KEY = 'token';

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token);
}

export function removeToken(): void {
  localStorage.removeItem(TOKEN_KEY);
}

export function authHeaders(): Record<string, string> {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

// ---- Core fetch wrapper ----
export async function apiCall<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const token = getToken();
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...((options.headers as Record<string, string>) || {}),
  };
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  const res = await fetch(path, { ...options, headers, cache: 'no-store' });

  if (res.status === 401) {
    removeToken();
    localStorage.removeItem('user');
    window.location.href = '/login';
    throw new ApiError(401, 'Unauthorized');
  }

  if (!res.ok) {
    let body: unknown;
    try {
      body = await res.json();
    } catch {
      body = undefined;
    }
    const message =
      (body as { error?: string } | undefined)?.error ?? res.statusText;
    throw new ApiError(res.status, message, body);
  }

  if (res.status === 204) return undefined as T;

  return res.json() as Promise<T>;
}

// ---- Auth ----
export const authApi = {
  login: (username: string, password: string) =>
    apiCall<{ token: string; user: User }>('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    }),
  me: () => apiCall<User>('/api/auth/me'),
};

// ---- Users ----
export const usersApi = {
  list: () => apiCall<User[]>('/api/users'),
  create: (data: { username: string; password: string; role?: string }) =>
    apiCall<User>('/api/users', {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  remove: (id: number) =>
    apiCall<{ success: boolean }>(`/api/users/${id}`, { method: 'DELETE' }),
};

// ---- Agents ----
export const agentsApi = {
  list: (params?: { group?: string; tag?: string }) => {
    const qs = new URLSearchParams();
    if (params?.group) qs.set('group', params.group);
    if (params?.tag) qs.set('tag', params.tag);
    const query = qs.toString();
    return apiCall<Agent[]>(`/api/agents${query ? `?${query}` : ''}`);
  },
  get: (id: number) => apiCall<Agent>(`/api/agents/${id}`),
  create: (data: { name: string; host: string; group_id?: number; tags?: string[]; token?: string; sink_ids?: number[] }) =>
    apiCall<Agent>('/api/agents', {
      method: 'POST',
      body: JSON.stringify({ ...data, tags: data.tags?.join(',') }),
    }),
  update: (id: number, data: { name?: string; host?: string; group_id?: number; tags?: string[]; token?: string }) =>
    apiCall<Agent>(`/api/agents/${id}`, {
      method: 'PUT',
      body: JSON.stringify({ ...data, tags: data.tags?.join(',') }),
    }),
  remove: (id: number) =>
    apiCall<{ success: boolean }>(`/api/agents/${id}`, { method: 'DELETE' }),
  sync: (id: number) =>
    apiCall<Agent>(`/api/agents/${id}/sync`, { method: 'POST' }),
  listTracepoints: (agentId: number, params?: { page?: number; page_size?: number }) => {
    const qs = new URLSearchParams();
    if (params?.page) qs.set('page', String(params.page));
    if (params?.page_size) qs.set('page_size', String(params.page_size));
    const query = qs.toString();
    return apiCall<PaginatedTracepoints>(`/api/agents/${agentId}/tracepoints${query ? `?${query}` : ''}`);
  },
  createTracepoint: (
    agentId: number,
    data: { name: string; description?: string; enabled?: boolean }
  ) =>
    apiCall<Tracepoint>(`/api/agents/${agentId}/tracepoints`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  updateTracepoint: (
    agentId: number,
    tracepointId: number,
    data: { enabled?: boolean; sink_id?: number | null }
  ) =>
    apiCall<Tracepoint>(
      `/api/agents/${agentId}/tracepoints/${tracepointId}`,
      { method: 'PUT', body: JSON.stringify(data) }
    ),
  startTracer: (agentId: number, tracerName: string) =>
    apiCall<{ success: boolean }>(
      `/api/agents/${agentId}/tracer/${encodeURIComponent(tracerName)}/start`,
      { method: 'POST' }
    ),
  stopTracer: (agentId: number, tracerName: string) =>
    apiCall<{ success: boolean }>(
      `/api/agents/${agentId}/tracer/${encodeURIComponent(tracerName)}/stop`,
      { method: 'POST' }
    ),
  removeTracepoint: (agentId: number, tracepointId: number) =>
    apiCall<{ success: boolean }>(
      `/api/agents/${agentId}/tracepoints/${tracepointId}`,
      { method: 'DELETE' }
    ),
  stopDebug: (agentId: number, tracerName: string) => {
    const qs = new URLSearchParams({ tracer: tracerName });
    return apiCall<{ success: boolean }>(
      `/api/agents/${agentId}/debug/stop?${qs.toString()}`,
      { method: 'POST' }
    );
  },
  getSinkNames: (id: number) => apiCall<string[]>(`/api/agents/${id}/sink-names`),
};

// ---- Groups ----
export const groupsApi = {
  list: () => apiCall<Group[]>('/api/groups'),
  create: (data: { name: string; description?: string }) =>
    apiCall<Group>('/api/groups', {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  remove: (id: number) =>
    apiCall<{ success: boolean }>(`/api/groups/${id}`, { method: 'DELETE' }),
};

// ---- Sinks ----
export const sinksApi = {
  list: () => apiCall<Sink[]>('/api/sinks'),
  get: (id: number) => apiCall<Sink>(`/api/sinks/${id}`),
  create: (data: { name: string; sink_type: string; config: string }) =>
    apiCall<Sink>('/api/sinks', {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  update: (id: number, data: { name?: string; sink_type?: string; config?: string }) =>
    apiCall<Sink>(`/api/sinks/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    }),
  remove: (id: number) =>
    apiCall<{ success: boolean }>(`/api/sinks/${id}`, { method: 'DELETE' }),
  bindAgent: (sinkId: number, agentId: number) =>
    apiCall<{ success: boolean; error?: string }>(`/api/sinks/${sinkId}/bind`, {
      method: 'POST',
      body: JSON.stringify({ agent_id: agentId }),
    }),
  unbindAgent: (sinkId: number, agentId: number) =>
    apiCall<{ success: boolean }>(`/api/sinks/${sinkId}/bind/${agentId}`, {
      method: 'DELETE',
    }),
  getAgents: (sinkId: number) =>
    apiCall<number[]>(`/api/sinks/${sinkId}/agents`),
  connectAgent: (sinkId: number, agentId: number) =>
    apiCall<{ success: boolean }>(`/api/sinks/${sinkId}/agents/${agentId}/connect`, {
      method: 'POST',
    }),
  disconnectAgent: (sinkId: number, agentId: number) =>
    apiCall<{ success: boolean }>(`/api/sinks/${sinkId}/agents/${agentId}/disconnect`, {
      method: 'POST',
    }),
  testConnectivity: (sinkId: number) =>
    apiCall<{ success: boolean }>(`/api/sinks/${sinkId}/test`, { method: 'POST' }),
};

/** Parse tags from backend (comma-separated string) into array */
export function parseTags(tags?: string | string[]): string[] {
  if (!tags) return [];
  if (Array.isArray(tags)) return tags;
  return tags.split(',').map((t) => t.trim()).filter(Boolean);
}

// ---- Stats ----
export const statsApi = {
  get: () => apiCall<Stats>('/api/stats'),
};
