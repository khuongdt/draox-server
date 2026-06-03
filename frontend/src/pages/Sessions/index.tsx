import { PageContainer, ProTable } from '@ant-design/pro-components';
import type { ProColumns } from '@ant-design/pro-components';
import { Button, Space, Popconfirm, message, Spin, Row, Col } from 'antd';
import { useCallback, useEffect, useState } from 'react';
import DarkStatisticCard from '@/components/DarkStatisticCard';
import { listSessions, destroySession, drainSession } from '@/services/sessions';

export default function SessionsPage() {
  const [sessions, setSessions] = useState<API.Session[]>([]);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(() => {
    setLoading(true);
    listSessions()
      .then((data) => setSessions(data))
      .catch((e: Error) => message.error(`Failed to load sessions: ${e.message}`))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    refresh();
    const t = setInterval(refresh, 10_000);
    return () => clearInterval(t);
  }, [refresh]);

  const handleDestroy = async (id: string) => {
    await destroySession(id);
    message.success('Session destroyed');
    refresh();
  };

  const handleDrain = async (id: string) => {
    await drainSession(id);
    message.info('Session set to draining');
    refresh();
  };

  const columns: ProColumns<API.Session>[] = [
    {
      title: 'Session ID',
      dataIndex: 'session_id',
      render: (_dom, record) => (
        <span style={{ fontFamily: 'monospace', color: '#e0e0e0', fontSize: 12 }}>
          {record.session_id.slice(0, 20)}…
        </span>
      ),
    },
    {
      title: 'Client ID',
      dataIndex: 'client_id',
      render: (_dom, record) => <span style={{ color: '#a0a0b0' }}>{record.client_id}</span>,
    },
    {
      title: 'Connections',
      dataIndex: 'connection_count',
      render: (_dom, record) => (
        <span style={{ color: '#ff8c42', fontWeight: 700 }}>{record.connection_count ?? 0}</span>
      ),
    },
    {
      title: 'Created At',
      dataIndex: 'created_at',
      render: (_dom, record) => (
        <span style={{ color: '#a0a0b0', fontSize: 12 }}>{new Date(record.created_at).toLocaleString()}</span>
      ),
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_dom, record) => (
        <Space>
          <Popconfirm
            title="Drain this session?"
            description="New connections will be rejected; existing connections can finish."
            onConfirm={() => handleDrain(record.session_id)}
            okText="Drain"
            okButtonProps={{ style: { background: '#f5a623', borderColor: '#f5a623', color: '#000' } }}
          >
            <Button size="small" style={{ color: '#f5a623', borderColor: '#f5a623' }}>
              Drain
            </Button>
          </Popconfirm>
          <Popconfirm
            title="Destroy this session?"
            description="All connections in this session will be immediately terminated."
            onConfirm={() => handleDestroy(record.session_id)}
            okText="Destroy"
            okButtonProps={{ danger: true }}
          >
            <Button size="small" danger>
              Destroy
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <PageContainer title="Sessions" subTitle="Server-authoritative client sessions">
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={24} sm={12}>
          <DarkStatisticCard title="Total Sessions" value={sessions.length} color="#e0e0e0" />
        </Col>
        <Col xs={24} sm={12}>
          <DarkStatisticCard
            title="Total Connections"
            value={sessions.reduce((sum, s) => sum + (s.connection_count ?? 0), 0)}
            color="#53c28b"
          />
        </Col>
      </Row>

      <Spin spinning={loading}>
        <ProTable<API.Session>
          columns={columns}
          dataSource={sessions}
          rowKey="session_id"
          search={false}
          options={{ reload: () => refresh() }}
          pagination={{ pageSize: 20 }}
          style={{ background: 'transparent' }}
        />
      </Spin>
    </PageContainer>
  );
}
