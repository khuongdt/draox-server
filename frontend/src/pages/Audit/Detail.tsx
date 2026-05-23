import { PageContainer, ProDescriptions } from '@ant-design/pro-components';
import { Card, Tag, Typography } from 'antd';
import { useParams } from '@umijs/max';

const { Text } = Typography;

const SEVERITY_STYLE: Record<string, { color: string; bg: string }> = {
  critical: { color: '#ef5350', bg: '#4a1010' },
  high: { color: '#ff7043', bg: '#3e1a00' },
  medium: { color: '#ffb300', bg: '#3e2f00' },
  low: { color: '#66bb6a', bg: '#0a2a0a' },
};

const MOCK_AUDIT_DETAIL = {
  id: 'audit-0001',
  timestamp: '2024-01-15T09:55:00Z',
  action: 'plugin.restart',
  actor: 'admin',
  target: 'clans',
  severity: 'high',
  details: {
    reason: 'Manual admin action via console',
    plugin_id: 'io.draox.plugin.clans',
    previous_state: 'ActiveEnabled',
    new_state: 'ActiveEnabled',
    restart_duration_ms: 234,
    request_ip: '192.168.0.10',
    user_agent: 'DraoxAdmin/1.0',
    session_id: 'admin-sess-abc123',
  },
};

export default function AuditDetailPage() {
  const { id } = useParams<{ id: string }>();
  const audit = { ...MOCK_AUDIT_DETAIL, id: id || MOCK_AUDIT_DETAIL.id };
  const sev = SEVERITY_STYLE[audit.severity] ?? { color: '#a0a0b0', bg: '#2a2a4a' };

  return (
    <PageContainer title={`Audit Event: ${audit.id}`} subTitle={audit.action}>
      <Card
        style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
        bodyStyle={{ padding: 24 }}
      >
        <ProDescriptions
          column={2}
          labelStyle={{ color: '#a0a0b0' }}
          contentStyle={{ color: '#e0e0e0' }}
        >
          <ProDescriptions.Item label="Audit ID">
            <span style={{ fontFamily: 'monospace', fontSize: 12 }}>{audit.id}</span>
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Timestamp">
            {new Date(audit.timestamp).toLocaleString()}
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Action">
            <span style={{ fontFamily: 'monospace', color: '#ff8c42' }}>{audit.action}</span>
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Actor">
            <span style={{ color: '#e0e0e0' }}>{audit.actor}</span>
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Target">
            <span style={{ color: '#a0a0b0' }}>{audit.target}</span>
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Severity">
            <Tag
              className={`severity-${audit.severity}`}
              style={{
                color: sev.color,
                background: sev.bg,
                border: `1px solid ${sev.color}44`,
                fontWeight: 700,
                textTransform: 'uppercase',
                fontSize: 11,
              }}
            >
              {audit.severity}
            </Tag>
          </ProDescriptions.Item>
        </ProDescriptions>
      </Card>

      <Card
        title={<span style={{ color: '#e0e0e0' }}>Event Details</span>}
        style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
        headStyle={{ borderBottom: '1px solid #2a2a4a' }}
      >
        <pre
          style={{
            background: '#0f3460',
            border: '1px solid #2a2a4a',
            borderRadius: 6,
            padding: 16,
            color: '#a0a0b0',
            fontSize: 13,
            fontFamily: 'monospace',
            overflow: 'auto',
            margin: 0,
          }}
        >
          {JSON.stringify(audit.details, null, 2)}
        </pre>
      </Card>
    </PageContainer>
  );
}
