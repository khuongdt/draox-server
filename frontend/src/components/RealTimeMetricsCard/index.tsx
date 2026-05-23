import { Card, Statistic } from 'antd';

type TrendDirection = 'up' | 'down' | 'flat';

interface RealTimeMetricsCardProps {
  title: string;
  value: number | string;
  suffix?: string;
  trend?: TrendDirection;
  trendValue?: string;
  color?: string;
}

const TREND_MAP: Record<TrendDirection, { symbol: string; color: string }> = {
  up: { symbol: '▲', color: '#53c28b' },
  down: { symbol: '▼', color: '#d32f2f' },
  flat: { symbol: '─', color: '#a0a0b0' },
};

const RealTimeMetricsCard: React.FC<RealTimeMetricsCardProps> = ({
  title,
  value,
  suffix,
  trend,
  trendValue,
  color = '#e0e0e0',
}) => {
  const trendInfo = trend ? TREND_MAP[trend] : null;

  return (
    <Card
      style={{
        background: '#16213e',
        border: '1px solid #2a2a4a',
        borderRadius: 8,
      }}
      bodyStyle={{ padding: '20px 24px' }}
    >
      <Statistic
        title={<span style={{ color: '#a0a0b0', fontSize: 13 }}>{title}</span>}
        value={value}
        suffix={suffix}
        valueStyle={{ color, fontSize: 28, fontWeight: 700 }}
      />
      {trendInfo && trendValue && (
        <div style={{ marginTop: 6 }}>
          <span style={{ color: trendInfo.color, fontSize: 13, fontWeight: 600 }}>
            {trendInfo.symbol} {trendValue}
          </span>
          <span style={{ color: '#a0a0b0', fontSize: 12, marginLeft: 6 }}>vs last period</span>
        </div>
      )}
    </Card>
  );
};

export default RealTimeMetricsCard;
