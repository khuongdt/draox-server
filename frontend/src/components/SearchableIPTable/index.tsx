import { ProTable } from '@ant-design/pro-components';
import { Button, Input, Tooltip } from 'antd';
import { useState } from 'react';

interface SearchableIPTableProps {
  data: Array<{ ip: string; [key: string]: unknown }>;
  columns?: any[];
  onAction?: (ip: string, action: string) => void;
  actionLabel?: string;
  actionDisabled?: boolean;
  actionTooltip?: string;
}

const SearchableIPTable: React.FC<SearchableIPTableProps> = ({
  data,
  columns = [],
  onAction,
  actionLabel = 'Remove',
  actionDisabled = false,
  actionTooltip,
}) => {
  const [searchText, setSearchText] = useState('');

  const filtered = data.filter((row) =>
    row.ip.toLowerCase().includes(searchText.toLowerCase()),
  );

  const ipColumn = {
    title: 'IP Address',
    dataIndex: 'ip',
    key: 'ip',
    render: (v: string) => (
      <span style={{ fontFamily: 'monospace', color: '#e0e0e0' }}>{v}</span>
    ),
  };

  const actionColumn = {
    title: 'Action',
    key: 'action',
    width: 100,
    render: (_: unknown, record: { ip: string }) => {
      const btn = (
        <Button
          size="small"
          danger
          disabled={actionDisabled}
          onClick={() => onAction?.(record.ip, 'remove')}
        >
          {actionLabel}
        </Button>
      );
      return actionDisabled && actionTooltip ? (
        <Tooltip title={actionTooltip}>{btn}</Tooltip>
      ) : btn;
    },
  };

  const allColumns = [ipColumn, ...columns, actionColumn];

  return (
    <div>
      <Input.Search
        placeholder="Search IP address…"
        value={searchText}
        onChange={(e) => setSearchText(e.target.value)}
        allowClear
        style={{ marginBottom: 12 }}
      />
      <ProTable
        columns={allColumns}
        dataSource={filtered}
        rowKey="ip"
        search={false}
        options={false}
        pagination={{ pageSize: 10, showSizeChanger: false }}
        style={{ background: 'transparent' }}
        size="small"
      />
    </div>
  );
};

export default SearchableIPTable;
