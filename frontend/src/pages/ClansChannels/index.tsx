import { PageContainer, ProTable } from '@ant-design/pro-components';
import type { ProColumns } from '@ant-design/pro-components';
import {
  Button,
  Form,
  Input,
  message,
  Modal,
  Popconfirm,
  Space,
  Spin,
  Tabs,
  Tag,
} from 'antd';
import { useCallback, useEffect, useState } from 'react';
import {
  createChannel,
  deleteChannel,
  freezeChannel,
  listChannels,
} from '@/services/channels';
import {
  createClan,
  deleteClan,
  freezeClan,
  listClans,
} from '@/services/clans';

type CreateMode = 'channel' | 'clan' | null;

export default function ClansChannelsPage() {
  const [channels, setChannels] = useState<API.Channel[]>([]);
  const [clans, setClans]       = useState<API.Clan[]>([]);
  const [loading, setLoading]   = useState(false);

  const refresh = useCallback(() => {
    setLoading(true);
    Promise.all([listChannels(), listClans()])
      .then(([c, k]) => {
        setChannels(c);
        setClans(k);
      })
      .catch((e: Error) => message.error(`Failed to load: ${e.message}`))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // ── Create modal ───────────────────────────────────────────────────────
  const [createMode, setCreateMode] = useState<CreateMode>(null);
  const [submitting, setSubmitting] = useState(false);
  const [form] = Form.useForm();

  const openCreate = (mode: 'channel' | 'clan') => {
    setCreateMode(mode);
    form.resetFields();
  };

  const handleCreate = async () => {
    try {
      const values = await form.validateFields();
      setSubmitting(true);
      if (createMode === 'channel') {
        await createChannel({ name: values.name, description: values.description });
        message.success(`Channel '${values.name}' created`);
      } else if (createMode === 'clan') {
        await createClan({ name: values.name, tag: values.tag });
        message.success(`Clan '${values.name}' created`);
      }
      setCreateMode(null);
      refresh();
    } catch {
      // validation or network — interceptor reports it
    } finally {
      setSubmitting(false);
    }
  };

  // ── Channel actions ────────────────────────────────────────────────────
  const handleChannelFreeze = async (ch: API.Channel) => {
    await freezeChannel(ch.id, !ch.frozen);
    message.success(`Channel ${ch.frozen ? 'unfrozen' : 'frozen'}`);
    refresh();
  };
  const handleChannelDelete = async (ch: API.Channel) => {
    await deleteChannel(ch.id);
    message.success(`Channel '${ch.name}' deleted`);
    refresh();
  };

  const channelColumns: ProColumns<API.Channel>[] = [
    {
      title: 'Name',
      dataIndex: 'name',
      render: (_dom, r) => (
        <Space>
          <span style={{ fontWeight: 600 }}>{r.name}</span>
          <span style={{ color: '#888', fontFamily: 'monospace', fontSize: 12 }}>#{r.id}</span>
        </Space>
      ),
    },
    { title: 'Type',  dataIndex: 'channel_type', render: (_dom, r) => <Tag>{r.channel_type}</Tag> },
    { title: 'Owner', dataIndex: 'created_by',   render: (_dom, r) => <code>{r.created_by}</code> },
    {
      title: 'Members',
      dataIndex: 'member_count',
      align: 'center' as const,
      width: 90,
    },
    {
      title: 'Status',
      key: 'status',
      render: (_dom, r) => (
        <Space>
          {r.is_system && <Tag color="orange">SYSTEM</Tag>}
          {r.frozen && <Tag color="red">FROZEN</Tag>}
          {!r.is_system && !r.frozen && <Tag color="green">ACTIVE</Tag>}
        </Space>
      ),
    },
    {
      title: 'Created',
      dataIndex: 'created_at',
      render: (_dom, r) => <span style={{ color: '#888' }}>{new Date(r.created_at).toLocaleString()}</span>,
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_dom, r) => (
        <Space>
          <Popconfirm
            title={r.frozen ? `Unfreeze '${r.name}'?` : `Freeze '${r.name}'?`}
            description={r.frozen ? 'Members can send messages again.' : 'New messages and new subscriptions will be rejected.'}
            onConfirm={() => handleChannelFreeze(r)}
            okText={r.frozen ? 'Unfreeze' : 'Freeze'}
          >
            <Button size="small">{r.frozen ? 'Unfreeze' : 'Freeze'}</Button>
          </Popconfirm>
          {!r.is_system && (
            <Popconfirm
              title={`Delete channel '${r.name}'?`}
              description="This action cannot be undone."
              onConfirm={() => handleChannelDelete(r)}
              okText="Delete"
              okButtonProps={{ danger: true }}
            >
              <Button size="small" danger>Delete</Button>
            </Popconfirm>
          )}
        </Space>
      ),
    },
  ];

  // ── Clan actions ───────────────────────────────────────────────────────
  const handleClanFreeze = async (cl: API.Clan) => {
    await freezeClan(cl.id, !cl.frozen);
    message.success(`Clan ${cl.frozen ? 'unfrozen' : 'frozen'}`);
    refresh();
  };
  const handleClanDelete = async (cl: API.Clan) => {
    await deleteClan(cl.id);
    message.success(`Clan '${cl.name}' deleted`);
    refresh();
  };

  const clanColumns: ProColumns<API.Clan>[] = [
    {
      title: 'Name',
      dataIndex: 'name',
      render: (_dom, r) => (
        <Space>
          <span style={{ fontWeight: 600 }}>{r.name}</span>
          <Tag color="blue">{r.tag}</Tag>
        </Space>
      ),
    },
    { title: 'Owner', dataIndex: 'owner_id', render: (_dom, r) => <code>{r.owner_id}</code> },
    {
      title: 'Members',
      key: 'members',
      align: 'center' as const,
      width: 110,
      render: (_dom, r) => `${r.member_count}/${r.max_members}`,
    },
    {
      title: 'Status',
      key: 'status',
      render: (_dom, r) => (
        <Space>
          {r.is_system && <Tag color="orange">SYSTEM</Tag>}
          {r.frozen && <Tag color="red">FROZEN</Tag>}
          {!r.is_system && !r.frozen && <Tag color="green">ACTIVE</Tag>}
        </Space>
      ),
    },
    {
      title: 'Created',
      dataIndex: 'created_at',
      render: (_dom, r) => <span style={{ color: '#888' }}>{new Date(r.created_at).toLocaleString()}</span>,
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_dom, r) => (
        <Space>
          <Popconfirm
            title={r.frozen ? `Unfreeze '${r.name}'?` : `Freeze '${r.name}'?`}
            description={r.frozen ? 'New join requests will be accepted again.' : 'New join requests will be rejected.'}
            onConfirm={() => handleClanFreeze(r)}
            okText={r.frozen ? 'Unfreeze' : 'Freeze'}
          >
            <Button size="small">{r.frozen ? 'Unfreeze' : 'Freeze'}</Button>
          </Popconfirm>
          {!r.is_system && (
            <Popconfirm
              title={`Delete clan '${r.name}'?`}
              description="This action cannot be undone."
              onConfirm={() => handleClanDelete(r)}
              okText="Delete"
              okButtonProps={{ danger: true }}
            >
              <Button size="small" danger>Delete</Button>
            </Popconfirm>
          )}
        </Space>
      ),
    },
  ];

  return (
    <PageContainer
      title="Clans & Channels"
      subTitle="Manage messaging channels and clans across the server"
    >
      <Spin spinning={loading}>
        <Tabs
          defaultActiveKey="channels"
          items={[
            {
              key: 'channels',
              label: `Channels (${channels.length})`,
              children: (
                <ProTable<API.Channel>
                  columns={channelColumns}
                  dataSource={channels}
                  rowKey="id"
                  search={false}
                  options={{ reload: () => refresh() }}
                  pagination={{ pageSize: 20 }}
                  toolBarRender={() => [
                    <Button type="primary" key="add" onClick={() => openCreate('channel')}>
                      + New Channel
                    </Button>,
                  ]}
                />
              ),
            },
            {
              key: 'clans',
              label: `Clans (${clans.length})`,
              children: (
                <ProTable<API.Clan>
                  columns={clanColumns}
                  dataSource={clans}
                  rowKey="id"
                  search={false}
                  options={{ reload: () => refresh() }}
                  pagination={{ pageSize: 20 }}
                  toolBarRender={() => [
                    <Button type="primary" key="add" onClick={() => openCreate('clan')}>
                      + New Clan
                    </Button>,
                  ]}
                />
              ),
            },
          ]}
        />
      </Spin>

      <Modal
        title={createMode === 'channel' ? 'New Channel' : 'New Clan'}
        open={createMode !== null}
        onOk={handleCreate}
        onCancel={() => setCreateMode(null)}
        confirmLoading={submitting}
        okText="Create"
        destroyOnClose
      >
        <Form form={form} layout="vertical" style={{ marginTop: 16 }}>
          <Form.Item
            name="name"
            label="Name"
            rules={[{ required: true, message: 'Please enter a name' }]}
          >
            <Input placeholder={createMode === 'channel' ? 'e.g. general' : 'e.g. Knights of Draox'} />
          </Form.Item>

          {createMode === 'channel' && (
            <Form.Item name="description" label="Description (optional)">
              <Input.TextArea rows={3} placeholder="Describe the purpose of this channel" />
            </Form.Item>
          )}

          {createMode === 'clan' && (
            <Form.Item
              name="tag"
              label="Tag"
              rules={[
                { required: true, message: 'Please enter a tag' },
                { max: 6, message: 'Tag must be 6 characters or less' },
              ]}
            >
              <Input placeholder="e.g. KGT" maxLength={6} />
            </Form.Item>
          )}
        </Form>
      </Modal>
    </PageContainer>
  );
}
