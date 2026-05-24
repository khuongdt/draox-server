import { useEffect, useState, useCallback } from 'react';
import { PageContainer, ProTable } from '@ant-design/pro-components';
import { Tag, Dropdown, Button, message, Spin, Row, Col } from 'antd';
import { MoreOutlined, ReloadOutlined } from '@ant-design/icons';
import PluginStatusBadge from '@/components/PluginStatusBadge';
import DarkStatisticCard from '@/components/DarkStatisticCard';
import {
  listPlugins,
  activatePlugin,
  deactivatePlugin,
  enablePlugin,
  disablePlugin,
  restartPlugin,
} from '@/services/plugins';
import { wsManager } from '@/services/wsManager';

const TYPE_STYLE: Record<string, { color: string; bg: string }> = {
  builtin: { color: '#90caf9', bg: '#1a237e' },
  wasm: { color: '#ce93d8', bg: '#4a1070' },
};

export default function PluginsPage() {
  const [plugins, setPlugins] = useState<API.Plugin[]>([]);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(() => {
    setLoading(true);
    listPlugins()
      .then((data) => setPlugins(data))
      .catch((e: Error) => message.error(`Failed to load plugins: ${e.message}`))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // ── /ws/plugins — update badge states without full reload ─────────────────────
  useEffect(() => {
    const unsub = wsManager.subscribe('plugins', () => {
      refresh();
    });
    return unsub;
  }, [refresh]);

  const handleAction = async (action: string, pluginId: string, pluginName: string) => {
    try {
      switch (action) {
        case 'activate':
          await activatePlugin(pluginId);
          break;
        case 'deactivate':
          await deactivatePlugin(pluginId);
          break;
        case 'enable':
          await enablePlugin(pluginId);
          break;
        case 'disable':
          await disablePlugin(pluginId);
          break;
        case 'restart':
          await restartPlugin(pluginId);
          break;
      }
      message.success(`Plugin ${pluginName}: ${action} successful`);
      refresh();
    } catch {
      message.error(`Failed to ${action} plugin ${pluginName}`);
    }
  };

  const enabledCount = plugins.filter(
    (p: API.Plugin) => p.state === 'ActiveEnabled',
  ).length;

  const columns = [
    {
      title: 'Name',
      dataIndex: 'name',
      render: (v: string) => (
        <span style={{ color: '#e0e0e0', fontWeight: 600 }}>{v}</span>
      ),
    },
    {
      title: 'Plugin ID',
      dataIndex: 'id',
      render: (v: string) => (
        <span style={{ fontFamily: 'monospace', color: '#a0a0b0', fontSize: 12 }}>{v}</span>
      ),
    },
    {
      title: 'Version',
      dataIndex: 'version',
      width: 90,
      render: (v: string) => <span style={{ color: '#a0a0b0' }}>v{v}</span>,
    },
    {
      title: 'Type',
      dataIndex: 'plugin_type',
      width: 90,
      render: (v: string) => {
        const s = TYPE_STYLE[v] ?? TYPE_STYLE['builtin'];
        return (
          <Tag
            style={{
              color: s.color,
              background: s.bg,
              border: `1px solid ${s.color}44`,
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
      title: 'Status',
      dataIndex: 'state',
      width: 160,
      render: (v: API.PluginState) => <PluginStatusBadge state={v} />,
    },
    {
      title: 'Actions',
      key: 'actions',
      width: 80,
      render: (_: unknown, record: API.Plugin) => (
        <Dropdown
          menu={{
            items: [
              { key: 'activate', label: 'Activate' },
              { key: 'deactivate', label: 'Deactivate' },
              { type: 'divider' },
              { key: 'enable', label: 'Enable' },
              { key: 'disable', label: 'Disable' },
              { type: 'divider' },
              { key: 'restart', label: 'Restart', danger: true },
            ],
            onClick: ({ key }) => handleAction(key, record.id, record.name),
          }}
        >
          <Button size="small" icon={<MoreOutlined />} />
        </Dropdown>
      ),
    },
  ];

  return (
    <PageContainer title="Plugins" subTitle="Manage server plugins and their lifecycle">
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={24} sm={8}>
          <DarkStatisticCard title="Total Plugins" value={plugins.length} color="#e0e0e0" />
        </Col>
        <Col xs={24} sm={8}>
          <DarkStatisticCard title="Active & Enabled" value={enabledCount} color="#53c28b" />
        </Col>
        <Col xs={24} sm={8}>
          <DarkStatisticCard
            title="Disabled / Installed"
            value={plugins.length - enabledCount}
            color="#a0a0b0"
          />
        </Col>
      </Row>

      <Spin spinning={loading}>
        <ProTable<API.Plugin>
          columns={columns}
          dataSource={plugins}
          rowKey="id"
          search={false}
          options={{ reload: () => refresh() }}
          toolBarRender={() => [
            <Button key="refresh" icon={<ReloadOutlined />} onClick={refresh}>
              Refresh
            </Button>,
          ]}
          pagination={false}
          style={{ background: 'transparent' }}
        />
      </Spin>
    </PageContainer>
  );
}
