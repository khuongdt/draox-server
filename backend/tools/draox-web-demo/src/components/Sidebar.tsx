import type { ChannelDto } from 'draox-sdk-web';

interface Props {
  channels:        ChannelDto[];
  activeChannel:   string;
  username:        string;
  onSelectChannel: (id: string) => void;
  onDisconnect:    () => void;
}

export default function Sidebar({ channels, activeChannel, username, onSelectChannel, onDisconnect }: Props) {
  return (
    <div className="sidebar">
      <div className="sidebar-header">
        <span className="sidebar-title">Channels</span>
      </div>

      <ul className="channel-list">
        {channels.map(ch => (
          <li
            key={ch.id}
            className={`channel-item ${ch.id === activeChannel ? 'active' : ''}`}
            onClick={() => onSelectChannel(ch.id)}
          >
            <span className="channel-hash">#</span>
            <span className="channel-name">{ch.name}</span>
          </li>
        ))}
      </ul>

      <div className="sidebar-footer">
        <div className="user-info">
          <div className="user-avatar">{username[0]?.toUpperCase()}</div>
          <span className="user-name">{username}</span>
        </div>
        <button className="btn-disconnect" onClick={onDisconnect} title="Disconnect">
          ⏻
        </button>
      </div>
    </div>
  );
}
