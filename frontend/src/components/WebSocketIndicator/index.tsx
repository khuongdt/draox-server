import { Tooltip } from 'antd';

type WSStatus = 'connected' | 'connecting' | 'closed';

interface WebSocketIndicatorProps {
  stream: string;
  status?: WSStatus;
}

const STATUS_COLOR: Record<WSStatus, string> = {
  connected: '#53c28b',
  connecting: '#f5a623',
  closed: '#d32f2f',
};

const STATUS_LABEL: Record<WSStatus, string> = {
  connected: 'Connected',
  connecting: 'Connecting…',
  closed: 'Closed',
};

const WebSocketIndicator: React.FC<WebSocketIndicatorProps> = ({
  stream,
  status = 'closed',
}) => {
  const color = STATUS_COLOR[status];
  const label = STATUS_LABEL[status];

  return (
    <Tooltip title={`${stream}: ${label}`}>
      <span
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: 6,
          cursor: 'default',
        }}
      >
        <span
          style={{
            width: 10,
            height: 10,
            borderRadius: '50%',
            background: color,
            boxShadow: `0 0 6px ${color}`,
            display: 'inline-block',
            animation: status === 'connecting' ? 'pulse 1.2s infinite' : undefined,
          }}
        />
        <span style={{ color: '#a0a0b0', fontSize: 12 }}>{stream}</span>
      </span>
    </Tooltip>
  );
};

export default WebSocketIndicator;
