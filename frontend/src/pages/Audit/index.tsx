import { PageContainer, ProTable } from '@ant-design/pro-components';
import type { ProColumns } from '@ant-design/pro-components';
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

  const columns: ProColumns<API.AuditEntry>[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 130,
      render: (_dom, record) => (
        <span style={{ fontFamily: 'monospace', color: '#a0a0b0', fontSize: 12 }}>{record.id}</span>
      ),
    },
    {
      title: 'Timestamp',
      dataIndex: 'timestamp',
      width: 170,
      sorter: (a, b) =>
        new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime(),
      render: (_dom, record) => (
        <span style={{ color: '#a0a0b0', fontSize: 12 }}>{new Date(record.timestamp).toLocaleString()}</span>
      ),
    },
    {
      title: 'Action',
      dataIndex: 'action',
      render: (_dom, record) => (
        <span style={{ fontFamily: 'monospace', color: '#ff8c42' }}>{record.action}</span>
      ),
    },
    {
      title: 'Actor',
      dataIndex: 'actor',
      render: (_dom, record) => <span style={{ color: '#e0e0e0' }}>{record.actor}</span>,
    },
    {
      title: 'Target',
      dataIndex: 'target',
      render: (_dom, record) => <span style={{ color: '#a0a0b0' }}>{record.target}</span>,
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
      onFilter: (value, record) => record.severity === value,
      render: (_dom, record) => {
        const s = SEVERITY_STYLE[record.severity] ?? { color: '#a0a0b0', bg: '#2a2a4a' };
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
            {record.severity}
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
