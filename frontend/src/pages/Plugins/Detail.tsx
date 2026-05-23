import { PageContainer, ProDescriptions } from '@ant-design/pro-components';
import { Card, Tag, Button, Space, Badge, message, Spin, Result } from 'antd';
import { useParams } from '@umijs/max';
import { useState, useEffect } from 'react';
import PluginStatusBadge from '@/components/PluginStatusBadge';
import {
  getPlugin, getPluginHealth,
  activatePlugin, deactivatePlugin,
  enablePlugin, disablePlugin, restartPlugin,
} from '@/services/plugins';

export default function PluginDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [plugin, setPlugin] = useState<API.Plugin | null>(null);
  const [health, setHealth] = useState<API.PluginHealth | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);

  useEffect(() => {
    if (!id) return;
    Promise.all([getPlugin(id), getPluginHealth(id)])
      .then(([p, h]) => { setPlugin(p); setHealth(h); })
      .catch(() => message.error('Failed to load plugin'))
      .finally(() => setLoading(false));
  }, [id]);

  const handleLifecycle = async (action: string) => {
    if (!id) return;
    setActionLoading(true);
    try {
      const handlers: Record<string, (pluginId: string) => Promise<void>> = {
        activate: activatePlugin,
        deactivate: deactivatePlugin,
        enable: enablePlugin,
        disable: disablePlugin,
        restart: restartPlugin,
      };
      await handlers[action]?.(id);
      const [updated, updatedHealth] = await Promise.all([getPlugin(id), getPluginHealth(id)]);
      setPlugin(updated);
      setHealth(updatedHealth);
      message.success(`Action "${action}" applied to plugin`);
    } catch {
      message.error(`Failed to apply "${action}"`);
    } finally {
      setActionLoading(false);
    }
  };

  if (loading) return <Spin style={{ display: 'block', margin: '80px auto' }} />;
  if (!plugin) return <Result status="404" title="Plugin not found" />;

  const typeStyle =
    plugin.plugin_type === 'builtin'
      ? { color: '#90caf9', bg: '#1a237e' }
      : { color: '#ce93d8', bg: '#4a1070' };

  return (
    <PageContainer title={plugin.name} subTitle={plugin.id}>
      {/* Details card */}
      <Card
        style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
        bodyStyle={{ padding: 24 }}
      >
        <ProDescriptions
          column={2}
          labelStyle={{ color: '#a0a0b0' }}
          contentStyle={{ color: '#e0e0e0' }}
        >
          <ProDescriptions.Item label="Plugin ID">
            <span style={{ fontFamily: 'monospace', fontSize: 12 }}>{plugin.id}</span>
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Version">v{plugin.version}</ProDescriptions.Item>
          <ProDescriptions.Item label="Author">{plugin.author}</ProDescriptions.Item>
          <ProDescriptions.Item label="Type">
            <Tag
              style={{
                color: typeStyle.color,
                background: typeStyle.bg,
                border: `1px solid ${typeStyle.color}44`,
                fontWeight: 600,
                textTransform: 'uppercase',
              }}
            >
              {plugin.plugin_type}
            </Tag>
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Status" span={2}>
            <PluginStatusBadge state={plugin.state} />
          </ProDescriptions.Item>
          <ProDescriptions.Item label="Description" span={2}>
            {plugin.description}
          </ProDescriptions.Item>
        </ProDescriptions>
      </Card>

      {/* Health card */}
      <Card
        title={<span style={{ color: '#e0e0e0' }}>Plugin Health</span>}
        style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
        headStyle={{ borderBottom: '1px solid #2a2a4a' }}
      >
        <Space size={24}>
          <Badge
            status={health?.status === 'healthy' ? 'success' : 'error'}
            text={
              <span style={{ color: health?.status === 'healthy' ? '#53c28b' : '#f44336', fontWeight: 600 }}>
                {health?.status ?? '—'}
              </span>
            }
          />
        </Space>
      </Card>

      {/* Lifecycle actions */}
      <Card
        title={<span style={{ color: '#e0e0e0' }}>Lifecycle</span>}
        style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
        headStyle={{ borderBottom: '1px solid #2a2a4a' }}
      >
        <Space>
          <Button loading={actionLoading} onClick={() => handleLifecycle('activate')}>Activate</Button>
          <Button loading={actionLoading} onClick={() => handleLifecycle('deactivate')}>Deactivate</Button>
          <Button loading={actionLoading} onClick={() => handleLifecycle('enable')} style={{ color: '#53c28b', borderColor: '#53c28b' }}>Enable</Button>
          <Button loading={actionLoading} onClick={() => handleLifecycle('disable')} style={{ color: '#f5a623', borderColor: '#f5a623' }}>Disable</Button>
          <Button loading={actionLoading} danger onClick={() => handleLifecycle('restart')}>Restart</Button>
        </Space>
      </Card>
    </PageContainer>
  );
}
