import { useEffect, useCallback } from 'react';
import { useRequest, useModel } from '@umijs/max';
import { PageContainer } from '@ant-design/pro-components';
import { Row, Col, Card, Skeleton } from 'antd';
import { Line, Pie } from '@ant-design/charts';
import DarkStatisticCard from '@/components/DarkStatisticCard';
import HealthStatusBar from '@/components/HealthStatusBar';
import BandwidthChart from '@/components/BandwidthChart';
import EventTimeline from '@/components/EventTimeline';
import { getDetailedHealth } from '@/services/health';
import { getMetrics } from '@/services/metrics';
import { wsManager } from '@/services/wsManager';
import { formatDuration } from '@/utils/formatters';

const chartAxisStyle = {
  label: { style: { fill: '#a0a0b0', fontSize: 11 } },
  line: { style: { stroke: '#2a2a4a' } },
  tickLine: { style: { stroke: '#2a2a4a' } },
  grid: { line: { style: { stroke: '#2a2a4a' } } },
};

export default function DashboardPage() {
  // ── Models ──────────────────────────────────────────────────────────────────
  const { snapshots, latest, addSnapshot } = useModel('metrics');
  const { events, addEvent } = useModel('events');

  // ── HTTP initial load ────────────────────────────────────────────────────────
  const { data: health, loading: healthLoading } = useRequest(getDetailedHealth, {
    pollingInterval: 30_000,
  });

  const { data: initialMetrics } = useRequest(getMetrics, {
    onSuccess: (data) => {
      // Backend may wrap response in { success, data } envelope — unwrap if needed
      const snap = (data as any)?.data ?? data;
      if (snap?.timestamp) addSnapshot(snap as API.MetricsSnapshot);
    },
  });

  // ── WebSocket subscriptions ──────────────────────────────────────────────────
  useEffect(() => {
    const unsubMetrics = wsManager.subscribe('metrics', (raw) => {
      // WS frame may be { type, payload } or { success, data } — unwrap to inner snapshot
      const snap = (raw as any)?.data ?? (raw as any)?.payload ?? raw;
      if (snap?.timestamp) addSnapshot(snap as API.MetricsSnapshot);
    });
    const unsubEvents = wsManager.subscribe('events', (raw) => {
      const evt = (raw as any)?.data ?? (raw as any)?.payload ?? raw;
      if (evt?.timestamp) addEvent(evt as API.ServerEvent);
    });
    return () => {
      unsubMetrics();
      unsubEvents();
    };
  }, [addSnapshot, addEvent]);

  // ── Derived chart data from ring buffer ──────────────────────────────────────
  const connOverTime = (snapshots as API.MetricsSnapshot[])
    .filter((s: API.MetricsSnapshot) => s != null && s.timestamp != null)
    .map((s: API.MetricsSnapshot) => ({
      time: new Date(s.timestamp).toLocaleTimeString(),
      value: s.connections_active ?? 0,
    }));

  const oneHourAgo = Date.now() - 3600_000;
  const bwData = (snapshots as API.MetricsSnapshot[])
    .filter((s: API.MetricsSnapshot) => s != null && s.timestamp != null)
    .filter((s: API.MetricsSnapshot) => new Date(s.timestamp!).getTime() >= oneHourAgo)
    .map((s: API.MetricsSnapshot) => ({
      timestamp: s.timestamp,
      bytes_sent: s.bytes_sent ?? 0,
      bytes_received: s.bytes_received ?? 0,
    }));

  // Protocol distribution from latest snapshot (static pie while no detailed data)
  const pieData = [
    { type: 'TCP', value: 45 },
    { type: 'UDP', value: 25 },
    { type: 'WebSocket', value: 20 },
    { type: 'HTTP', value: 10 },
  ];

  // ── Health components for HealthStatusBar ───────────────────────────────────
  const healthComponents = health?.components
    ? Object.entries(health.components).map(([name, info]) => ({
        name,
        status: info.status as 'healthy' | 'degraded' | 'unhealthy' | 'unknown',
        message: info.message,
      }))
    : [
        { name: 'Socket Server', status: 'unknown' as const },
        { name: 'Connection Manager', status: 'unknown' as const },
        { name: 'Traffic Guard', status: 'unknown' as const },
        { name: 'Data Store', status: 'unknown' as const },
        { name: 'Cache Layer', status: 'unknown' as const },
      ];

  const stats = latest ?? initialMetrics;

  return (
    <PageContainer title="Dashboard" subTitle="System overview and real-time metrics">
      {/* Health bar */}
      <div style={{ marginBottom: 16 }}>
        <Skeleton loading={healthLoading} active paragraph={{ rows: 1 }}>
          <HealthStatusBar components={healthComponents} />
        </Skeleton>
      </div>

      {/* Statistic cards */}
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={24} sm={12} lg={6}>
          <DarkStatisticCard
            title="Active Connections"
            value={stats?.connections_active ?? 0}
            color="#53c28b"
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <DarkStatisticCard
            title="Total Connections"
            value={stats?.connections_total ?? 0}
            color="#ff8c42"
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <DarkStatisticCard
            title="Requests"
            value={stats?.requests_total ?? 0}
            color="#ab47bc"
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <DarkStatisticCard
            title="Errors"
            value={stats?.errors_total ?? 0}
            color="#d32f2f"
          />
        </Col>
      </Row>

      {/* Charts row 1 */}
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={24} lg={16}>
          <Card
            title={<span style={{ color: '#e0e0e0' }}>Bandwidth Usage (live)</span>}
            style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
            headStyle={{ borderBottom: '1px solid #2a2a4a' }}
          >
            <BandwidthChart
              data={bwData.length > 0 ? bwData : []}
              height={260}
            />
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card
            title={<span style={{ color: '#e0e0e0' }}>Protocol Distribution</span>}
            style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
            headStyle={{ borderBottom: '1px solid #2a2a4a' }}
          >
            <Pie
              data={pieData}
              angleField="value"
              colorField="type"
              height={260}
              color={['#90caf9', '#a5d6a7', '#ef9a9a', '#ff8a65']}
              legend={{ itemName: { style: { fill: '#a0a0b0' } } }}
              label={{
                content: ({ type, value }: { type: string; value: number }) =>
                  `${type}: ${value}%`,
                style: { fill: '#e0e0e0', fontSize: 12 },
              }}
            />
          </Card>
        </Col>
      </Row>

      {/* Charts row 2 */}
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={12}>
          <Card
            title={<span style={{ color: '#e0e0e0' }}>Connections Over Time (live)</span>}
            style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
            headStyle={{ borderBottom: '1px solid #2a2a4a' }}
          >
            {connOverTime.length > 1 ? (
              <Line
                data={connOverTime}
                xField="time"
                yField="value"
                height={260}
                color="#e05d10"
                smooth
                xAxis={chartAxisStyle}
                yAxis={chartAxisStyle}
                point={{ size: 0 }}
              />
            ) : (
              <div
                style={{
                  height: 260,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: '#a0a0b0',
                  fontSize: 13,
                }}
              >
                Waiting for metrics stream…
              </div>
            )}
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card
            title={<span style={{ color: '#e0e0e0' }}>Recent Events (live)</span>}
            style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
            headStyle={{ borderBottom: '1px solid #2a2a4a' }}
          >
            <EventTimeline events={events} maxEvents={10} />
          </Card>
        </Col>
      </Row>
    </PageContainer>
  );
}
