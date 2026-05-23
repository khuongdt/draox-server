import { ProTable } from '@ant-design/pro-components';
import { Tag, Badge, Button, Popconfirm, Space } from 'antd';

interface ConnectionRecord {
  id: string;
  remote_address: string;
  protocol: 'tcp' | 'udp' | 'ws' | 'http';
  connected_at: string;
  state: 'active' | 'idle' | 'closing';
  bytes_in: number;
  bytes_out: number;
}

interface ConnectionTableProps {
  dataSource?: ConnectionRecord[];
  onDisconnect?: (id: string) => void;
  showActions?: boolean;
  loading?: boolean;
}

const PROTOCOL_STYLE: Record<string, { color: string; background: string }> = {
  tcp: { color: '#90caf9', background: '#1a237e' },
  udp: { color: '#a5d6a7', background: '#1b5e20' },
  ws: { color: '#ef9a9a', background: '#4a1010' },
  http: { color: '#ff8a65', background: '#3e2723' },
};

const STATE_STATUS: Record<string, 'success' | 'default' | 'warning'> = {
  active: 'success',
  idle: 'default',
  closing: 'warning',
};

const formatBytes = (bytes: number): string => {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(2)} GB`;
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(2)} MB`;
  if (bytes >= 1_024) return `${(bytes / 1_024).toFixed(1)} KB`;
  return `${bytes} B`;
};

const ConnectionTable: React.FC<ConnectionTableProps> = ({
  dataSource = [],
  onDisconnect,
  showActions = true,
  loading = false,
}) => {
  const columns: any[] = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 140,
      render: (v: string) => (
        <span style={{ fontFamily: 'monospace', color: '#e0e0e0', fontSize: 12 }}>
          {v.slice(0, 8)}…
        </span>
      ),
    },
    {
      title: 'Remote Address',
      dataIndex: 'remote_address',
      render: (v: string) => <span style={{ color: '#a0a0b0' }}>{v}</span>,
    },
    {
      title: 'Protocol',
      dataIndex: 'protocol',
      width: 90,
      render: (v: string) => {
        const style = PROTOCOL_STYLE[v] ?? { color: '#e0e0e0', background: '#2a2a4a' };
        return (
          <Tag
            className={`protocol-${v}`}
            style={{
              color: style.color,
              background: style.background,
              border: `1px solid ${style.color}44`,
              fontWeight: 600,
              textTransform: 'uppercase',
              fontSize: 11,
            }}
          >
            {v}
          </Tag>
        );
      },
    },
    {
      title: 'Connected At',
      dataIndex: 'connected_at',
      width: 160,
      render: (v: string) => (
        <span style={{ color: '#a0a0b0', fontSize: 12 }}>
          {new Date(v).toLocaleString()}
        </span>
      ),
    },
    {
      title: 'State',
      dataIndex: 'state',
      width: 90,
      render: (v: string) => (
        <Badge status={STATE_STATUS[v] ?? 'default'} text={<span style={{ color: '#e0e0e0' }}>{v}</span>} />
      ),
    },
    {
      title: 'Bytes In',
      dataIndex: 'bytes_in',
      width: 100,
      render: (v: number) => <span style={{ color: '#53c28b' }}>{formatBytes(v)}</span>,
    },
    {
      title: 'Bytes Out',
      dataIndex: 'bytes_out',
      width: 100,
      render: (v: number) => <span style={{ color: '#e05d10' }}>{formatBytes(v)}</span>,
    },
  ];

  if (showActions) {
    columns.push({
      title: 'Actions',
      key: 'actions',
      width: 110,
      render: (_: unknown, record: ConnectionRecord) => (
        <Popconfirm
          title="Disconnect this client?"
          description="The client connection will be terminated."
          onConfirm={() => onDisconnect?.(record.id)}
          okText="Disconnect"
          okButtonProps={{ danger: true }}
        >
          <Button size="small" danger>
            Disconnect
          </Button>
        </Popconfirm>
      ),
    });
  }

  return (
    <ProTable<ConnectionRecord>
      columns={columns}
      dataSource={dataSource}
      loading={loading}
      rowKey="id"
      search={false}
      pagination={{ pageSize: 10, showSizeChanger: false }}
      options={false}
      style={{ background: 'transparent' }}
    />
  );
};

export default ConnectionTable;
