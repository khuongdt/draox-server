import { useRequest } from '@umijs/max';
import { PageContainer, ProTable } from '@ant-design/pro-components';
import { Badge, Button, Space, Popconfirm, message, Spin, Row, Col } from 'antd';
import DarkStatisticCard from '@/components/DarkStatisticCard';
import { listSessions, destroySession, drainSession } from '@/services/sessions';

const STATE_STATUS: Record<string, 'success' | 'processing' | 'default'> = {
  active: 'success',
  draining: 'processing',
  closed: 'default',
};

export default function SessionsPage() {
  const { data: sessions = [], loading, refresh } = useRequest(listSessions, {
    refreshOnWindowFocus: false,
    pollingInterval: 10_000,
  });

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

  const activeCount = sessions.filter((s: API.Session) => s.state === 'active').length;

  const columns = [
    {
      title: 'Session ID',
      dataIndex: 'id',
      render: (v: string) => (
        <span style={{ fontFamily: 'monospace', color: '#e0e0e0', fontSize: 12 }}>
          {v.slice(0, 20)}…
        </span>
      ),
    },
    {
      title: 'Client ID',
      dataIndex: 'client_id',
      render: (v: string) => <span style={{ color: '#a0a0b0' }}>{v}</span>,
    },
    {
      title: 'Connections',
      dataIndex: 'connections',
      render: (v: string[]) => (
        <span style={{ color: '#ff8c42', fontWeight: 700 }}>{v?.length ?? 0}</span>
      ),
    },
    {
      title: 'Created At',
      dataIndex: 'created_at',
      render: (v: string) => (
        <span style={{ color: '#a0a0b0', fontSize: 12 }}>{new Date(v).toLocaleString()}</span>
      ),
    },
    {
      title: 'State',
      dataIndex: 'state',
      render: (v: string) => (
        <Badge
          status={STATE_STATUS[v] ?? 'default'}
          text={<span style={{ color: '#e0e0e0' }}>{v}</span>}
        />
      ),
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_: unknown, record: API.Session) => (
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
