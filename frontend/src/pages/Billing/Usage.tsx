import { useRequest, useAccess } from '@umijs/max';
import { PageContainer, ProDescriptions } from '@ant-design/pro-components';
import { Card, Input, Button, Form, Row, Col, Progress, Alert, Skeleton } from 'antd';
import { SearchOutlined } from '@ant-design/icons';
import { getUsage } from '@/services/billing';
import DarkStatisticCard from '@/components/DarkStatisticCard';
import { formatBytes } from '@/utils/formatters';

export default function UsagePage() {
  const access = useAccess();
  const [form] = Form.useForm();

  const {
    data: usage,
    loading,
    error,
    run: fetchUsage,
  } = useRequest((clientId: string) => getUsage(clientId), { manual: true });

  const handleSearch = ({ client_id }: { client_id: string }) => {
    fetchUsage(client_id);
  };

  if (!access?.canBillingManage) {
    return (
      <PageContainer title="Billing Usage">
        <Alert
          type="error"
          message="Insufficient permissions"
          description="Admin role required to view billing usage."
        />
      </PageContainer>
    );
  }

  const bwUsed = usage?.bandwidth_used ?? 0;
  const bwMax = 1_000_000_000; // 1 GB default when no plan data available
  const bwPercent = Math.min(100, Math.round((bwUsed / bwMax) * 100));
  const connMax = 10; // placeholder — would come from plan data

  return (
    <PageContainer title="Billing Usage" subTitle="View usage metrics for a client">
      {/* Search bar */}
      <Card
        title={<span style={{ color: '#e0e0e0' }}>Client Lookup</span>}
        style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
        headStyle={{ borderBottom: '1px solid #2a2a4a' }}
      >
        <Form form={form} layout="inline" onFinish={handleSearch}>
          <Form.Item
            name="client_id"
            rules={[{ required: true, message: 'Client ID required' }]}
          >
            <Input placeholder="Client ID (e.g., client-001)" style={{ width: 260 }} />
          </Form.Item>
          <Form.Item>
            <Button
              htmlType="submit"
              loading={loading}
              icon={<SearchOutlined />}
              style={{ background: '#e05d10', borderColor: '#e05d10', color: '#fff', fontWeight: 600 }}
            >
              Search
            </Button>
          </Form.Item>
        </Form>
      </Card>

      {error && (
        <Alert type="error" message="Client not found or usage data unavailable" style={{ marginBottom: 16 }} />
      )}

      {loading && (
        <Card style={{ background: '#16213e', border: '1px solid #2a2a4a' }}>
          <Skeleton active paragraph={{ rows: 4 }} />
        </Card>
      )}

      {!loading && usage && (
        <>
          {/* Client info */}
          <Card
            style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
            bodyStyle={{ padding: 24 }}
          >
            <ProDescriptions
              column={3}
              labelStyle={{ color: '#a0a0b0' }}
              contentStyle={{ color: '#e0e0e0' }}
            >
              <ProDescriptions.Item label="Client ID">
                <span style={{ fontFamily: 'monospace' }}>{usage.client_id}</span>
              </ProDescriptions.Item>
              <ProDescriptions.Item label="Plan">
                <span style={{ color: '#ff8c42', fontWeight: 700 }}>{usage.plan_id}</span>
              </ProDescriptions.Item>
              <ProDescriptions.Item label="API Calls">
                <span style={{ color: '#ab47bc', fontWeight: 700 }}>
                  {usage.api_calls.toLocaleString()}
                </span>
              </ProDescriptions.Item>
            </ProDescriptions>
          </Card>

          {/* Usage meters */}
          <Row gutter={[16, 16]}>
            <Col xs={24} sm={8}>
              <DarkStatisticCard
                title="Connections Used"
                value={`${usage.connections_used} / ${connMax}`}
                color={usage.connections_used >= connMax ? '#d32f2f' : '#e0e0e0'}
              />
            </Col>
            <Col xs={24} sm={8}>
              <Card
                style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
                bodyStyle={{ padding: '20px 24px' }}
              >
                <div style={{ color: '#a0a0b0', fontSize: 13, marginBottom: 8 }}>
                  Bandwidth Used
                </div>
                <div style={{ color: '#e0e0e0', fontSize: 22, fontWeight: 700, marginBottom: 8 }}>
                  {formatBytes(bwUsed)}{' '}
                  <span style={{ color: '#a0a0b0', fontSize: 14, fontWeight: 400 }}>
                    / {formatBytes(bwMax)}
                  </span>
                </div>
                <Progress
                  percent={bwPercent}
                  strokeColor={bwPercent >= 90 ? '#d32f2f' : '#e05d10'}
                  trailColor="#2a2a4a"
                  size="small"
                />
              </Card>
            </Col>
            <Col xs={24} sm={8}>
              <DarkStatisticCard
                title="API Calls"
                value={usage.api_calls.toLocaleString()}
                color="#ab47bc"
              />
            </Col>
          </Row>
        </>
      )}
    </PageContainer>
  );
}
