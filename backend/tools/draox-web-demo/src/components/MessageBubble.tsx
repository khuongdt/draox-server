import type { MessageDto } from 'draox-sdk-web';

interface Props {
  message:    MessageDto;
  isOwn:      boolean;
  showMeta:   boolean;
}

function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

export default function MessageBubble({ message, isOwn, showMeta }: Props) {
  return (
    <div className={`msg-group ${isOwn ? 'own' : 'other'}`}>
      {showMeta && (
        <div className="msg-meta">
          <span className="msg-sender">{message.sender_id}</span>
          <span className="msg-time">{formatTime(message.sent_at)}</span>
        </div>
      )}
      <div className="msg-bubble">
        {message.text}
        {message.edited_at && <span className="msg-edited">(edited)</span>}
      </div>
    </div>
  );
}
