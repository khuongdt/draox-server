import { Area } from '@ant-design/charts';

interface BandwidthDataPoint {
  timestamp: string;
  bytes_sent: number;
  bytes_received: number;
}

interface BandwidthChartProps {
  data: BandwidthDataPoint[];
  height?: number;
}

const formatBytes = (bytes: number): string => {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(1)} MB`;
  if (bytes >= 1_024) return `${(bytes / 1_024).toFixed(1)} KB`;
  return `${bytes} B`;
};

const BandwidthChart: React.FC<BandwidthChartProps> = ({ data, height = 300 }) => {
  let rangeMs = 0;
  if (data.length > 1) {
    const minTime = new Date(data[0].timestamp).getTime();
    const maxTime = new Date(data[data.length - 1].timestamp).getTime();
    rangeMs = maxTime - minTime;
  }

  // Transform data into two series for the chart
  const chartData = data.flatMap((d) => [
    { timestamp: d.timestamp, value: d.bytes_sent, series: 'Sent' },
    { timestamp: d.timestamp, value: d.bytes_received, series: 'Received' },
  ]);

  const formatTime = (v: string) => {
    const d = new Date(v);
    // rangeMs is in milliseconds. 86400000ms = 1 day, 300000ms = 5 minutes
    if (rangeMs > 86400000) {
      return `${d.getMonth() + 1}/${d.getDate()} ${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`;
    } else if (rangeMs > 300000) {
      return `${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`;
    } else {
      return `${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}:${String(d.getSeconds()).padStart(2, '0')}`;
    }
  };

  const config = {
    data: chartData,
    xField: 'timestamp',
    yField: 'value',
    seriesField: 'series',
    height,
    color: ['#e05d10', '#53c28b'],
    areaStyle: () => ({ fillOpacity: 0.15 }),
    smooth: true,
    legend: {
      itemName: { style: { fill: '#a0a0b0' } },
    },
    xAxis: {
      label: {
        style: { fill: '#a0a0b0', fontSize: 11 },
        formatter: formatTime,
      },
      line: { style: { stroke: '#2a2a4a' } },
      tickLine: { style: { stroke: '#2a2a4a' } },
      grid: { line: { style: { stroke: '#2a2a4a' } } },
    },
    yAxis: {
      label: {
        style: { fill: '#a0a0b0', fontSize: 11 },
        formatter: (v: number) => formatBytes(v),
      },
      line: { style: { stroke: '#2a2a4a' } },
      grid: { line: { stroke: '#2a2a4a' } },
    },
    tooltip: {
      formatter: (datum: { series: string; value: number }) => ({
        name: datum.series,
        value: formatBytes(datum.value),
      }),
    },
    theme: 'dark',
    background: 'transparent',
  };

  return <Area {...config} />;
};

export default BandwidthChart;
