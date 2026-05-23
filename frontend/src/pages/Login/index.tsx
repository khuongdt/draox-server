import { ProForm, ProFormText } from '@ant-design/pro-components';
import { Alert, Card, Typography, message } from 'antd';
import { history, useModel } from '@umijs/max';
import { useState } from 'react';
import { LockOutlined, UserOutlined, CloudServerOutlined } from '@ant-design/icons';
import { login } from '@/services/auth';

const { Title, Text } = Typography;

export default function LoginPage() {
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const { setInitialState } = useModel('@@initialState');

  const handleSubmit = async (values: { username: string; password: string }) => {
    setLoading(true);
    setError('');
    try {
      const result = await login(values.username, values.password);
      localStorage.setItem('draox_token', result.token);
      localStorage.setItem('draox_role', result.role);
      // Update initialState immediately so access.ts gets the correct role
      // without requiring a full page reload
      await setInitialState((prev) => ({
        ...prev,
        currentUser: {
          token:    result.token,
          role:     result.role,
          username: result.username,
        },
      }));
      message.success('Login successful');
      history.push('/dashboard');
    } catch {
      setError('Invalid username or password');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      style={{
        minHeight: '100vh',
        background: '#1a1a2e',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 24,
      }}
    >
      <Card
        style={{
          width: '100%',
          maxWidth: 400,
          background: '#16213e',
          border: '1px solid #2a2a4a',
          borderRadius: 12,
        }}
        bodyStyle={{ padding: 40 }}
      >
        {/* Logo and title */}
        <div style={{ textAlign: 'center', marginBottom: 32 }}>
          <CloudServerOutlined
            style={{ fontSize: 48, color: '#e05d10', marginBottom: 12 }}
          />
          <Title level={3} style={{ color: '#e0e0e0', margin: 0, fontWeight: 700 }}>
            Draox Admin
          </Title>
          <Text style={{ color: '#a0a0b0', fontSize: 13 }}>
            Server Management Console
          </Text>
        </div>

        {error && (
          <Alert
            type="error"
            message={error}
            showIcon
            style={{ marginBottom: 20, background: '#4a1010', border: '1px solid #d32f2f' }}
          />
        )}

        <ProForm
          submitter={{
            searchConfig: { submitText: 'Sign In' },
            submitButtonProps: {
              size: 'large',
              loading,
              style: {
                width: '100%',
                background: '#e05d10',
                borderColor: '#e05d10',
                fontWeight: 600,
              },
            },
            resetButtonProps: false,
          }}
          onFinish={handleSubmit}
        >
          <ProFormText
            name="username"
            placeholder="Username"
            fieldProps={{
              prefix: <UserOutlined style={{ color: '#a0a0b0' }} />,
              size: 'large',
              style: { background: '#0f3460', border: '1px solid #2a2a4a', color: '#e0e0e0' },
            }}
            rules={[{ required: true, message: 'Please enter your username' }]}
          />
          <ProFormText.Password
            name="password"
            placeholder="Password"
            fieldProps={{
              prefix: <LockOutlined style={{ color: '#a0a0b0' }} />,
              size: 'large',
              style: { background: '#0f3460', border: '1px solid #2a2a4a' },
            }}
            rules={[{ required: true, message: 'Please enter your password' }]}
          />
        </ProForm>

        <div style={{ textAlign: 'center', marginTop: 16 }}>
          <Text style={{ color: '#a0a0b0', fontSize: 12 }}>
            Draox Server v1.0.0 — Secured Admin Access
          </Text>
        </div>
      </Card>
    </div>
  );
}
