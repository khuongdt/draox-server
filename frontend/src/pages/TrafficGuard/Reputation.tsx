import { PageContainer } from '@ant-design/pro-components';
import { Card, Input, Button, Form, message } from 'antd';
import { useState } from 'react';
import IPReputationGauge from '@/components/IPReputationGauge';
import { getReputation } from '@/services/trafficGuard';

interface ReputationResult {
  ip: string;
  score: number;
}

export default function IPReputationPage() {
  const [result, setResult] = useState<ReputationResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [form] = Form.useForm();

  const handleLookup = async (values: { ip: string }) => {
    setLoading(true);
    try {
      const data = await getReputation(values.ip);
      setResult({ ip: data.ip, score: data.score });
    } catch {
      message.error('Failed to look up IP reputation');
      setResult(null);
    } finally {
      setLoading(false);
    }
  };

  return (
    <PageContainer title="IP Reputation" subTitle="Look up reputation score for any IP address">
      <Card
        title={<span style={{ color: '#e0e0e0' }}>IP Lookup</span>}
        style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
        headStyle={{ borderBottom: '1px solid #2a2a4a' }}
      >
        <Form form={form} layout="inline" onFinish={handleLookup}>
          <Form.Item
            name="ip"
            rules={[{ required: true, message: 'Please enter an IP address' }]}
          >
            <Input
              placeholder="Enter IP address (e.g., 203.0.113.5)"
              style={{ width: 300 }}
            />
          </Form.Item>
          <Form.Item>
            <Button
              htmlType="submit"
              loading={loading}
              style={{ background: '#e05d10', borderColor: '#e05d10', color: '#fff', fontWeight: 600 }}
            >
              Lookup
            </Button>
          </Form.Item>
        </Form>
      </Card>

      {result && (
        <Card
          title={
            <span style={{ color: '#e0e0e0' }}>
              Reputation Result:{' '}
              <span style={{ fontFamily: 'monospace', color: '#ff8c42' }}>{result.ip}</span>
            </span>
          }
          style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
          headStyle={{ borderBottom: '1px solid #2a2a4a' }}
        >
          <div style={{ maxWidth: 360 }}>
            <IPReputationGauge score={result.score} />
          </div>
        </Card>
      )}
    </PageContainer>
  );
}
