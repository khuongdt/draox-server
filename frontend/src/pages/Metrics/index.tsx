import { useEffect } from 'react';
import { useRequest, useModel } from '@umijs/max';
import { PageContainer } from '@ant-design/pro-components';
import { Row, Col, Card } from 'antd';
import { Line, Column } from '@ant-design/charts';
import DarkStatisticCard from '@/components/DarkStatisticCard';
import BandwidthChart from '@/components/BandwidthChart';
import { getMetrics } from '@/services/metrics';
import { wsManager } from '@/services/wsManager';
import { formatBytes } from '@/utils/formatters';

const chartAxisStyle = {
  label: { style: { fill: '#a0a0b0', fontSize: 11 } },
  line: { style: { stroke: '#2a2a4a' } },
  tickLine: { style: { stroke: '#2a2a4a' } },
  grid: { line: { style: { stroke: '#2a2a4a' } } },
};

export default function MetricsPage() {
  // ── Shared ring-buffer model ──────────────────────────────────────────────────
  const { snapshots, latest, addSnapshot } = useModel('metrics');

  // ── HTTP initial load ─────────────────────────────────────────────────────────
  useRequest(getMetrics, {
    onSuccess: (data) => addSnapshot(data),
  });

  // ── /ws/metrics — feed the ring buffer every ~5 s ────────────────────────────
  useEffect(() => {
    const unsub = wsManager.subscribe('metrics', (data) => {
      addSnapshot(data as API.MetricsSnapshot);
    });
    return unsub;
  }, [addSnapshot]);

  // ── Derived chart data from ring buffer ───────────────────────────────────────
  const validSnapshots: API.MetricsSnapshot[] =
    (snapshots as API.MetricsSnapshot[] | undefined)?.filter(
      (s: API.MetricsSnapshot) => s && s.timestamp,
    ) ?? [];

  const connOverTime = validSnapshots.map((s: API.MetricsSnapshot) => ({
    time: new Date(s.timestamp).toLocaleTimeString(),
    value: s.connections_active,
  }));

  const oneHourAgo = Date.now() - 3600_000;
  const bwData = validSnapshots
    .filter((s: API.MetricsSnapshot) => new Date(s.timestamp).getTime() >= oneHourAgo)
    .map((s: API.MetricsSnapshot) => ({
      timestamp: s.timestamp,
      bytes_sent: s.bytes_sent,
      bytes_received: s.bytes_received,
    }));

  // Requests vs Errors — last 12 snapshots as grouped bar
  const reqErrData = validSnapshots.slice(-12).flatMap((s: API.MetricsSnapshot) => [
    {
      interval: new Date(s.timestamp).toLocaleTimeString(),
      count: s.requests_total,
      type: 'Requests',
    },
    {
      interval: new Date(s.timestamp).toLocaleTimeString(),
      count: s.errors_total,
      type: 'Errors',
    },
  ]);

  // Error rate line — last 30 snapshots
  const errRateData = validSnapshots.map((s: API.MetricsSnapshot) => ({
    time: new Date(s.timestamp).toLocaleTimeString(),
    value: s.requests_total > 0 ? s.errors_total / s.requests_total : 0,
  }));

  return (
    <PageContainer title="Metrics" subTitle="Real-time server performance metrics">
      {/* Stats row */}
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={12} lg={4}>
          <DarkStatisticCard
            title="Active Connections"
            value={latest?.connections_active ?? 0}
            color="#53c28b"
          />
        </Col>
        <Col xs={12} lg={4}>
          <DarkStatisticCard
            title="Total Connections"
            value={latest?.connections_total ?? 0}
            color="#e0e0e0"
          />
        </Col>
        <Col xs={12} lg={4}>
          <DarkStatisticCard
            title="Bytes Received"
            value={latest ? formatBytes(latest.bytes_received) : '—'}
            color="#90caf9"
          />
        </Col>
        <Col xs={12} lg={4}>
          <DarkStatisticCard
            title="Bytes Sent"
            value={latest ? formatBytes(latest.bytes_sent) : '—'}
            color="#e05d10"
          />
        </Col>
        <Col xs={12} lg={4}>
          <DarkStatisticCard
            title="Total Requests"
            value={latest?.requests_total ?? 0}
            color="#ab47bc"
          />
        </Col>
        <Col xs={12} lg={4}>
          <DarkStatisticCard
            title="Total Errors"
            value={latest?.errors_total ?? 0}
            color="#d32f2f"
          />
        </Col>
      </Row>

      {/* Charts 2×2 grid */}
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={12}>
          <Card
            title={<span style={{ color: '#e0e0e0' }}>Active Connections (live)</span>}
            style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
            headStyle={{ borderBottom: '1px solid #2a2a4a' }}
          >
            {connOverTime.length > 1 ? (
              <Line
                data={connOverTime}
                xField="time"
                yField="value"
                height={240}
                color="#53c28b"
                smooth
                xAxis={chartAxisStyle}
                yAxis={chartAxisStyle}
                point={{ size: 0 }}
              />
            ) : (
              <div
                style={{
                  height: 240,
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
            title={<span style={{ color: '#e0e0e0' }}>Bandwidth (Sent / Received)</span>}
            style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
            headStyle={{ borderBottom: '1px solid #2a2a4a' }}
          >
            <BandwidthChart data={bwData} height={240} />
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card
            title={<span style={{ color: '#e0e0e0' }}>Requests vs Errors</span>}
            style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
            headStyle={{ borderBottom: '1px solid #2a2a4a' }}
          >
            {reqErrData.length > 0 ? (
              <Column
                data={reqErrData}
                xField="interval"
                yField="count"
                seriesField="type"
                isGroup
                height={240}
                color={['#e05d10', '#d32f2f']}
                xAxis={chartAxisStyle}
                yAxis={chartAxisStyle}
                legend={{ itemName: { style: { fill: '#a0a0b0' } } }}
              />
            ) : (
              <div
                style={{
                  height: 240,
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
            title={<span style={{ color: '#e0e0e0' }}>Error Rate (%)</span>}
            style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
            headStyle={{ borderBottom: '1px solid #2a2a4a' }}
          >
            {errRateData.length > 1 ? (
              <Line
                data={errRateData}
                xField="time"
                yField="value"
                height={240}
                color="#d32f2f"
                smooth
                xAxis={chartAxisStyle}
                yAxis={{
                  ...chartAxisStyle,
                  label: {
                    ...chartAxisStyle.label,
                    formatter: (v: string) => `${(parseFloat(v) * 100).toFixed(1)}%`,
                  },
                }}
                point={{ size: 0 }}
              />
            ) : (
              <div
                style={{
                  height: 240,
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
      </Row>
    </PageContainer>
  );
}
