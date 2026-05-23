import {
  PageContainer, StepsForm, ProFormText, ProFormTextArea,
  ProFormSelect, ProFormRadio, ProFormDigit,
} from '@ant-design/pro-components';
import { Upload, Button, message, Card, Typography, Space, Alert } from 'antd';
import { InboxOutlined } from '@ant-design/icons';
import { useState, useRef } from 'react';
import { useRequest, useAccess, history } from '@umijs/max';
import { publishPlugin } from '@/services/marketplace';

const { Text } = Typography;

const CATEGORY_OPTIONS = [
  { label: 'Analytics', value: 'analytics' },
  { label: 'Security', value: 'security' },
  { label: 'Messaging', value: 'messaging' },
  { label: 'Utilities', value: 'utilities' },
  { label: 'Monitoring', value: 'monitoring' },
  { label: 'Other', value: 'other' },
];

export default function PublishPluginPage() {
  const access = useAccess();
  const [pricingType, setPricingType] = useState<'free' | 'paid'>('free');
  const [dxpFile, setDxpFile] = useState<File | null>(null);
  const formData = useRef<Record<string, unknown>>({});

  const { loading: publishing, run: runPublish } = useRequest(
    (data: FormData) => publishPlugin(data),
    {
      manual: true,
      onSuccess: () => {
        message.success('Plugin submitted for review! You will be notified when it is approved.');
        history.push('/marketplace');
      },
    },
  );

  if (!access?.canPublishPlugin) {
    return (
      <PageContainer title="Publish Plugin">
        <Alert
          type="error"
          message="Insufficient permissions"
          description="You need admin or operator role to publish plugins."
        />
      </PageContainer>
    );
  }

  const handlePublish = async () => {
    if (!dxpFile) {
      message.error('Please upload a .dxp plugin package');
      return;
    }
    const fd = new FormData();
    Object.entries(formData.current).forEach(([k, v]) => {
      if (v !== undefined && v !== null) {
        fd.append(k, String(v));
      }
    });
    fd.append('package', dxpFile, dxpFile.name);
    await runPublish(fd);
  };

  return (
    <PageContainer title="Publish Plugin" subTitle="Submit a new plugin to the Draox Marketplace">
      <Card
        style={{ background: '#16213e', border: '1px solid #2a2a4a' }}
        bodyStyle={{ padding: 32 }}
      >
        <StepsForm
          onFinish={handlePublish}
          formProps={{
            onValuesChange: (_, allValues) => {
              formData.current = { ...formData.current, ...allValues };
            },
          }}
          submitter={{
            render: ({ step, onSubmit, onPre }) => (
              <Space>
                {step > 0 && <Button onClick={onPre}>Previous</Button>}
                <Button
                  type="primary"
                  onClick={onSubmit}
                  loading={step === 3 && publishing}
                  style={{ background: '#e05d10', borderColor: '#e05d10', fontWeight: 600 }}
                >
                  {step === 3 ? 'Submit Plugin' : 'Next Step'}
                </Button>
              </Space>
            ),
          }}
        >
          {/* Step 1 — Metadata */}
          <StepsForm.StepForm name="metadata" title="Metadata">
            <ProFormText
              name="name"
              label="Plugin Name"
              placeholder="e.g., Analytics Pro"
              rules={[{ required: true }]}
            />
            <ProFormText
              name="plugin_id"
              label="Plugin ID (reverse-domain)"
              placeholder="e.g., com.yourcompany.pluginname"
              rules={[
                { required: true },
                {
                  pattern: /^[a-z][a-z0-9.]+$/,
                  message: 'Use lowercase reverse-domain format',
                },
              ]}
            />
            <ProFormText
              name="version"
              label="Version"
              placeholder="e.g., 1.0.0"
              rules={[
                { required: true },
                { pattern: /^\d+\.\d+\.\d+$/, message: 'Semver format required (e.g. 1.0.0)' },
              ]}
            />
            <ProFormTextArea
              name="description"
              label="Description"
              placeholder="Describe what your plugin does…"
              rules={[{ required: true, min: 50, message: 'At least 50 characters required' }]}
              fieldProps={{ rows: 4 }}
            />
            <ProFormSelect
              name="category"
              label="Category"
              options={CATEGORY_OPTIONS}
              rules={[{ required: true }]}
            />
          </StepsForm.StepForm>

          {/* Step 2 — Author */}
          <StepsForm.StepForm name="author" title="Author">
            <ProFormText
              name="author_name"
              label="Author / Organization Name"
              placeholder="e.g., Draox Labs"
              rules={[{ required: true }]}
            />
            <ProFormText
              name="author_email"
              label="Contact Email"
              placeholder="e.g., hello@example.com"
              rules={[{ required: true, type: 'email' }]}
            />
            <ProFormText
              name="website"
              label="Website (optional)"
              placeholder="https://your-plugin-site.com"
            />
            <ProFormText
              name="repository"
              label="Source Repository (optional)"
              placeholder="https://github.com/org/repo"
            />
          </StepsForm.StepForm>

          {/* Step 3 — Pricing */}
          <StepsForm.StepForm name="pricing" title="Pricing">
            <ProFormRadio.Group
              name="pricing_type"
              label="Pricing Model"
              initialValue="free"
              options={[
                { label: 'Free', value: 'free' },
                { label: 'Paid (monthly subscription)', value: 'paid' },
              ]}
              fieldProps={{ onChange: (e) => setPricingType(e.target.value) }}
            />
            {pricingType === 'paid' && (
              <ProFormDigit
                name="price"
                label="Price (USD/month)"
                min={0.99}
                max={999.99}
                placeholder="e.g., 9.99"
                rules={[{ required: true, min: 0.99, type: 'number' }]}
                fieldProps={{ prefix: '$', precision: 2 }}
              />
            )}
          </StepsForm.StepForm>

          {/* Step 4 — Upload & Submit */}
          <StepsForm.StepForm name="submit" title="Review & Submit">
            <Card
              title={<span style={{ color: '#e0e0e0' }}>Upload Plugin Package (.dxp)</span>}
              style={{ background: '#0f3460', border: '1px solid #2a2a4a', marginBottom: 20 }}
              headStyle={{ borderBottom: '1px solid #2a2a4a' }}
            >
              <Upload.Dragger
                accept=".dxp"
                maxCount={1}
                beforeUpload={(file) => {
                  setDxpFile(file);
                  return false;
                }}
                onRemove={() => setDxpFile(null)}
                style={{ background: '#16213e', border: '1px dashed #2a2a4a' }}
              >
                <p className="ant-upload-drag-icon">
                  <InboxOutlined style={{ color: '#e05d10', fontSize: 32 }} />
                </p>
                <p style={{ color: '#e0e0e0' }}>Click or drag your .dxp file here</p>
                <p style={{ color: '#a0a0b0', fontSize: 12 }}>
                  The .dxp package must include plugin.toml + WASM binary + assets
                </p>
              </Upload.Dragger>
              {dxpFile && (
                <Text style={{ color: '#53c28b', display: 'block', marginTop: 8 }}>
                  ✓ {dxpFile.name} ({(dxpFile.size / 1024).toFixed(1)} KB)
                </Text>
              )}
            </Card>
            <Card
              style={{ background: '#0f3460', border: '1px solid #2a2a4a' }}
              bodyStyle={{ padding: 16 }}
            >
              <Text style={{ color: '#a0a0b0', fontSize: 13 }}>
                By submitting, you agree to the Draox Marketplace Terms of Service. Your plugin will
                be reviewed for security and compliance before being published. Ed25519 signing is
                required for WASM plugins.
              </Text>
            </Card>
          </StepsForm.StepForm>
        </StepsForm>
      </Card>
    </PageContainer>
  );
}
