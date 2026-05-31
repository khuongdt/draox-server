import { useEffect, useState, useCallback } from 'react';
import { PageContainer } from '@ant-design/pro-components';
import { Row, Col, message, Spin } from 'antd';
import DarkStatisticCard from '@/components/DarkStatisticCard';
import ConnectionTable from '@/components/ConnectionTable';
import ConfirmActionModal from '@/components/ConfirmActionModal';
import { listConnections, disconnectConnection, getConnectionStats } from '@/services/connections';
import { wsManager } from '@/services/wsManager';

export default function ConnectionsPage() {
  const [modalVisible, setModalVisible] = useState(false);
  const [selectedId, setSelectedId] = useState('');
  const [connections, setConnections] = useState<API.Connection[]>([]);
  const [loading, setLoading] = useState(false);
  const [stats, setStats] = useState<API.ConnectionStats | undefined>(undefined);

  // ── HTTP data ────────────────────────────────────────────────────────────────
  // listConnections + getConnectionStats are fetched independently so a stats
  // failure (e.g. 404 or backend error) still lets the table render.
  const refresh = useCallback(() => {
    setLoading(true);
    listConnections()
      .then((list) => setConnections(list))
      .catch((e: Error) => message.error(`Failed to load connections: ${e.message}`))
      .finally(() => setLoading(false));
    getConnectionStats()
      .then((st) => setStats(st))
      .catch(() => {
        // Stats are optional — silent fail. Table still works.
        setStats(undefined);
      });
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  // ── WebSocket /ws/connections — auto-refresh on state changes ─────────────────
  useEffect(() => {
    const unsub = wsManager.subscribe('connections', () => {
      refresh();
    });
    return unsub;
  }, [refresh]);

  // ── Actions ───────────────────────────────────────────────────────────────────
  const handleDisconnect = (id: string) => {
    setSelectedId(id);
    setModalVisible(true);
  };

  const doDisconnect = async () => {
    await disconnectConnection(selectedId);
    setModalVisible(false);
    message.success('Client disconnected');
    refresh();
  };

  const activeCount =
    stats?.active ??
    connections.filter((c) => c.state === 'established').length;

  return (
    <PageContainer title="Connections" subTitle="Manage active client connections">
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={24} sm={12} lg={6}>
          <DarkStatisticCard
            title="Total Connections"
            value={stats?.total ?? connections.length}
            color="#e0e0e0"
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <DarkStatisticCard title="Active" value={activeCount} color="#53c28b" />
        </Col>
        {stats?.by_protocol &&
          Object.entries(stats.by_protocol)
            .slice(0, 2)
            .map(([proto, count]) => (
              <Col key={proto} xs={24} sm={12} lg={6}>
                <DarkStatisticCard
                  title={proto.toUpperCase()}
                  value={count as number}
                  color="#90caf9"
                />
              </Col>
            ))}
      </Row>

      <Spin spinning={loading}>
        <ConnectionTable
          dataSource={connections.map((c) => ({
            id: c.id,
            remote_address: c.remote_addr,
            protocol: c.protocol as 'tcp' | 'udp' | 'ws' | 'http',
            connected_at: c.connected_at,
            state: (c.state === 'established' ? 'active' : c.state) as
              | 'active'
              | 'idle'
              | 'closing',
            bytes_in: c.bytes_received,
            bytes_out: c.bytes_sent,
          }))}
          onDisconnect={handleDisconnect}
          showActions
        />
      </Spin>

      <ConfirmActionModal
        visible={modalVisible}
        title="Disconnect Client"
        description={`Force disconnect connection ${selectedId.slice(0, 16)}…? The client will lose all active data transfer.`}
        type="danger"
        confirmText="Disconnect"
        onConfirm={doDisconnect}
        onCancel={() => setModalVisible(false)}
      />
    </PageContainer>
  );
}
