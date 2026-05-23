import { Space, Tooltip } from 'antd';

type ComponentStatus = 'healthy' | 'degraded' | 'unhealthy' | 'unknown';

interface HealthComponent {
  name: string;
  status: ComponentStatus;
}

interface HealthStatusBarProps {
  components: HealthComponent[];
}

const STATUS_COLOR: Record<ComponentStatus, string> = {
  healthy: '#53c28b',
  degraded: '#f5a623',
  unhealthy: '#d32f2f',
  unknown: '#a0a0b0',
};

const STATUS_LABEL: Record<ComponentStatus, string> = {
  healthy: 'Healthy',
  degraded: 'Degraded',
  unhealthy: 'Unhealthy',
  unknown: 'Unknown',
};

const HealthStatusBar: React.FC<HealthStatusBarProps> = ({ components }) => {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        padding: '10px 16px',
        background: '#16213e',
        border: '1px solid #2a2a4a',
        borderRadius: 8,
      }}
    >
      <span style={{ color: '#a0a0b0', fontSize: 12, fontWeight: 600, whiteSpace: 'nowrap' }}>
        System Health
      </span>
      <Space size={12} wrap>
        {components.map((c) => {
          const color = STATUS_COLOR[c.status];
          return (
            <Tooltip key={c.name} title={`${c.name}: ${STATUS_LABEL[c.status]}`}>
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5, cursor: 'default' }}>
                <span
                  style={{
                    width: 10,
                    height: 10,
                    borderRadius: '50%',
                    background: color,
                    boxShadow: `0 0 5px ${color}`,
                    display: 'inline-block',
                    flexShrink: 0,
                  }}
                />
                <span style={{ color: '#e0e0e0', fontSize: 12 }}>{c.name}</span>
              </span>
            </Tooltip>
          );
        })}
      </Space>
    </div>
  );
};

export default HealthStatusBar;
