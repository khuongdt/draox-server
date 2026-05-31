import { useRequest, useAccess } from '@umijs/max';
import { PageContainer } from '@ant-design/pro-components';
import { Row, Col, Card, Badge, Button, Space, Typography, Skeleton, Alert } from 'antd';
import { DeleteOutlined } from '@ant-design/icons';
import { useState } from 'react';
import DarkStatisticCard from '@/components/DarkStatisticCard';
import ConfirmActionModal from '@/components/ConfirmActionModal';
import { getCacheStats, getCacheHealth, flushCache } from '@/services/cache';
import { formatBytes } from '@/utils/formatters';

const { Text } = Typography;

export default function CachePage() {
  const [flushVisible, setFlushVisible] = useState(false);
  const access = useAccess();

  const { data: stats, loading: statsLoading, refresh: refreshStats } = useRequest(getCacheStats, {
    refreshOnWindowFocus: false,
    pollingInterval: 10_000,
  });

  const { data: health, loading: healthLoading } = useRequest(getCacheHealth, {
    refreshOnWindowFocus: false,
    pollingInterval: 15_000,
  });

  const { loading: flushing, run: runFlush } = useRequest(flushCache, {
    manual: true,
    onSuccess: () => {
      setFlushVisible(false);
      refreshStats();
    },
  });

  const healthStatus = health?.status === 'healthy' ? 'success' : 'error';

  return (
    <PageContainer title="Cache Layer" subTitle="Redis and in-memory cache monitoring">
      {/* Stats row */}
      <Skeleton loading={statsLoading} active paragraph={{ rows: 2 }}>
        <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
          <Col xs={24} sm={6}>
            <DarkStatisticCard
              title="Hit Rate"
              value={stats ? (stats.hit_rate * 100).toFixed(1) : '—'}
              suffix="%"
              color="#53c28b"
            />
          </Col>
          <Col xs={24} sm={6}>
            <DarkStatisticCard
              title="Memory Used"
              value={stats ? formatBytes(stats.memory_bytes) : '—'}
              color="#ff8c42"
            />
          </Col>
          <Col xs={24} sm={6}>
            <DarkStatisticCard
              title="Total Keys"
              value={stats?.keys ?? 0}
              color="#e0e0e0"
            />
          </Col>
          <Col xs={24} sm={6}>
            <DarkStatisticCard
              title="Cache Hits"
              value={stats?.hits ?? 0}
              color="#90caf9"
            />
          </Col>
        </Row>
      </Skeleton>

      {/* Health card */}
      <Card
        title={<span style={{ color: '#e0e0e0' }}>Cache Health</span>}
        style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
        headStyle={{ borderBottom: '1px solid #2a2a4a' }}
      >
        <Skeleton loading={healthLoading} active paragraph={{ rows: 1 }}>
          <Space size={32}>
            <Badge
              status={healthStatus as 'success' | 'error'}
              text={
                <span style={{ color: healthStatus === 'success' ? '#53c28b' : '#d32f2f', fontWeight: 600 }}>
                  {health?.status ?? 'Unknown'}
                </span>
              }
            />
            <span>
              <Text style={{ color: '#a0a0b0' }}>Latency: </Text>
              <Text style={{ color: '#e0e0e0', fontWeight: 600 }}>
                {health ? `${health.latency_ms} ms` : '—'}
              </Text>
            </span>
            <span>
              <Text style={{ color: '#a0a0b0' }}>Backend: </Text>
              <Text style={{ color: '#e0e0e0' }}>{stats?.backend ?? '—'}</Text>
            </span>
          </Space>
        </Skeleton>
      </Card>

      {/* Flush action */}
      {access?.canCacheFlush !== false && (
        <Card
          title={<span style={{ color: '#d32f2f' }}>Danger Zone</span>}
          style={{ background: '#16213e', border: '1px solid #d32f2f' }}
          headStyle={{ borderBottom: '1px solid #4a1010' }}
        >
          <Button
            danger
            type="primary"
            icon={<DeleteOutlined />}
            loading={flushing}
            onClick={() => setFlushVisible(true)}
          >
            Flush Cache
          </Button>
          <Text style={{ color: '#a0a0b0', marginLeft: 12, fontSize: 13 }}>
            This will clear all Redis and in-memory cache entries. Services will experience a
            temporary slowdown.
          </Text>
        </Card>
      )}

      <ConfirmActionModal
        visible={flushVisible}
        title="Flush All Cache"
        description="All cached data (Redis + in-memory) will be purged. Applications will temporarily serve uncached data, causing higher database load."
        type="danger"
        confirmText="Flush Cache"
        onConfirm={async () => { await runFlush(); }}
        onCancel={() => setFlushVisible(false)}
      />
    </PageContainer>
  );
}
