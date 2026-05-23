import { useRequest, history, useAccess } from '@umijs/max';
import { PageContainer, ProCard } from '@ant-design/pro-components';
import {
  Input, Select, Row, Col, Tag, Rate, Typography, Space,
  Tabs, Skeleton, Empty, Button,
} from 'antd';
import { DownloadOutlined, AppstoreOutlined, ThunderboltOutlined } from '@ant-design/icons';
import { useState } from 'react';
import { searchPlugins, getFeatured, getPopular } from '@/services/marketplace';

const { Text } = Typography;

const CATEGORIES = [
  { value: '', label: 'All Categories' },
  { value: 'analytics', label: 'Analytics' },
  { value: 'security', label: 'Security' },
  { value: 'messaging', label: 'Messaging' },
  { value: 'utilities', label: 'Utilities' },
  { value: 'monitoring', label: 'Monitoring' },
];

function PluginCard({ plugin }: { plugin: API.MarketplacePlugin }) {
  return (
    <ProCard
      hoverable
      onClick={() => history.push(`/marketplace/${plugin.id}`)}
      style={{
        background: '#16213e',
        border: '1px solid #2a2a4a',
        borderRadius: 10,
        height: '100%',
        cursor: 'pointer',
        transition: 'border-color 0.2s',
      }}
      bodyStyle={{ padding: 20 }}
    >
      <div
        style={{
          width: 48,
          height: 48,
          background: '#0f3460',
          border: '1px solid #2a2a4a',
          borderRadius: 10,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          marginBottom: 12,
        }}
      >
        {plugin.icon_url ? (
          <img src={plugin.icon_url} alt={plugin.name} style={{ width: 36, height: 36, borderRadius: 6 }} />
        ) : (
          <AppstoreOutlined style={{ fontSize: 24, color: '#e05d10' }} />
        )}
      </div>

      <Text strong style={{ color: '#e0e0e0', fontSize: 15, display: 'block' }}>
        {plugin.name}
      </Text>
      <Text style={{ color: '#a0a0b0', fontSize: 12, display: 'block', marginBottom: 6 }}>
        by {plugin.author}
      </Text>
      <Text
        ellipsis
        style={{ color: '#a0a0b0', fontSize: 13, display: 'block', marginBottom: 12 }}
      >
        {plugin.description}
      </Text>

      <Tag
        style={{
          color: '#a0a0b0',
          background: '#0f3460',
          border: '1px solid #2a2a4a',
          fontSize: 11,
          marginBottom: 12,
          display: 'inline-block',
        }}
      >
        {plugin.category}
      </Tag>

      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div>
          <Rate disabled value={plugin.rating} allowHalf style={{ fontSize: 13 }} />
          <Text style={{ color: '#a0a0b0', fontSize: 11, marginLeft: 4 }}>
            {plugin.rating.toFixed(1)}
          </Text>
        </div>
        <Space size={4}>
          <DownloadOutlined style={{ color: '#a0a0b0', fontSize: 11 }} />
          <Text style={{ color: '#a0a0b0', fontSize: 11 }}>
            {plugin.downloads.toLocaleString()}
          </Text>
        </Space>
        <Tag
          color={plugin.price_cents === 0 ? 'success' : undefined}
          style={
            plugin.price_cents > 0
              ? { color: '#ff8c42', background: '#3e2000', border: '1px solid #ff8c4244' }
              : {}
          }
        >
          {plugin.price_cents === 0
            ? 'Free'
            : `$${(plugin.price_cents / 100).toFixed(2)}`}
        </Tag>
      </div>
    </ProCard>
  );
}

function PluginGrid({
  plugins,
  loading,
  emptyText,
}: {
  plugins: API.MarketplacePlugin[];
  loading: boolean;
  emptyText?: string;
}) {
  if (loading) {
    return (
      <Row gutter={[16, 16]}>
        {Array.from({ length: 6 }).map((_, i) => (
          <Col key={i} xs={24} sm={12} lg={8}>
            <ProCard style={{ background: '#16213e', border: '1px solid #2a2a4a' }}>
              <Skeleton active avatar paragraph={{ rows: 3 }} />
            </ProCard>
          </Col>
        ))}
      </Row>
    );
  }
  if (plugins.length === 0) {
    return (
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description={<span style={{ color: '#a0a0b0' }}>{emptyText ?? 'No plugins found'}</span>}
        style={{ padding: '60px 0' }}
      />
    );
  }
  return (
    <Row gutter={[16, 16]}>
      {plugins.map((p) => (
        <Col key={p.id} xs={24} sm={12} lg={8}>
          <PluginCard plugin={p} />
        </Col>
      ))}
    </Row>
  );
}

export default function MarketplacePage() {
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState('');
  const access = useAccess();

  const {
    data: searchResults = [],
    loading: searchLoading,
    run: runSearch,
  } = useRequest(
    () => searchPlugins({ q: search || undefined, category: category || undefined }),
    { refreshDeps: [search, category], debounceWait: 400 },
  );

  const { data: featured = [], loading: featuredLoading } = useRequest(getFeatured);
  const { data: popular = [], loading: popularLoading } = useRequest(getPopular);

  const tabItems = [
    {
      key: 'search',
      label: 'Search',
      children: (
        <PluginGrid
          plugins={searchResults}
          loading={searchLoading}
          emptyText="No plugins match your search"
        />
      ),
    },
    {
      key: 'featured',
      label: (
        <span>
          <ThunderboltOutlined style={{ marginRight: 4 }} />
          Featured
        </span>
      ),
      children: (
        <PluginGrid
          plugins={featured}
          loading={featuredLoading}
          emptyText="No featured plugins available"
        />
      ),
    },
    {
      key: 'popular',
      label: 'Most Popular',
      children: (
        <PluginGrid
          plugins={popular}
          loading={popularLoading}
          emptyText="No popular plugins available"
        />
      ),
    },
  ];

  return (
    <PageContainer
      title="Marketplace"
      subTitle="Discover and install WASM plugins"
      extra={
        access?.canPublishPlugin && (
          <Button
            type="primary"
            onClick={() => history.push('/marketplace/publish')}
            style={{ background: '#e05d10', borderColor: '#e05d10', fontWeight: 600 }}
          >
            Publish Plugin
          </Button>
        )
      }
    >
      <Space style={{ marginBottom: 20 }} size={12} wrap>
        <Input.Search
          placeholder="Search plugins…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onSearch={() => runSearch()}
          style={{ width: 300 }}
          allowClear
        />
        <Select
          value={category}
          onChange={setCategory}
          options={CATEGORIES}
          style={{ width: 180 }}
          placeholder="All categories"
        />
      </Space>

      <Tabs defaultActiveKey="search" items={tabItems} />
    </PageContainer>
  );
}
