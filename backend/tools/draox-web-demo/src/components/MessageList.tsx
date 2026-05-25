import { useEffect, useRef } from 'react';
import type { MessageDto } from 'draox-sdk-web';
import MessageBubble from './MessageBubble.tsx';

interface Props {
  messages:    MessageDto[];
  username:    string;
  typingText:  string;
}

export default function MessageList({ messages, username, typingText }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, typingText]);

  return (
    <>
      <div className="messages-area">
        {messages.map((msg, i) => {
          const prev = messages[i - 1];
          const showMeta = !prev || prev.sender_id !== msg.sender_id;
          return (
            <MessageBubble
              key={msg.id}
              message={msg}
              isOwn={msg.sender_id === username}
              showMeta={showMeta}
            />
          );
        })}
        <div ref={bottomRef} />
      </div>
      <div className="typing-indicator">{typingText}</div>
    </>
  );
}
