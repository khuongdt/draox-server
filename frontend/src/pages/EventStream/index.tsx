import { useEffect } from 'react';
import { useModel } from '@umijs/max';
import { PageContainer } from '@ant-design/pro-components';
import { Card, Checkbox, Switch, Button, Space, Typography, Badge } from 'antd';
import { ClearOutlined, PauseCircleOutlined, PlayCircleOutlined } from '@ant-design/icons';
import { useState } from 'react';
import EventTimeline from '@/components/EventTimeline';
import { wsManager } from '@/services/wsManager';

const { Text } = Typography;

const ALL_CATEGORIES = ['connection', 'session', 'guard', 'plugin', 'server', 'custom'];

export default function EventStreamPage() {
  const [enabledCategories, setEnabledCategories] = useState<string[]>(ALL_CATEGORIES);
  const [autoScroll, setAutoScroll] = useState(true);

  // ── Shared FIFO events model ──────────────────────────────────────────────────
  const { events, paused, addEvent, clear, togglePause } = useModel('events');

  // ── /ws/events — feed the FIFO buffer ────────────────────────────────────────
  useEffect(() => {
    const unsub = wsManager.subscribe('events', (data) => {
      addEvent(data as API.ServerEvent);
    });
    return unsub;
  }, [addEvent]);

  // ── Filter displayed events by selected categories ────────────────────────────
  const displayedEvents = enabledCategories.length === ALL_CATEGORIES.length
    ? events
    : events.filter((e: API.ServerEvent) => enabledCategories.includes(e.category));

  return (
    <PageContainer title="Event Stream" subTitle="Real-time server event feed">
      {/* Toolbar */}
      <Card
        style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
        bodyStyle={{ padding: 16 }}
      >
        <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 16 }}>
          {/* Category filters */}
          <div>
            <Text style={{ color: '#a0a0b0', fontSize: 12, marginRight: 8 }}>
              Categories:
            </Text>
            <Checkbox.Group
              value={enabledCategories}
              onChange={(vals) => setEnabledCategories(vals as string[])}
            >
              <Space wrap>
                {ALL_CATEGORIES.map((cat) => (
                  <Checkbox key={cat} value={cat}>
                    <span style={{ color: '#e0e0e0', textTransform: 'capitalize', fontSize: 13 }}>
                      {cat}
                    </span>
                  </Checkbox>
                ))}
              </Space>
            </Checkbox.Group>
          </div>

          {/* Controls */}
          <Space style={{ marginLeft: 'auto' }}>
            <Space>
              <Text style={{ color: '#a0a0b0', fontSize: 12 }}>Auto-scroll</Text>
              <Switch checked={autoScroll} onChange={setAutoScroll} size="small" />
            </Space>
            <Button
              icon={paused ? <PlayCircleOutlined /> : <PauseCircleOutlined />}
              onClick={togglePause}
              style={{
                color: paused ? '#53c28b' : '#f5a623',
                borderColor: paused ? '#53c28b' : '#f5a623',
              }}
            >
              {paused ? 'Resume' : 'Pause'}
            </Button>
            <Button icon={<ClearOutlined />} onClick={clear} danger>
              Clear
            </Button>
          </Space>
        </div>
      </Card>

      {/* Status bar */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12 }}>
        <Badge status={paused ? 'warning' : 'success'} />
        <Text style={{ color: '#a0a0b0', fontSize: 12 }}>
          {paused ? 'Stream paused' : 'Stream live'} · {displayedEvents.length} events
          {enabledCategories.length < ALL_CATEGORIES.length &&
            ` (filtered from ${events.length})`}
        </Text>
      </div>

      {/* Event feed */}
      <Card
        style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
        bodyStyle={{ padding: 16 }}
      >
        {displayedEvents.length > 0 ? (
          <EventTimeline
            events={displayedEvents}
            maxEvents={200}
            filters={enabledCategories}
            paused={paused}
          />
        ) : (
          <div style={{ textAlign: 'center', padding: 40, color: '#a0a0b0' }}>
            {paused
              ? 'Stream is paused. Resume to see new events.'
              : 'No events yet. Waiting for the /ws/events stream…'}
          </div>
        )}
      </Card>
    </PageContainer>
  );
}
