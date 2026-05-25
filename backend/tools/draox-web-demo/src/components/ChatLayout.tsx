import { useState, useEffect, useCallback, useRef } from 'react';
import type { MessageDto, ChannelDto } from 'draox-sdk-web';
import type { AppContext } from '../App.tsx';
import Sidebar from './Sidebar.tsx';
import MessageList from './MessageList.tsx';
import MessageInput from './MessageInput.tsx';
import StatusBar from './StatusBar.tsx';

interface Props {
  ctx:          AppContext;
  onDisconnect: () => void;
}

export default function ChatLayout({ ctx, onDisconnect }: Props) {
  const { client, messaging, username } = ctx;

  const [channels,       setChannels]       = useState<ChannelDto[]>([]);
  const [activeChannel,  setActiveChannel]  = useState('');
  const [messages,       setMessages]       = useState<MessageDto[]>([]);
  const [typingText,     setTypingText]     = useState('');
  const [clientState,    setClientState]    = useState(client.state);
  const typingClearRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Track connection state
  useEffect(() => {
    const handler = () => setClientState(client.state);
    client.on('stateChanged', handler);
    return () => client.off('stateChanged', handler);
  }, [client]);

  // Load channels on mount
  useEffect(() => {
    messaging.getChannels().then(list => {
      setChannels(list);
      if (list.length > 0 && !activeChannel) {
        setActiveChannel(list[0].id);
      }
    }).catch(() => {});
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Load history when active channel changes
  useEffect(() => {
    if (!activeChannel) return;
    messaging.getHistory(activeChannel, 50).then(r => {
      setMessages(r.messages);
    }).catch(() => {});
  }, [activeChannel, messaging]);

  // Subscribe to realtime events
  useEffect(() => {
    const unsubMessage = messaging.onMessage(e => {
      if (e.message.channel_id !== activeChannel) return;
      setMessages(prev => [...prev, e.message]);
    });

    const unsubTyping = messaging.onTyping(e => {
      if (e.channel_id !== activeChannel || e.user_id === username) return;
      if (!e.is_typing) {
        setTypingText('');
        return;
      }
      setTypingText(`${e.username} is typing…`);
      if (typingClearRef.current) clearTimeout(typingClearRef.current);
      typingClearRef.current = setTimeout(() => setTypingText(''), 4000);
    });

    const unsubDeleted = messaging.onMessageDeleted(e => {
      if (e.channel_id !== activeChannel) return;
      setMessages(prev => prev.filter(m => m.id !== e.message_id));
    });

    return () => {
      unsubMessage();
      unsubTyping();
      unsubDeleted();
    };
  }, [activeChannel, username, messaging]);

  const handleSelectChannel = useCallback((id: string) => {
    setActiveChannel(id);
    setMessages([]);
    setTypingText('');
  }, []);

  const handleDisconnect = useCallback(() => {
    client.disconnect('user_logout').catch(() => {});
    onDisconnect();
  }, [client, onDisconnect]);

  const activeChannelObj = channels.find(c => c.id === activeChannel);

  return (
    <div className="chat-layout">
      <Sidebar
        channels={channels}
        activeChannel={activeChannel}
        username={username}
        onSelectChannel={handleSelectChannel}
        onDisconnect={handleDisconnect}
      />

      <div className="chat-main">
        <div className="chat-header">
          <span className="chat-channel-name">
            {activeChannelObj ? `#${activeChannelObj.name}` : ''}
          </span>
          {activeChannelObj?.topic && (
            <span className="chat-channel-topic">{activeChannelObj.topic}</span>
          )}
          <div className="chat-header-right">
            <StatusBar state={clientState} />
          </div>
        </div>

        <MessageList
          messages={messages}
          username={username}
          typingText={typingText}
        />

        <MessageInput
          channel={activeChannel}
          messaging={messaging}
          onSent={() => {}}
          disabled={!activeChannel || clientState !== 'connected'}
        />
      </div>
    </div>
  );
}
