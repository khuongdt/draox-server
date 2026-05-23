import { useEffect } from 'react';
import { useRequest } from '@umijs/max';
import { PageContainer, ProTable } from '@ant-design/pro-components';
import { Tabs, Row, Col, Card, Input, Button, Form, Space, message, Spin } from 'antd';
import DarkStatisticCard from '@/components/DarkStatisticCard';
import SearchableIPTable from '@/components/SearchableIPTable';
import IPReputationGauge from '@/components/IPReputationGauge';
import {
  getGuardStats,
  listBans,
  banIp,
  unbanIp,
  addWhitelist,
  addBlacklist,
  getReputation,
} from '@/services/trafficGuard';
import { wsManager } from '@/services/wsManager';

const IPV4_PATTERN = /^(\d{1,3}\.){3}\d{1,3}(\/\d{1,2})?$/;
const IPV6_PATTERN = /^([0-9a-fA-F:]+)(\/\d{1,3})?$/;

export default function TrafficGuardPage() {
  const [banForm] = Form.useForm();
  const [wlInput, setWlInput] = Form.useWatch ? ['', () => {}] : ['', () => {}];

  // ── HTTP data ────────────────────────────────────────────────────────────────
  const { data: guardStats, refresh: refreshStats } = useRequest(getGuardStats, {
    refreshOnWindowFocus: false,
    pollingInterval: 15_000,
  });

  const {
    data: banData,
    loading: bansLoading,
    refresh: refreshBans,
  } = useRequest(listBans, { refreshOnWindowFocus: false });

  // ── /ws/guard — auto-refresh stats + bans on guard events ────────────────────
  useEffect(() => {
    const unsub = wsManager.subscribe('guard', () => {
      refreshStats();
      refreshBans();
    });
    return unsub;
  }, [refreshStats, refreshBans]);

  const bans = banData?.bans ?? [];

  // ── Actions ──────────────────────────────────────────────────────────────────
  const handleBanIP = async (values: { ip: string; reason?: string }) => {
    await banIp(values.ip, values.reason);
    banForm.resetFields();
    message.success(`IP ${values.ip} banned`);
    refreshBans();
    refreshStats();
  };

  const handleUnban = async (ip: string) => {
    await unbanIp(ip);
    message.success(`IP ${ip} unbanned`);
    refreshBans();
    refreshStats();
  };

  const handleAddWhitelist = async (ip: string) => {
    if (!ip) return;
    await addWhitelist(ip);
    message.success('IP added to whitelist');
    refreshStats();
  };

  const handleAddBlacklist = async (ip: string) => {
    if (!ip) return;
    await addBlacklist(ip);
    message.success('IP added to blacklist');
    refreshStats();
  };

  const banColumns = [
    {
      title: 'IP',
      dataIndex: 'ip',
      render: (v: string) => (
        <span style={{ fontFamily: 'monospace', color: '#e0e0e0' }}>{v}</span>
      ),
    },
    {
      title: 'Reason',
      dataIndex: 'reason',
      render: (v: string) => <span style={{ color: '#a0a0b0' }}>{v}</span>,
    },
    {
      title: 'Expires',
      dataIndex: 'expires_at',
      render: (v: string) => (
        <span style={{ color: '#a0a0b0', fontSize: 12 }}>{new Date(v).toLocaleString()}</span>
      ),
    },
    {
      title: 'Count',
      dataIndex: 'ban_count',
      render: (v: number) => (
        <span style={{ color: '#d32f2f', fontWeight: 700 }}>{v}</span>
      ),
    },
    {
      title: 'Action',
      key: 'action',
      render: (_: unknown, record: API.BanEntry) => (
        <Button size="small" onClick={() => handleUnban(record.ip)}>
          Unban
        </Button>
      ),
    },
  ];

  const tabItems = [
    {
      key: 'overview',
      label: 'Overview',
      children: (
        <Row gutter={[16, 16]}>
          <Col xs={24} sm={8}>
            <DarkStatisticCard
              title="Active Bans"
              value={guardStats?.active_bans ?? 0}
              color="#d32f2f"
            />
          </Col>
          <Col xs={24} sm={8}>
            <DarkStatisticCard
              title="Blacklisted IPs"
              value={guardStats?.blacklisted_entries ?? 0}
              color="#f5a623"
            />
          </Col>
          <Col xs={24} sm={8}>
            <DarkStatisticCard
              title="Whitelisted IPs"
              value={guardStats?.whitelisted_entries ?? 0}
              color="#53c28b"
            />
          </Col>
        </Row>
      ),
    },
    {
      key: 'bans',
      label: `Bans${bans.length > 0 ? ` (${bans.length})` : ''}`,
      children: (
        <div>
          <Card
            title={<span style={{ color: '#e0e0e0' }}>Ban IP Address</span>}
            style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
            headStyle={{ borderBottom: '1px solid #2a2a4a' }}
          >
            <Form form={banForm} layout="inline" onFinish={handleBanIP}>
              <Form.Item
                name="ip"
                rules={[
                  { required: true, message: 'IP required' },
                  {
                    validator: (_, val) =>
                      IPV4_PATTERN.test(val) || IPV6_PATTERN.test(val)
                        ? Promise.resolve()
                        : Promise.reject('Invalid IP address'),
                  },
                ]}
              >
                <Input placeholder="IP address (IPv4 or IPv6)" style={{ width: 220 }} />
              </Form.Item>
              <Form.Item name="reason">
                <Input placeholder="Reason (optional)" style={{ width: 200 }} />
              </Form.Item>
              <Form.Item>
                <Button htmlType="submit" danger type="primary">
                  Ban IP
                </Button>
              </Form.Item>
            </Form>
          </Card>

          <Spin spinning={bansLoading}>
            <ProTable
              columns={banColumns}
              dataSource={bans}
              rowKey="ip"
              search={false}
              options={{ reload: () => refreshBans() }}
              pagination={{ pageSize: 20 }}
            />
          </Spin>
        </div>
      ),
    },
    {
      key: 'lists',
      label: 'IP Lists',
      children: (
        <Row gutter={[16, 16]}>
          <Col xs={24} lg={12}>
            <Card
              title={<span style={{ color: '#53c28b' }}>Whitelist</span>}
              style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
              headStyle={{ borderBottom: '1px solid #2a2a4a' }}
            >
              <AddIPForm onAdd={handleAddWhitelist} placeholder="IP to whitelist" />
              <SearchableIPTable data={[]} actionLabel="Remove" actionDisabled />
            </Card>
          </Col>
          <Col xs={24} lg={12}>
            <Card
              title={<span style={{ color: '#d32f2f' }}>Blacklist</span>}
              style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
              headStyle={{ borderBottom: '1px solid #2a2a4a' }}
            >
              <AddIPForm onAdd={handleAddBlacklist} placeholder="IP to blacklist" isDanger />
              <SearchableIPTable data={[]} actionLabel="Remove" actionDisabled />
            </Card>
          </Col>
        </Row>
      ),
    },
    {
      key: 'reputation',
      label: 'IP Reputation',
      children: <ReputationTab />,
    },
  ];

  return (
    <PageContainer title="Traffic Guard" subTitle="Anti-spam, DDoS protection and rate limiting">
      <Tabs defaultActiveKey="overview" items={tabItems} />
    </PageContainer>
  );
}

