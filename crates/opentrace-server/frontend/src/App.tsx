import { Routes, Route, Navigate } from 'react-router-dom';
import Layout from '@/components/Layout';
import { ToastProvider } from '@/components/Toast';
import './i18n';

import Login from '@/pages/Login';
import Dashboard from '@/pages/Dashboard';
import Agents from '@/pages/Agents';
import AgentDetail from '@/pages/AgentDetail';
import AgentEdit from '@/pages/AgentEdit';
import Sinks from '@/pages/Sinks';
import SinkDetail from '@/pages/SinkDetail';
import Users from '@/pages/Users';
import Profile from '@/pages/Profile';
import Debug from '@/pages/Debug';
import SystemConfig from '@/pages/SystemConfig';
import { useAuth } from '@/hooks/useAuth';
import { PageTitleProvider } from '@/components/PageTitleContext';

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth();
  if (loading) {
    return (
      <div
        className="min-h-screen flex items-center justify-center"
        style={{ background: 'var(--bg)' }}
      >
        <div
          className="w-8 h-8 border-2 rounded-full animate-spin"
          style={{ borderColor: 'var(--brand)', borderTopColor: 'transparent' }}
        />
      </div>
    );
  }
  if (!user) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

export default function App() {
  return (
    <ToastProvider>
      <PageTitleProvider>
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route
          path="/"
          element={
            <ProtectedRoute>
              <Layout />
            </ProtectedRoute>
          }
        >
          <Route index element={<Dashboard />} />
          <Route path="agents" element={<Agents />} />
          <Route path="agents/:id" element={<AgentDetail />} />
          <Route path="agents/:id/edit" element={<AgentEdit />} />
          <Route path="sinks" element={<Sinks />} />
          <Route path="sinks/:id" element={<SinkDetail />} />
          <Route path="users" element={<Users />} />
          <Route path="profile" element={<Profile />} />
          <Route path="system-config" element={<SystemConfig />} />
          <Route path="agents/:id/debug" element={<Debug />} />
        </Route>
      </Routes>
      </PageTitleProvider>
    </ToastProvider>
  );
}
