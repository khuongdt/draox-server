import { useAccess } from '@umijs/max';
import {
  PageContainer, ProTable, ProForm, ProFormText, ProFormSelect,
} from '@ant-design/pro-components';
import type { ProColumns } from '@ant-design/pro-components';
import { Tag, Button, Space, Popconfirm, Modal, message, Spin, Empty } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { useState, useEffect, useCallback } from 'react';
import { listRoutes, registerRoute, deleteRoute } from '@/services/routes';

const METHOD_COLORS: Record<string, string> = {
  GET: '#42a5f5',
  POST: '#66bb6a',
  PUT: '#ffb300',
  PATCH: '#ff7043',
  DELETE: '#ef5350',
  HEAD: '#ce93d8',
  OPTIONS: '#80cbc4',
};

const HTTP_METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'];

export default function RoutesPage() {
  const access = useAccess();
  const [modalVisible, setModalVisible] = useState(false);
  const [routes, setRoutes] = useState<API.DynamicRoute[]>([]);
  const [loading, setLoading] = useState(false);
  const [registering, setRegistering] = useState(false);

  const refresh = useCallback(() => {
    setLoading(true);
    listRoutes()
      .then((data) => setRoutes(data))
      .catch((e: Error) => message.error(`Failed to load routes: ${e.message}`))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    refresh();
    const t = setInterval(refresh, 30_000);
    return () => clearInterval(t);
  }, [refresh]);

  const handleRegister = async (values: {
    plugin_id: string;
    path_pattern: string;
    methods: string[];
  }) => {
    setRegistering(true);
    try {
      await registerRoute(values.plugin_id, values.path_pattern, values.methods);
      setModalVisible(false);
      message.success('Route registered');
      refresh();
    } catch (e) {
      message.error(`Failed to register: ${(e as Error).message}`);
    } finally {
      setRegistering(false);
    }
  };

  const runDelete = async (pluginId: string) => {
    try {
      await deleteRoute(pluginId);
      message.success('Route deleted');
      refresh();
    } catch (e) {
      message.error(`Failed to delete: ${(e as Error).message}`);
    }
  };

  const columns: ProColumns<API.DynamicRoute>[] = [
    {
      title: 'Plugin ID',
      dataIndex: 'plugin_id',
      render: (_dom, record) => (
        <span style={{ fontFamily: 'monospace', color: '#a0a0b0', fontSize: 12 }}>{record.plugin_id}</span>
      ),
    },
    {
      title: 'Path',
      dataIndex: 'path',
      render: (_dom, record) => (
        <span style={{ fontFamily: 'monospace', color: '#ff8c42' }}>{record.path}</span>
      ),
    },
    {
      title: 'Methods',
      dataIndex: 'methods',
      render: (_dom, record) => (
        <Space size={4} wrap>
          {(record.methods ?? []).map((m) => (
            <Tag
              key={m}
              style={{
                color: METHOD_COLORS[m] ?? '#e0e0e0',
                background: `${METHOD_COLORS[m] ?? '#e0e0e0'}22`,
                border: `1px solid ${METHOD_COLORS[m] ?? '#e0e0e0'}44`,
                fontWeight: 700,
                fontSize: 11,
              }}
            >
              {m}
            </Tag>
          ))}
        </Space>
      ),
    },
    {
      title: 'Created At',
      dataIndex: 'created_at',
      render: (_dom, record) => (
        <span style={{ color: '#a0a0b0', fontSize: 12 }}>{new Date(record.created_at).toLocaleString()}</span>
      ),
    },
    ...(access?.canRouteManage
      ? ([
          {
            title: 'Actions',
            key: 'actions',
            width: 90,
            render: (_dom, record) => (
              <Popconfirm
                title="Delete all routes for this plugin?"
                description="Plugin will no longer serve routes registered at this path."
                onConfirm={() => runDelete(record.plugin_id)}
                okText="Delete"
                okButtonProps={{ danger: true }}
              >
                <Button size="small" danger>
                  Delete
                </Button>
              </Popconfirm>
            ),
          } as ProColumns<API.DynamicRoute>,
        ])
      : []),
  ];

  return (
    <PageContainer
      title="Plugin Routes"
      subTitle="HTTP routes registered by plugins"
      extra={
        access?.canRouteManage && (
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => setModalVisible(true)}
            style={{ background: '#e05d10', borderColor: '#e05d10', fontWeight: 600 }}
          >
            Register Route
          </Button>
        )
      }
    >
      <Spin spinning={loading}>
        <ProTable<API.DynamicRoute>
          columns={columns}
          dataSource={routes}
          rowKey={(r) => `${r.plugin_id}:${r.path}`}
          search={false}
          options={{ reload: () => refresh() }}
          pagination={{ pageSize: 20 }}
          locale={{
            emptyText: (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={
                  <span style={{ color: '#a0a0b0' }}>No plugin routes registered</span>
                }
              />
            ),
          }}
          style={{ background: 'transparent' }}
        />
      </Spin>

      <Modal
        open={modalVisible}
        title={<span style={{ color: '#e0e0e0' }}>Register Plugin Route</span>}
        onCancel={() => setModalVisible(false)}
        footer={null}
        style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
        styles={{
          header: { background: '#16213e', borderBottom: '1px solid #2a2a4a' },
        }}
      >
        <ProForm
          onFinish={handleRegister}
          submitter={{
            searchConfig: { submitText: 'Register' },
            submitButtonProps: {
              loading: registering,
              style: { background: '#e05d10', borderColor: '#e05d10', fontWeight: 600 },
            },
            resetButtonProps: { onClick: () => setModalVisible(false) },
          }}
        >
          <ProFormText
            name="plugin_id"
            label="Plugin ID"
            placeholder="io.draox.plugin.example"
            rules={[{ required: true }]}
          />
          <ProFormText
            name="path_pattern"
            label="Path Pattern"
            placeholder="/api/resource/*"
            rules={[{ required: true, pattern: /^\//, message: 'Path must start with /' }]}
          />
          <ProFormSelect
            name="methods"
            label="HTTP Methods"
            mode="multiple"
            options={HTTP_METHODS.map((m) => ({ label: m, value: m }))}
            rules={[{ required: true, type: 'array', min: 1 }]}
          />
        </ProForm>
      </Modal>
    </PageContainer>
  );
}
