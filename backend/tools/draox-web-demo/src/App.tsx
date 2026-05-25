import { useState, useCallback } from 'react';
import { DraoxClient, MessagingPlugin } from 'draox-sdk-web';
import type { ChannelDto, MessageDto } from 'draox-sdk-web';
import LoginPanel from './components/LoginPanel.tsx';
import ChatLayout from './components/ChatLayout.tsx';

export interface AppContext {
  client:    DraoxClient;
  messaging: MessagingPlugin;
  username:  string;
}

export interface ChatState {
  channels:       ChannelDto[];
  currentChannel: string;
  messages:       MessageDto[];
  typingText:     string;
}

type Screen = 'login' | 'chat';

export default function App() {
  const [screen, setScreen] = useState<Screen>('login');
  const [ctx, setCtx]       = useState<AppContext | null>(null);

  const handleConnected = useCallback((context: AppContext) => {
    setCtx(context);
    setScreen('chat');
  }, []);

  const handleDisconnect = useCallback(() => {
    ctx?.client.disconnect();
    setCtx(null);
    setScreen('login');
  }, [ctx]);

  if (screen === 'login') {
    return <LoginPanel onConnected={handleConnected} />;
  }

  return (
    <ChatLayout
      ctx={ctx!}
      onDisconnect={handleDisconnect}
    />
  );
}
