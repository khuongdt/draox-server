import { PageContainer, ProDescriptions } from '@ant-design/pro-components';
import { Card, Row, Col, Button, Space, Badge, message, Spin, Result } from 'antd';
import { useParams } from '@umijs/max';
import { useState, useEffect } from 'react';
import DarkStatisticCard from '@/components/DarkStatisticCard';
import ConfirmActionModal from '@/components/ConfirmActionModal';
import { getSession, getSessionMetrics, destroySession, drainSession } from '@/services/sessions';

type ModalAction = 'destroy' | 'drain' | null;

const formatDuration = (secs: number): string => {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
};

const formatBytes = (bytes: number): string => {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(2)} GB`;
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(2)} MB`;
  if (bytes >= 1_024) return `${(bytes / 1_024).toFixed(1)} KB`;
  return `${bytes} B`;
};

export default function SessionDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [session, setSession] = useState<API.Session | null>(null);
  const [metrics, setMetrics] = useState<API.SessionMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [modalAction, setModalAction] = useState<ModalAction>(null);

  useEffect(() => {
    if (!id) return;
    Promise.all([getSession(id), getSessionMetrics(id)])
      .then(([s, m]) => { setSession(s); setMetrics(m); })
      .catch(() => message.error('Failed to load session'))
      .finally(() => setLoading(false));
  }, [id]);

  const handleAction = async () => {
    if (!id || !modalAction) return;
    if (modalAction === 'destroy') {
      await destroySession(id);
    } else {
      await drainSession(id);
    }
    setModalAction(null);
    message.success(modalAction === 'destroy' ? 'Session destroyed' : 'Session draining');
  };

  if (loading) return <Spin style={{ display: 'block', margin: '80px auto' }} />;
  if (!session) return <Result status="404" title="Session not found" />;

  return (
    <PageContainer title={`Session: ${session.id.slice(0, 20)}…`} subTitle="Session details">
      {/* Stats row */}
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={24} sm={8}>
          <DarkStatisticCard
            title="Active Connections"
            value={metrics?.connection_count ?? session.connections.length}
            color="#ff8c42"
          />
        </Col>
        <Col xs={24} sm={8}>
          <DarkStatisticCard
            title="Duration"
            value={metrics ? formatDuration(metrics.duration_secs) : '—'}
            color="#53c28b"
          />
        </Col>
        <Col xs={24} sm={8}>
          <DarkStatisticCard
            title="Bytes Transferred"
            value={metrics ? formatBytes(metrics.bytes_sent + metrics.bytes_received) : '—'}
            color="#a0a0b0"
          />
        </Col>
      </Row>

      {/* Session info */}
      <Card
        style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
        bodyStyle={{ padding: 24 }}
      >
        <ProDescriptions
          column={2}
          labelStyle={{ color: '#a0a0b0' }}
          contentStyle={{ color: '#e0e0e0' }}
        >
          <ProDescriptions.Item label="Session ID">
            <span style={{ fontFamily: 'monospace', fontSize: 12 }}>{session.id}</span>
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Client ID">{session.client_id}</ProDescriptions.Item>
          <ProDescriptions.Item label="State">
            <Badge status="success" text={<span style={{ color: '#e0e0e0' }}>{session.state}</span>} />
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Connections">
            {session.connections.length}
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Created At">
            {new Date(session.created_at).toLocaleString()}
          </ProDescriptions.Item>
        </ProDescriptions>
      </Card>

      {/* Actions */}
      <Card
        title={<span style={{ color: '#e0e0e0' }}>Session Actions</span>}
        style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
        headStyle={{ borderBottom: '1px solid #2a2a4a' }}
      >
        <Space>
          <Button
            style={{ color: '#f5a623', borderColor: '#f5a623' }}
            onClick={() => setModalAction('drain')}
          >
            Drain Session
          </Button>
          <Button danger type="primary" onClick={() => setModalAction('destroy')}>
            Destroy Session
          </Button>
        </Space>
      </Card>

      <ConfirmActionModal
        visible={modalAction !== null}
        title={modalAction === 'destroy' ? 'Destroy Session' : 'Drain Session'}
        description={
          modalAction === 'destroy'
            ? 'All connections in this session will be immediately closed. This cannot be undone.'
            : 'New connections will be rejected. Existing connections can finish gracefully.'
        }
        type={modalAction === 'destroy' ? 'danger' : 'warning'}
        confirmText={modalAction === 'destroy' ? 'Destroy' : 'Drain'}
        onConfirm={handleAction}
        onCancel={() => setModalAction(null)}
      />
    </PageContainer>
  );
}
