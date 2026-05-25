import { useState, useRef, KeyboardEvent } from 'react';
import type { MessagingPlugin } from 'draox-sdk-web';

interface Props {
  channel:   string;
  messaging: MessagingPlugin;
  onSent:    () => void;
  disabled?: boolean;
}

export default function MessageInput({ channel, messaging, onSent, disabled }: Props) {
  const [text, setText]             = useState('');
  const [sending, setSending]       = useState(false);
  const typingTimerRef              = useRef<ReturnType<typeof setTimeout> | null>(null);

  const triggerTyping = () => {
    void messaging.sendTyping(channel);
    if (typingTimerRef.current) clearTimeout(typingTimerRef.current);
    typingTimerRef.current = setTimeout(() => { typingTimerRef.current = null; }, 3000);
  };

  const send = async () => {
    const trimmed = text.trim();
    if (!trimmed || sending) return;
    setSending(true);
    try {
      await messaging.sendMessage(channel, trimmed);
      setText('');
      onSent();
    } catch {
      // error silently — the parent can show a toast if needed
    } finally {
      setSending(false);
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void send();
      return;
    }
    triggerTyping();
  };

  return (
    <div className="input-area">
      <div className="input-row">
        <textarea
          className="msg-input"
          placeholder={`Message #${channel}`}
          value={text}
          rows={1}
          disabled={disabled || sending}
          onChange={e => setText(e.target.value)}
          onKeyDown={handleKeyDown}
        />
        <button
          className="send-btn"
          onClick={() => void send()}
          disabled={!text.trim() || sending || disabled}
        >
          Send
        </button>
      </div>
    </div>
  );
}
