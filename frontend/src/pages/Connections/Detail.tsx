import { PageContainer, ProDescriptions } from '@ant-design/pro-components';
import { Card, Button, Tag, Badge, message, Spin, Result } from 'antd';
import { useParams } from '@umijs/max';
import { useState, useEffect } from 'react';
import ConfirmActionModal from '@/components/ConfirmActionModal';
import { getConnection, disconnectConnection } from '@/services/connections';

const PROTOCOL_STYLE: Record<string, { color: string; background: string }> = {
  tcp: { color: '#90caf9', background: '#1a237e' },
  udp: { color: '#a5d6a7', background: '#1b5e20' },
  websocket: { color: '#ef9a9a', background: '#4a1010' },
  http: { color: '#ff8a65', background: '#3e2723' },
};

const formatBytes = (bytes: number): string => {
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(2)} MB`;
  if (bytes >= 1_024) return `${(bytes / 1_024).toFixed(1)} KB`;
  return `${bytes} B`;
};

export default function ConnectionDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [conn, setConn] = useState<API.Connection | null>(null);
  const [loading, setLoading] = useState(true);
  const [modalVisible, setModalVisible] = useState(false);

  useEffect(() => {
    if (!id) return;
    getConnection(id)
      .then(setConn)
      .catch(() => message.error('Failed to load connection'))
      .finally(() => setLoading(false));
  }, [id]);

  const doDisconnect = async () => {
    if (!id) return;
    await disconnectConnection(id);
    setModalVisible(false);
    message.success('Connection terminated');
  };

  if (loading) return <Spin style={{ display: 'block', margin: '80px auto' }} />;
  if (!conn) return <Result status="404" title="Connection not found" />;

  const pStyle = PROTOCOL_STYLE[conn.protocol] ?? {};

  return (
    <PageContainer title={`Connection: ${conn.id.slice(0, 16)}…`} subTitle="Connection details and management">
      <Card
        style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
        bodyStyle={{ padding: 24 }}
      >
        <ProDescriptions
          column={2}
          labelStyle={{ color: '#a0a0b0' }}
          contentStyle={{ color: '#e0e0e0' }}
        >
          <ProDescriptions.Item label="Connection ID">
            <span style={{ fontFamily: 'monospace', fontSize: 12 }}>{conn.id}</span>
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Remote Address">
            {conn.remote_addr}
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Protocol">
            <Tag
              style={{
                color: pStyle.color,
                background: pStyle.background,
                border: `1px solid ${pStyle.color}44`,
                fontWeight: 600,
                textTransform: 'uppercase',
              }}
            >
              {conn.protocol}
            </Tag>
          </ProDescriptions.Item>
          <ProDescriptions.Item label="State">
            <Badge status="success" text={<span style={{ color: '#e0e0e0' }}>{conn.state}</span>} />
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Connected At">
            {new Date(conn.connected_at).toLocaleString()}
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Session ID">
            <span style={{ fontFamily: 'monospace', fontSize: 12 }}>{conn.session_id ?? '—'}</span>
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Bytes Sent">
            <span style={{ color: '#e05d10' }}>{formatBytes(conn.bytes_sent)}</span>
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Bytes Received">
            <span style={{ color: '#53c28b' }}>{formatBytes(conn.bytes_received)}</span>
          </ProDescriptions.Item>
        </ProDescriptions>
      </Card>

      {/* Danger zone */}
      <Card
        title={<span style={{ color: '#d32f2f' }}>Danger Zone</span>}
        style={{ background: '#16213e', border: '1px solid #d32f2f' }}
        headStyle={{ borderBottom: '1px solid #4a1010' }}
      >
        <Button
          danger
          type="primary"
          onClick={() => setModalVisible(true)}
        >
          Force Disconnect
        </Button>
        <span style={{ color: '#a0a0b0', marginLeft: 12, fontSize: 13 }}>
          Immediately terminates the TCP connection and notifies the session manager.
        </span>
      </Card>

      <ConfirmActionModal
        visible={modalVisible}
        title="Force Disconnect"
        description="This will immediately close the connection. Any in-flight data will be lost."
        type="danger"
        confirmText="Disconnect"
        onConfirm={doDisconnect}
        onCancel={() => setModalVisible(false)}
      />
    </PageContainer>
  );
}
