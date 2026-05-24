import { PageContainer, ProTable } from '@ant-design/pro-components';
import { Tag, DatePicker, message } from 'antd';
import { useState, useEffect } from 'react';
import { getAuditLogs } from '@/services/audit';

const SEVERITY_STYLE: Record<string, { color: string; bg: string }> = {
  critical: { color: '#ef5350', bg: '#4a1010' },
  high: { color: '#ff7043', bg: '#3e1a00' },
  medium: { color: '#ffb300', bg: '#3e2f00' },
  low: { color: '#66bb6a', bg: '#0a2a0a' },
};

export default function AuditPage() {
  const [dateRange, setDateRange] = useState<[string, string] | null>(null);
  const [severityFilter, setSeverityFilter] = useState<string | undefined>();
  const [page, setPage] = useState(1);
  const pageSize = 20;

  const [logs, setLogs] = useState<API.AuditEntry[]>([]);
  const [loading, setLoading] = useState(false);

  const refresh = () => {
    setLoading(true);
    getAuditLogs({ page, size: pageSize, severity: severityFilter })
      .then((data) => setLogs(data))
      .catch((e: Error) => message.error(`Failed to load audit logs: ${e.message}`))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
  }, [page, severityFilter]); // eslint-disable-line react-hooks/exhaustive-deps

  const columns = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 130,
      render: (v: string) => (
        <span style={{ fontFamily: 'monospace', color: '#a0a0b0', fontSize: 12 }}>{v}</span>
      ),
    },
    {
      title: 'Timestamp',
      dataIndex: 'timestamp',
      width: 170,
      sorter: (a: API.AuditEntry, b: API.AuditEntry) =>
        new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime(),
      render: (v: string) => (
        <span style={{ color: '#a0a0b0', fontSize: 12 }}>{new Date(v).toLocaleString()}</span>
      ),
    },
    {
      title: 'Action',
      dataIndex: 'action',
      render: (v: string) => (
        <span style={{ fontFamily: 'monospace', color: '#ff8c42' }}>{v}</span>
      ),
    },
    {
      title: 'Actor',
      dataIndex: 'actor',
      render: (v: string) => <span style={{ color: '#e0e0e0' }}>{v}</span>,
    },
    {
      title: 'Target',
      dataIndex: 'target',
      render: (v: string) => <span style={{ color: '#a0a0b0' }}>{v}</span>,
    },
    {
      title: 'Severity',
      dataIndex: 'severity',
      width: 110,
      filters: [
        { text: 'Critical', value: 'critical' },
        { text: 'High', value: 'high' },
        { text: 'Medium', value: 'medium' },
        { text: 'Low', value: 'low' },
      ],
      onFilter: (value: boolean | React.Key, record: API.AuditEntry) =>
        record.severity === value,
      render: (v: string) => {
        const s = SEVERITY_STYLE[v] ?? { color: '#a0a0b0', bg: '#2a2a4a' };
        return (
          <Tag
            style={{
              color: s.color,
              background: s.bg,
              border: `1px solid ${s.color}44`,
              fontWeight: 700,
              textTransform: 'uppercase',
              fontSize: 11,
            }}
          >
            {v}
          </Tag>
        );
      },
    },
  ];

  return (
    <PageContainer
      title="Audit Log"
      subTitle="Security and operational audit trail"
      extra={
        <DatePicker.RangePicker
          onChange={(_, str) => setDateRange(str as [string, string])}
          style={{ background: '#16213e', borderColor: '#2a2a4a' }}
        />
      }
    >
      <ProTable<API.AuditEntry>
        columns={columns}
        dataSource={logs}
        rowKey="id"
        loading={loading}
        search={false}
        options={{ reload: () => refresh() }}
        pagination={{
          pageSize,
          current: page,
          onChange: (p) => setPage(p),
          showSizeChanger: false,
        }}
        style={{ background: 'transparent' }}
      />
    </PageContainer>
  );
}
