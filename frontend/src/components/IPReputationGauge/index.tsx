import { Gauge } from '@ant-design/charts';
import { Spin, Typography } from 'antd';

const { Text } = Typography;

interface IPReputationGaugeProps {
  score: number;
  loading?: boolean;
}

const getRiskLabel = (score: number): { label: string; color: string } => {
  if (score <= 30) return { label: 'Low Risk', color: '#53c28b' };
  if (score <= 60) return { label: 'Medium Risk', color: '#f5a623' };
  if (score <= 80) return { label: 'High Risk', color: '#ff7043' };
  return { label: 'Critical Risk', color: '#d32f2f' };
};

const IPReputationGauge: React.FC<IPReputationGaugeProps> = ({ score, loading = false }) => {
  const { label, color } = getRiskLabel(score);

  const config = {
    percent: score / 100,
    height: 200,
    range: {
      ticks: [0, 0.3, 0.6, 0.8, 1],
      color: ['#53c28b', '#f5a623', '#ff7043', '#d32f2f'],
    },
    indicator: {
      pointer: { style: { stroke: '#e0e0e0' } },
      pin: { style: { stroke: '#e0e0e0' } },
    },
    statistic: {
      content: {
        formatter: () => `${score}`,
        style: { color: '#e0e0e0', fontSize: 28, fontWeight: 700 },
      },
      title: {
        formatter: () => 'Score',
        style: { color: '#a0a0b0', fontSize: 12 },
      },
    },
    gaugeStyle: { lineCap: 'round' },
  };

  if (loading) {
    return (
      <div style={{ height: 200, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Spin />
      </div>
    );
  }

  return (
    <div
      style={{
        background: '#16213e',
        border: '1px solid #2a2a4a',
        borderRadius: 8,
        padding: 16,
        textAlign: 'center',
      }}
    >
      <Gauge {...config} />
      <div style={{ marginTop: 8 }}>
        <Text style={{ color, fontSize: 16, fontWeight: 700 }}>{label}</Text>
        <br />
        <Text style={{ color: '#a0a0b0', fontSize: 12 }}>Reputation Score: {score}/100</Text>
      </div>
    </div>
  );
};

export default IPReputationGauge;
