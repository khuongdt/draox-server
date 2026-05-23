import { useRequest, useAccess } from '@umijs/max';
import { PageContainer } from '@ant-design/pro-components';
import { Collapse, Card, Button, Skeleton, Alert } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import { getConfig, reloadConfig } from '@/services/config';

export default function ConfigPage() {
  const access = useAccess();

  const { data: config, loading, error } = useRequest(getConfig, {
    refreshOnWindowFocus: false,
  });

  const { loading: reloading, run: runReload } = useRequest(reloadConfig, {
    manual: true,
    onSuccess: () => {
      // Optionally re-fetch the config after reload
    },
  });

  const collapseItems = config
    ? Object.entries(config).map(([section, conf]) => ({
        key: section,
        label: (
          <span
            style={{
              color: '#e0e0e0',
              fontWeight: 600,
              textTransform: 'uppercase',
              letterSpacing: 1,
            }}
          >
            [{section}]
          </span>
        ),
        children: (
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
            {JSON.stringify(conf, null, 2)}
          </pre>
        ),
      }))
    : [];

  return (
    <PageContainer
      title="Configuration"
      subTitle="Server configuration (read from config/default.toml)"
      extra={
        access?.canReloadConfig !== false && (
          <Button
            icon={<ReloadOutlined />}
            loading={reloading}
            onClick={() => runReload()}
            style={{ background: '#e05d10', borderColor: '#e05d10', color: '#fff' }}
          >
            Reload Config
          </Button>
        )
      }
    >
      {error && (
        <Alert
          type="error"
          message="Failed to load configuration"
          description={String(error)}
          style={{ marginBottom: 16 }}
        />
      )}

      <Skeleton loading={loading} active paragraph={{ rows: 8 }}>
        <Card
          style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
          bodyStyle={{ padding: 0 }}
        >
          {collapseItems.length > 0 ? (
            <Collapse
              items={collapseItems}
              defaultActiveKey={collapseItems[0]?.key ? [collapseItems[0].key] : []}
              style={{ background: 'transparent', border: 'none' }}
            />
          ) : (
            !loading && (
              <div style={{ padding: 32, color: '#a0a0b0', textAlign: 'center' }}>
                No configuration sections returned by the server.
              </div>
            )
          )}
        </Card>
      </Skeleton>
    </PageContainer>
  );
}
