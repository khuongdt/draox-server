import { useRequest, useAccess } from '@umijs/max';
import { PageContainer, ProCard } from '@ant-design/pro-components';
import {
  Tag, Button, List, Typography, Skeleton, Empty,
  Modal, Form, Input, message, Alert,
} from 'antd';
import { CheckCircleOutlined } from '@ant-design/icons';
import { useState } from 'react';
import { getPlans, assignPlan } from '@/services/billing';

const { Title, Text } = Typography;

const PLAN_COLORS: Record<number, string> = {};
const FALLBACK_COLORS = ['#a0a0b0', '#e05d10', '#53c28b'];

export default function PlansPage() {
  const access = useAccess();
  const [assignModal, setAssignModal] = useState<{ visible: boolean; planId: string; planName: string }>({
    visible: false,
    planId: '',
    planName: '',
  });
  const [assignForm] = Form.useForm();

  const { data: plans = [], loading, error } = useRequest(getPlans);

  const { loading: assigning, run: runAssign } = useRequest(
    (clientId: string, planId: string) => assignPlan(clientId, planId),
    {
      manual: true,
      onSuccess: () => {
        message.success(`Plan assigned successfully`);
        setAssignModal({ visible: false, planId: '', planName: '' });
        assignForm.resetFields();
      },
    },
  );

  const handleAssignSubmit = ({ client_id }: { client_id: string }) => {
    runAssign(client_id, assignModal.planId);
  };

  if (error) {
    return (
      <PageContainer title="Subscription Plans">
        <Alert type="error" message="Failed to load billing plans" description={String(error)} />
      </PageContainer>
    );
  }

  return (
    <PageContainer title="Subscription Plans" subTitle="Choose the plan that fits your scale">
      <Skeleton loading={loading} active paragraph={{ rows: 8 }}>
        {plans.length === 0 ? (
          <Empty description={<span style={{ color: '#a0a0b0' }}>No plans available</span>} />
        ) : (
          <ProCard.Group style={{ gap: 16, flexWrap: 'wrap' }}>
            {plans.map((plan, idx) => {
              const color = FALLBACK_COLORS[idx % FALLBACK_COLORS.length];
              const isCurrent = idx === 0; // API should indicate current plan; use first as fallback

              return (
                <ProCard
                  key={plan.id}
                  style={{
                    background: '#16213e',
                    border: `1px solid ${isCurrent ? color : '#2a2a4a'}`,
                    borderRadius: 12,
                    flex: 1,
                    minWidth: 260,
                    position: 'relative',
                  }}
                  bodyStyle={{ padding: 28 }}
                >
                  {isCurrent && (
                    <Tag
                      color="#e05d10"
                      style={{ position: 'absolute', top: 16, right: 16, fontWeight: 700 }}
                    >
                      Current Plan
                    </Tag>
                  )}

                  <Title level={3} style={{ color, margin: 0 }}>
                    {plan.name}
                  </Title>

                  <div style={{ margin: '12px 0 8px' }}>
                    <span style={{ color: '#e0e0e0', fontSize: 36, fontWeight: 800 }}>
                      {plan.price_cents === 0 ? '$0' : `$${(plan.price_cents / 100).toFixed(0)}`}
                    </span>
                    <span style={{ color: '#a0a0b0', fontSize: 14 }}>/mo</span>
                  </div>

                  <Text style={{ color: '#ff8c42', fontWeight: 600, display: 'block', marginBottom: 20 }}>
                    {plan.max_connections > 0
                      ? `${plan.max_connections} connections`
                      : 'Unlimited connections'}
                  </Text>

                  <List
                    dataSource={plan.features}
                    renderItem={(item) => (
                      <List.Item style={{ padding: '4px 0', border: 'none' }}>
                        <CheckCircleOutlined style={{ color: '#53c28b', marginRight: 8 }} />
                        <Text style={{ color: '#e0e0e0' }}>{item}</Text>
                      </List.Item>
                    )}
                    style={{ marginBottom: 24 }}
                  />

                  {access?.canBillingManage ? (
                    <Button
                      block
                      type={isCurrent ? 'default' : 'primary'}
                      disabled={isCurrent}
                      style={
                        !isCurrent
                          ? { background: '#e05d10', borderColor: '#e05d10', fontWeight: 600 }
                          : { color: '#a0a0b0', borderColor: '#2a2a4a' }
                      }
                      onClick={() =>
                        setAssignModal({ visible: true, planId: plan.id, planName: plan.name })
                      }
                    >
                      {isCurrent ? 'Active' : 'Assign to Client'}
                    </Button>
                  ) : (
                    <Button block disabled style={{ color: '#a0a0b0', borderColor: '#2a2a4a' }}>
                      {isCurrent ? 'Active' : 'Contact Admin'}
                    </Button>
                  )}
                </ProCard>
              );
            })}
          </ProCard.Group>
        )}
      </Skeleton>

      {/* Assign plan modal */}
      <Modal
        open={assignModal.visible}
        title={<span style={{ color: '#e0e0e0' }}>Assign Plan: {assignModal.planName}</span>}
        onCancel={() => setAssignModal({ visible: false, planId: '', planName: '' })}
        footer={null}
        style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
        styles={{
          header: { background: '#16213e', borderBottom: '1px solid #2a2a4a' },
        }}
      >
        <Form form={assignForm} onFinish={handleAssignSubmit} layout="vertical">
          <Form.Item
            name="client_id"
            label={<span style={{ color: '#a0a0b0' }}>Client ID</span>}
            rules={[{ required: true, message: 'Client ID is required' }]}
          >
            <Input placeholder="e.g., client-001" />
          </Form.Item>
          <Button
            htmlType="submit"
            type="primary"
            loading={assigning}
            block
            style={{ background: '#e05d10', borderColor: '#e05d10', fontWeight: 600 }}
          >
            Assign Plan
          </Button>
        </Form>
      </Modal>
    </PageContainer>
  );
}
