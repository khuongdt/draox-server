import { Timeline, Tag, Typography } from 'antd';

const { Text } = Typography;

interface TimelineEvent {
  type: string;
  data: unknown;
  timestamp: string;
  category: string;
}

interface EventTimelineProps {
  events: TimelineEvent[];
  maxEvents?: number;
  filters?: string[];
  paused?: boolean;
}

const CATEGORY_COLOR: Record<string, string> = {
  connection: '#42a5f5',
  session: '#26c6da',
  guard: '#ef5350',
  plugin: '#ab47bc',
  server: '#ff7043',
  custom: '#78909c',
};

const EventTimeline: React.FC<EventTimelineProps> = ({
  events,
  maxEvents = 100,
  filters,
  paused: _paused,
}) => {
  // Filter by category if filters provided
  const filtered = filters && filters.length > 0
    ? events.filter((e) => filters.includes(e.category))
    : events;

  // Limit display count
  const displayed = filtered.slice(0, maxEvents);

  const items = displayed.map((evt, idx) => {
    const color = CATEGORY_COLOR[evt.category] ?? CATEGORY_COLOR.custom;
    const dataStr = typeof evt.data === 'string'
      ? evt.data
      : JSON.stringify(evt.data);
    const truncated = dataStr.length > 80 ? `${dataStr.slice(0, 80)}…` : dataStr;

    return {
      key: idx,
      color,
      children: (
        <div style={{ marginBottom: 4 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
            <Tag
              style={{
                background: color + '22',
                borderColor: color,
                color,
                fontSize: 11,
                lineHeight: '18px',
              }}
            >
              {evt.category}
            </Tag>
            <Text strong style={{ color: '#e0e0e0', fontSize: 13 }}>
              {evt.type}
            </Text>
            <Text style={{ color: '#a0a0b0', fontSize: 11 }}>
              {new Date(evt.timestamp).toLocaleTimeString()}
            </Text>
          </div>
          {truncated && (
            <Text
              style={{
                color: '#a0a0b0',
                fontSize: 11,
                fontFamily: 'monospace',
                display: 'block',
                marginTop: 2,
              }}
            >
              {truncated}
            </Text>
          )}
        </div>
      ),
    };
  });

  return (
    <div
      style={{
        maxHeight: 420,
        overflowY: 'auto',
        padding: '8px 4px',
      }}
    >
      <Timeline items={items} />
    </div>
  );
};

export default EventTimeline;
