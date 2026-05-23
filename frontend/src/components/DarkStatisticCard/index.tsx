import { Card, Statistic } from 'antd';

interface DarkStatisticCardProps {
  title: string;
  value: number | string;
  suffix?: string;
  precision?: number;
  color?: string;
  loading?: boolean;
}

const DarkStatisticCard: React.FC<DarkStatisticCardProps> = ({
  title,
  value,
  suffix,
  precision,
  color = '#e0e0e0',
  loading = false,
}) => {
  return (
    <Card
      loading={loading}
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
        precision={precision}
        valueStyle={{ color, fontSize: 28, fontWeight: 700 }}
      />
    </Card>
  );
};

export default DarkStatisticCard;
