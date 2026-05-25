import type { ClientState } from 'draox-sdk-web';

interface Props {
  state: ClientState;
}

const LABELS: Record<ClientState, string> = {
  connected:    'Connected',
  connecting:   'Connecting…',
  reconnecting: 'Reconnecting…',
  disconnected: 'Disconnected',
};

export default function StatusBar({ state }: Props) {
  return (
    <div className="status-bar">
      <div className={`status-dot ${state}`} />
      <span>{LABELS[state]}</span>
    </div>
  );
}
