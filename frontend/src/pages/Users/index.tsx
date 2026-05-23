import { useRequest } from '@umijs/max';
import { PageContainer, ProTable } from '@ant-design/pro-components';
import {
  Button,
  Form,
  Input,
  message,
  Modal,
  Popconfirm,
  Select,
  Space,
  Spin,
  Tag,
} from 'antd';
import { useState } from 'react';
import { listUsers, createUser, updateUser, deleteUser } from '@/services/users';

const ROLE_COLOR: Record<API.AdminRole, string> = {
  admin: '#ff4d4f',
  operator: '#faad14',
  viewer: '#52c41a',
};

const ROLE_OPTIONS = [
  { label: 'Admin', value: 'admin' },
  { label: 'Operator', value: 'operator' },
  { label: 'Viewer', value: 'viewer' },
];

type ModalMode = 'create' | 'edit';

export default function UsersPage() {
  const { data: users = [], loading, refresh } = useRequest(listUsers, {
    refreshOnWindowFocus: false,
  });

  const [modalOpen, setModalOpen] = useState(false);
  const [modalMode, setModalMode] = useState<ModalMode>('create');
  const [editingUser, setEditingUser] = useState<API.AdminUser | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [form] = Form.useForm();

  const openCreate = () => {
    setModalMode('create');
    setEditingUser(null);
    form.resetFields();
    setModalOpen(true);
  };

  const openEdit = (user: API.AdminUser) => {
    setModalMode('edit');
    setEditingUser(user);
    form.setFieldsValue({ role: user.role });
    setModalOpen(true);
  };

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setSubmitting(true);

      if (modalMode === 'create') {
        await createUser(values as API.CreateUserRequest);
        message.success(`User '${values.username}' created`);
      } else if (editingUser) {
        const payload: API.UpdateUserRequest = {};
        if (values.password) payload.password = values.password;
        if (values.role) payload.role = values.role;
        await updateUser(editingUser.username, payload);
        message.success(`User '${editingUser.username}' updated`);
      }

      setModalOpen(false);
      refresh();
    } catch {
      // validation error or network — already shown by interceptor
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (username: string) => {
    await deleteUser(username);
    message.success(`User '${username}' deleted`);
    refresh();
  };

  const columns = [
    {
      title: 'Username',
      dataIndex: 'username',
      render: (v: string) => (
        <span style={{ fontFamily: 'monospace', color: '#e0e0e0' }}>{v}</span>
      ),
    },
    {
      title: 'Role',
      dataIndex: 'role',
      render: (v: API.AdminRole) => (
        <Tag color={ROLE_COLOR[v]} style={{ fontWeight: 600 }}>
          {v.toUpperCase()}
        </Tag>
      ),
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_: unknown, record: API.AdminUser) => (
        <Space>
          <Button size="small" onClick={() => openEdit(record)}>
            Edit
          </Button>
          <Popconfirm
            title={`Delete user '${record.username}'?`}
            description="This action cannot be undone."
            onConfirm={() => handleDelete(record.username)}
            okText="Delete"
            okButtonProps={{ danger: true }}
          >
            <Button size="small" danger>
              Delete
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <PageContainer
      title="Users"
      subTitle="Manage admin dashboard accounts"
      extra={
        <Button type="primary" onClick={openCreate}>
          Add User
        </Button>
      }
    >
      <Spin spinning={loading}>
        <ProTable<API.AdminUser>
          columns={columns}
          dataSource={users}
          rowKey="username"
          search={false}
          options={{ reload: () => refresh() }}
          pagination={{ pageSize: 20 }}
          style={{ background: 'transparent' }}
        />
      </Spin>

      <Modal
        title={modalMode === 'create' ? 'Add User' : `Edit User — ${editingUser?.username}`}
        open={modalOpen}
        onOk={handleSubmit}
        onCancel={() => setModalOpen(false)}
        confirmLoading={submitting}
        okText={modalMode === 'create' ? 'Create' : 'Save'}
        destroyOnClose
      >
        <Form form={form} layout="vertical" style={{ marginTop: 16 }}>
          {modalMode === 'create' && (
            <Form.Item
              name="username"
              label="Username"
              rules={[{ required: true, message: 'Please enter a username' }]}
            >
              <Input placeholder="e.g. operator2" autoComplete="off" />
            </Form.Item>
          )}

          <Form.Item
            name="password"
            label={modalMode === 'edit' ? 'New Password (leave blank to keep)' : 'Password'}
            rules={
              modalMode === 'create'
                ? [
                    { required: true, message: 'Please enter a password' },
                    { min: 8, message: 'Password must be at least 8 characters' },
                  ]
                : [{ min: 8, message: 'Password must be at least 8 characters' }]
            }
          >
            <Input.Password placeholder="Min. 8 characters" autoComplete="new-password" />
          </Form.Item>

          <Form.Item
            name="role"
            label="Role"
            rules={[{ required: true, message: 'Please select a role' }]}
          >
            <Select options={ROLE_OPTIONS} placeholder="Select role" />
          </Form.Item>
        </Form>
      </Modal>
    </PageContainer>
  );
}
