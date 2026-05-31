import { PageContainer, ProTable } from '@ant-design/pro-components';
import type { ProColumns } from '@ant-design/pro-components';
import { Badge, Button, Space, Popconfirm, message, Spin, Row, Col } from 'antd';
import { useCallback, useEffect, useState } from 'react';
import DarkStatisticCard from '@/components/DarkStatisticCard';
import { listSessions, destroySession, drainSession } from '@/services/sessions';

const STATE_STATUS: Record<string, 'success' | 'processing' | 'default'> = {
  active: 'success',
  draining: 'processing',
  closed: 'default',
};

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

  const activeCount = sessions.filter((s) => s.state === 'active').length;

  const columns: ProColumns<API.Session>[] = [
    {
      title: 'Session ID',
      dataIndex: 'id',
      render: (_dom, record) => (
        <span style={{ fontFamily: 'monospace', color: '#e0e0e0', fontSize: 12 }}>
          {record.id.slice(0, 20)}…
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
      dataIndex: 'connections',
      render: (_dom, record) => (
        <span style={{ color: '#ff8c42', fontWeight: 700 }}>{record.connections?.length ?? 0}</span>
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
      title: 'State',
      dataIndex: 'state',
      render: (_dom, record) => (
        <Badge
          status={STATE_STATUS[record.state] ?? 'default'}
          text={<span style={{ color: '#e0e0e0' }}>{record.state}</span>}
        />
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
            onConfirm={() => handleDrain(record.id)}
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
            onConfirm={() => handleDestroy(record.id)}
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
          <DarkStatisticCard title="Active" value={activeCount} color="#53c28b" />
        </Col>
      </Row>

      <Spin spinning={loading}>
        <ProTable<API.Session>
          columns={columns}
          dataSource={sessions}
          rowKey="id"
          search={false}
          options={{ reload: () => refresh() }}
          pagination={{ pageSize: 20 }}
          style={{ background: 'transparent' }}
        />
      </Spin>
    </PageContainer>
  );
}