// ── Helper sub-components ─────────────────────────────────────────────────────

function AddIPForm({
  onAdd,
  placeholder,
  isDanger = false,
}: {
  onAdd: (ip: string) => void;
  placeholder: string;
  isDanger?: boolean;
}) {
  const [form] = Form.useForm();
  const onFinish = ({ ip }: { ip: string }) => {
    onAdd(ip);
    form.resetFields();
  };
  return (
    <Form form={form} layout="inline" onFinish={onFinish} style={{ marginBottom: 12 }}>
      <Form.Item name="ip" rules={[{ required: true, message: 'IP required' }]}>
        <Input placeholder={placeholder} style={{ width: 180 }} />
      </Form.Item>
      <Form.Item>
        <Button htmlType="submit" danger={isDanger}>
          Add
        </Button>
      </Form.Item>
    </Form>
  );
}

function ReputationTab() {
  const [form] = Form.useForm();
  const { data: repData, loading, run } = useRequest(getReputation, {
    manual: true,
  });

  const onLookup = ({ ip }: { ip: string }) => run(ip);

  return (
    <Card
      title={<span style={{ color: '#e0e0e0' }}>IP Reputation Lookup</span>}
      style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
      headStyle={{ borderBottom: '1px solid #2a2a4a' }}
    >
      <Form form={form} layout="inline" onFinish={onLookup} style={{ marginBottom: 24 }}>
        <Form.Item name="ip" rules={[{ required: true, message: 'IP required' }]}>
          <Input placeholder="Enter IP address" style={{ width: 240 }} />
        </Form.Item>
        <Form.Item>
          <Button htmlType="submit" loading={loading} style={{ color: '#e05d10', borderColor: '#e05d10' }}>
            Look up
          </Button>
        </Form.Item>
      </Form>
      {repData && (
        <Row justify="center">
          <Col>
            <IPReputationGauge ip={repData.ip} score={repData.score} />
          </Col>
        </Row>
      )}
    </Card>
  );
}
