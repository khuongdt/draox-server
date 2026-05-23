import { Badge } from 'antd';

type PluginState = 'Installed' | 'ActiveEnabled' | 'ActiveDisabled' | 'Uninstalled';

interface PluginStatusBadgeProps {
  state: PluginState;
}

const STATE_MAP: Record<PluginState, { status: 'default' | 'success' | 'warning' | 'error'; text: string }> = {
  Installed: { status: 'default', text: 'Installed' },
  ActiveEnabled: { status: 'success', text: 'Active & Enabled' },
  ActiveDisabled: { status: 'warning', text: 'Active, Disabled' },
  Uninstalled: { status: 'error', text: 'Uninstalled' },
};

const PluginStatusBadge: React.FC<PluginStatusBadgeProps> = ({ state }) => {
  const { status, text } = STATE_MAP[state] ?? STATE_MAP.Uninstalled;
  return <Badge status={status} text={<span style={{ color: '#e0e0e0' }}>{text}</span>} />;
};

export default PluginStatusBadge;
