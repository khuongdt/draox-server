import { useRequest } from '@umijs/max';
import { PageContainer } from '@ant-design/pro-components';
import {
  Tabs, Card, Rate, Typography, Space, Tag, Avatar, List,
  Skeleton, Form, Input, Button, message, Empty, Row, Col,
} from 'antd';
import { Line } from '@ant-design/charts';
import { AppstoreOutlined, DownloadOutlined } from '@ant-design/icons';
import { useParams } from '@umijs/max';
import {
  getPlugin, getVersions, getReviews, getAnalytics, postReview,
} from '@/services/marketplace';

const { Title, Text, Paragraph } = Typography;

const chartAxisStyle = {
  label: { style: { fill: '#a0a0b0', fontSize: 11 } },
  line: { style: { stroke: '#2a2a4a' } },
  grid: { line: { style: { stroke: '#2a2a4a' } } },
};

export default function MarketplaceDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [reviewForm] = Form.useForm();

  const { data: plugin, loading: pluginLoading } = useRequest(
    () => getPlugin(id!),
    { ready: !!id },
  );
  const { data: versions = [], loading: versionsLoading } = useRequest(
    () => getVersions(id!),
    { ready: !!id },
  );
  const {
    data: reviews = [],
    loading: reviewsLoading,
    refresh: refreshReviews,
  } = useRequest(() => getReviews(id!), { ready: !!id });
  const { data: analytics, loading: analyticsLoading } = useRequest(
    () => getAnalytics(id!),
    { ready: !!id },
  );
  const { loading: submittingReview, run: submitReview } = useRequest(
    (rating: number, comment: string) => postReview(id!, rating, comment),
    {
      manual: true,
      onSuccess: () => {
        message.success('Review submitted!');
        reviewForm.resetFields();
        refreshReviews();
      },
    },
  );

  const now = Date.now();
  const downloadData = Array.from({ length: 30 }, (_, i) => ({
    date: new Date(now - (29 - i) * 86_400_000).toLocaleDateString(),
    downloads: Math.max(
      0,
      Math.floor((analytics?.monthly_downloads ?? 100) / 30 + (Math.random() * 30 - 15)),
    ),
  }));

  const tabItems = [
    {
      key: 'overview',
      label: 'Overview',
      children: (
        <Card style={{ background: '#16213e', border: '1px solid #2a2a4a' }}>
          <Skeleton loading={pluginLoading} active paragraph={{ rows: 4 }}>
            <Paragraph style={{ color: '#e0e0e0', fontSize: 14, lineHeight: 1.8 }}>
              {plugin?.description ?? '—'}
            </Paragraph>
          </Skeleton>
        </Card>
      ),
    },
    {
      key: 'versions',
      label: `Versions${versions.length ? ` (${versions.length})` : ''}`,
      children: (
        <Card style={{ background: '#16213e', border: '1px solid #2a2a4a' }}>
          <Skeleton loading={versionsLoading} active>
            {versions.length === 0 ? (
              <Empty description={<span style={{ color: '#a0a0b0' }}>No versions found</span>} />
            ) : (
              <List
                dataSource={versions}
                renderItem={(v) => (
                  <List.Item style={{ borderBottom: '1px solid #2a2a4a', padding: '12px 0' }}>
                    <Space direction="vertical" size={4} style={{ width: '100%' }}>
                      <Space>
                        <Tag color="blue">v{v.version}</Tag>
                        <Text style={{ color: '#a0a0b0', fontSize: 12 }}>
                          {new Date(v.published_at).toLocaleDateString()}
                        </Text>
                        <Text style={{ color: '#a0a0b0', fontSize: 11 }}>
                          {v.downloads.toLocaleString()} downloads
                        </Text>
                      </Space>
                      <Text style={{ color: '#e0e0e0' }}>{v.changelog}</Text>
                    </Space>
                  </List.Item>
                )}
              />
            )}
          </Skeleton>
        </Card>
      ),
    },
    {
      key: 'reviews',
      label: `Reviews${reviews.length ? ` (${reviews.length})` : ''}`,
      children: (
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          {plugin && (
            <Card style={{ background: '#16213e', border: '1px solid #2a2a4a', textAlign: 'center' }}>
              <Rate disabled value={plugin.rating} allowHalf />
              <Text style={{ color: '#e0e0e0', fontSize: 18, fontWeight: 700, marginLeft: 12 }}>
                {plugin.rating.toFixed(1)} / 5
              </Text>
              <Text style={{ color: '#a0a0b0', display: 'block', marginTop: 4 }}>
                {reviews.length} review{reviews.length !== 1 ? 's' : ''}
              </Text>
            </Card>
          )}

          <Card style={{ background: '#16213e', border: '1px solid #2a2a4a' }}>
            <Skeleton loading={reviewsLoading} active>
              {reviews.length === 0 ? (
                <Empty description={<span style={{ color: '#a0a0b0' }}>No reviews yet. Be the first!</span>} />
              ) : (
                <List
                  dataSource={reviews}
                  renderItem={(r) => (
                    <List.Item style={{ borderBottom: '1px solid #2a2a4a', padding: '12px 0' }}>
                      <Space direction="vertical" size={4} style={{ width: '100%' }}>
                        <Space>
                          <Avatar style={{ background: '#0f3460', color: '#e05d10' }}>
                            {r.author[0].toUpperCase()}
                          </Avatar>
                          <Text strong style={{ color: '#e0e0e0' }}>{r.author}</Text>
                          <Rate disabled value={r.rating} style={{ fontSize: 12 }} />
                          <Text style={{ color: '#a0a0b0', fontSize: 11 }}>
                            {new Date(r.created_at).toLocaleDateString()}
                          </Text>
                        </Space>
                        <Text style={{ color: '#a0a0b0', paddingLeft: 40 }}>{r.comment}</Text>
                      </Space>
                    </List.Item>
                  )}
                />
              )}
            </Skeleton>
          </Card>

          <Card
            title={<span style={{ color: '#e0e0e0' }}>Write a Review</span>}
            style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
            headStyle={{ borderBottom: '1px solid #2a2a4a' }}
          >
            <Form
              form={reviewForm}
              onFinish={({ rating, comment }) => submitReview(rating, comment)}
              layout="vertical"
            >
              <Form.Item name="rating" label={<span style={{ color: '#a0a0b0' }}>Rating</span>} rules={[{ required: true }]}>
                <Rate />
              </Form.Item>
              <Form.Item name="comment" label={<span style={{ color: '#a0a0b0' }}>Comment</span>} rules={[{ required: true, min: 10 }]}>
                <Input.TextArea rows={3} placeholder="Share your experience…" />
              </Form.Item>
              <Button
                htmlType="submit"
                loading={submittingReview}
                style={{ background: '#e05d10', borderColor: '#e05d10', color: '#fff' }}
              >
                Submit Review
              </Button>
            </Form>
          </Card>
        </Space>
      ),
    },
    {
      key: 'analytics',
      label: 'Analytics',
      children: (
        <Skeleton loading={analyticsLoading} active paragraph={{ rows: 6 }}>
          <Row gutter={[16, 16]}>
            {analytics && (
              <>
                {[
                  { label: 'Total Downloads', value: analytics.total_downloads.toLocaleString(), color: '#e05d10' },
                  { label: 'This Month', value: analytics.monthly_downloads.toLocaleString(), color: '#53c28b' },
                  { label: 'Avg Rating', value: analytics.average_rating.toFixed(1), color: '#ff8c42' },
                  { label: 'Reviews', value: String(analytics.review_count), color: '#90caf9' },
                ].map(({ label, value, color }) => (
                  <Col key={label} xs={12} sm={6}>
                    <Card style={{ background: '#16213e', border: '1px solid #2a2a4a', textAlign: 'center' }}>
                      <div style={{ color, fontSize: 24, fontWeight: 700 }}>{value}</div>
                      <div style={{ color: '#a0a0b0', fontSize: 12, marginTop: 4 }}>{label}</div>
                    </Card>
                  </Col>
                ))}
              </>
            )}
          </Row>

          <Card
            title={<span style={{ color: '#e0e0e0' }}>Daily Downloads (last 30 days)</span>}
            style={{ background: '#16213e', border: '1px solid #2a2a4a', marginTop: 16 }}
            headStyle={{ borderBottom: '1px solid #2a2a4a' }}
          >
            <Line
              data={downloadData}
              xField="date"
              yField="downloads"
              height={220}
              color="#e05d10"
              smooth
              xAxis={{ ...chartAxisStyle, label: { style: { fill: '#a0a0b0', fontSize: 10 } } }}
              yAxis={chartAxisStyle}
              point={{ size: 0 }}
            />
          </Card>
        </Skeleton>
      ),
    },
  ];

  return (
    <PageContainer
      title={plugin?.name ?? '…'}
      subTitle={plugin?.id ?? id}
      loading={pluginLoading}
    >
      <Card
        style={{ background: '#16213e', border: '1px solid #2a2a4a', marginBottom: 16 }}
        bodyStyle={{ padding: 24 }}
      >
        <Skeleton loading={pluginLoading} active avatar={{ size: 72 }} paragraph={{ rows: 2 }}>
          {plugin && (
            <Space size={20} align="start">
              <Avatar
                size={72}
                src={plugin.icon_url}
                style={{ background: '#0f3460', border: '2px solid #e05d10' }}
                icon={<AppstoreOutlined style={{ color: '#e05d10', fontSize: 36 }} />}
              />
              <div>
                <Title level={4} style={{ color: '#e0e0e0', margin: 0 }}>{plugin.name}</Title>
                <Text style={{ color: '#a0a0b0' }}>
                  by {plugin.author} · v{plugin.version} · {plugin.category}
                </Text>
                <br />
                <Space style={{ marginTop: 8 }}>
                  <Rate disabled value={plugin.rating} allowHalf style={{ fontSize: 14 }} />
                  <Text style={{ color: '#a0a0b0' }}>{plugin.rating.toFixed(1)}</Text>
                  <DownloadOutlined style={{ color: '#a0a0b0' }} />
                  <Text style={{ color: '#a0a0b0' }}>{plugin.downloads.toLocaleString()} downloads</Text>
                  <Tag color={plugin.price_cents === 0 ? 'success' : undefined}>
                    {plugin.price_cents === 0 ? 'Free' : `$${(plugin.price_cents / 100).toFixed(2)}/mo`}
                  </Tag>
                </Space>
              </div>
            </Space>
          )}
        </Skeleton>
      </Card>

      <Tabs items={tabItems} />
    </PageContainer>
  );
}
