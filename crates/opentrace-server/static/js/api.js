const API_BASE = '/api';

/* ═══ API 调用基础函数 ═══ */
async function apiCall(path, options = {}) {
    const token = localStorage.getItem('token');
    const headers = {
        'Content-Type': 'application/json',
        ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
        ...options.headers,
    };

    try {
        const response = await fetch(`${API_BASE}${path}`, {
            ...options,
            headers,
        });

        if (response.status === 401) {
            localStorage.removeItem('token');
            localStorage.removeItem('user');
            window.location.href = '/login.html';
            return;
        }

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.error || `HTTP ${response.status}`);
        }

        return data;
    } catch (error) {
        if (error.name === 'TypeError' && error.message.includes('fetch')) {
            throw new Error('网络连接失败，请检查网络');
        }
        throw error;
    }
}

/* ═══ 按钮加载状态管理 ═══ */
function setButtonLoading(button, loading, loadingText) {
    if (!button) return;
    
    if (loading) {
        button.dataset.originalText = button.textContent;
        button.textContent = loadingText || '处理中...';
        button.disabled = true;
    } else {
        button.textContent = button.dataset.originalText || button.textContent;
        button.disabled = false;
        delete button.dataset.originalText;
    }
}

const api = {
    login: (username, password) => apiCall('/auth/login', {
        method: 'POST',
        body: JSON.stringify({ username, password }),
    }),
    me: () => apiCall('/auth/me'),

    listUsers: () => apiCall('/users'),
    createUser: (data) => apiCall('/users', {
        method: 'POST',
        body: JSON.stringify(data),
    }),
    deleteUser: (id) => apiCall(`/users/${id}`, { method: 'DELETE' }),

    listAgents: (params = {}) => {
        const query = new URLSearchParams(params).toString();
        return apiCall(`/agents${query ? `?${query}` : ''}`);
    },
    getAgent: (id) => apiCall(`/agents/${id}`),
    createAgent: (data) => apiCall('/agents', {
        method: 'POST',
        body: JSON.stringify(data),
    }),
    updateAgent: (id, data) => apiCall(`/agents/${id}`, {
        method: 'PUT',
        body: JSON.stringify(data),
    }),
    deleteAgent: (id) => apiCall(`/agents/${id}`, { method: 'DELETE' }),

    listGroups: () => apiCall('/groups'),
    createGroup: (data) => apiCall('/groups', {
        method: 'POST',
        body: JSON.stringify(data),
    }),
    deleteGroup: (id) => apiCall(`/groups/${id}`, { method: 'DELETE' }),

    listSinks: () => apiCall('/sinks'),
    getSink: (id) => apiCall(`/sinks/${id}`),
    createSink: (data) => apiCall('/sinks', {
        method: 'POST',
        body: JSON.stringify(data),
    }),
    updateSink: (id, data) => apiCall(`/sinks/${id}`, {
        method: 'PUT',
        body: JSON.stringify(data),
    }),
    deleteSink: (id) => apiCall(`/sinks/${id}`, { method: 'DELETE' }),
    bindAgent: (sinkId, agentId) => apiCall(`/sinks/${sinkId}/bind`, {
        method: 'POST',
        body: JSON.stringify({ agent_id: agentId }),
    }),
    unbindAgent: (sinkId, agentId) => apiCall(`/sinks/${sinkId}/bind/${agentId}`, {
        method: 'DELETE',
    }),
    getSinkAgents: (sinkId) => apiCall(`/sinks/${sinkId}/agents`),

    getStats: () => apiCall('/stats'),
};
