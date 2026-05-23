import { useEffect, useState } from 'react';
import { PageContainer } from '@ant-design/pro-components';
import {
  Alert,
  Button,
  Collapse,
  Descriptions,
  message,
  Modal,
  Skeleton,
  Table,
  Tabs,
  Tag,
  Tooltip,
} from 'antd';
import { EditOutlined, ReloadOutlined, WarningOutlined } from '@ant-design/icons';
import { getConfig, reloadConfig, updateConfig } from '@/services/config';

// ─── Helpers ──────────────────────────────────────────────────────────────────

function isObject(v: unknown): v is Record<string, unknown> {
  return v !== null && typeof v === 'object' && !Array.isArray(v);
}

function computeDiff(
  oldObj: Record<string, unknown>,
  newObj: Record<string, unknown>,
  prefix = '',
): API.ConfigDiff[] {
  const diffs: API.ConfigDiff[] = [];
  const allKeys = new Set([...Object.keys(oldObj), ...Object.keys(newObj)]);
  for (const key of allKeys) {
    const path = prefix ? `${prefix}.${key}` : key;
    const o = oldObj[key];
    const n = newObj[key];
    if (isObject(o) && isObject(n)) {
      diffs.push(...computeDiff(o, n, path));
    } else {
      const oldStr = JSON.stringify(o);
      const newStr = JSON.stringify(n);
      if (oldStr !== newStr) {
        diffs.push({ path, old: o, new: n });
      }
    }
  }
  return diffs;
}

// ─── SectionCard ──────────────────────────────────────────────────────────────

interface SectionCardProps {
  title: string;
  data: Record<string, unknown>;
  canEdit: boolean;
  onEdit: (section: string, data: Record<string, unknown>) => void;
}

function SectionCard({ title, data, canEdit, onEdit }: SectionCardProps) {
  const scalarEntries = Object.entries(data).filter(([, v]) => !isObject(v));
  const nestedEntries = Object.entries(data).filter(([, v]) => isObject(v));

  const descItems = scalarEntries.map(([k, v]) => ({
    key: k,
    label: k,
    children: renderScalar(k, v),
  }));

  const nestedItems = nestedEntries.map(([k, v]) => ({
    key: k,
    label: <span style={{ color: '#a0a0b0', fontFamily: 'monospace' }}>[{k}]</span>,
    children: (
      <Descriptions
        size="small"
        column={2}
        items={Object.entries(v as Record<string, unknown>).map(([kk, vv]) => ({
          key: kk,
          label: kk,
          children: renderScalar(`${k}.${kk}`, vv),
        }))}
      />
    ),
  }));

  return (
    <div
      style={{
        background: '#1a2744',
        border: '1px solid #2a3a5a',
        borderRadius: 8,
        marginBottom: 16,
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '10px 16px',
          borderBottom: '1px solid #2a3a5a',
        }}
      >
        <span
          style={{
            color: '#e0e0e0',
            fontWeight: 700,
            fontFamily: 'monospace',
            letterSpacing: 1,
            textTransform: 'uppercase',
          }}
        >
          [{title}]
        </span>
        {canEdit && (
          <Button
            size="small"
            icon={<EditOutlined />}
            onClick={() => onEdit(title, data)}
            style={{ background: '#e05d10', borderColor: '#e05d10', color: '#fff' }}
          >
            Edit
          </Button>
        )}
      </div>
      <div style={{ padding: '12px 16px' }}>
        {descItems.length > 0 && (
          <Descriptions
            size="small"
            column={2}
            items={descItems}
            style={{ marginBottom: nestedItems.length > 0 ? 12 : 0 }}
          />
        )}
        {nestedItems.length > 0 && (
          <Collapse
            size="small"
            items={nestedItems}
            style={{ background: 'transparent', border: 'none' }}
          />
        )}
      </div>
    </div>
  );
}

function renderScalar(key: string, value: unknown): React.ReactNode {
  if (value === '[REDACTED]') {
    return <Tag color="red">REDACTED</Tag>;
  }
  if (typeof value === 'boolean') {
    return <Tag color={value ? 'green' : 'default'}>{String(value)}</Tag>;
  }
  if (value === null || value === undefined) {
    return <span style={{ color: '#666' }}>—</span>;
  }
  if (Array.isArray(value)) {
    return (
      <span style={{ color: '#a0c4ff', fontFamily: 'monospace', fontSize: 12 }}>
        [{value.join(', ')}]
      </span>
    );
  }
  return <span style={{ color: '#e0e0e0', fontFamily: 'monospace', fontSize: 12 }}>{String(value)}</span>;
}

// ─── EditModal ────────────────────────────────────────────────────────────────

interface EditModalProps {
  open: boolean;
  section: string;
  data: Record<string, unknown>;
  onCancel: () => void;
  onPreview: (section: string, parsed: Record<string, unknown>, raw: string) => void;
}

function EditModal({ open, section, data, onCancel, onPreview }: EditModalProps) {
  const [raw, setRaw] = useState('');
  const [parseError, setParseError] = useState('');

  useEffect(() => {
    if (open) {
      setRaw(JSON.stringify(data, null, 2));
      setParseError('');
    }
  }, [open, data]);

  const handlePreview = () => {
    try {
      const parsed = JSON.parse(raw);
      if (!isObject(parsed)) {
        setParseError('Must be a JSON object');
        return;
      }
      setParseError('');
      onPreview(section, parsed, raw);
    } catch {
      setParseError('Invalid JSON');
    }
  };

  return (
    <Modal
      open={open}
      title={
        <span style={{ fontFamily: 'monospace' }}>
          Edit [{section}]
        </span>
      }
      onCancel={onCancel}
      width={640}
      footer={[
        <Button key="cancel" onClick={onCancel}>
          Cancel
        </Button>,
        <Button key="preview" type="primary" onClick={handlePreview}>
          Preview Changes
        </Button>,
      ]}
    >
      <p style={{ color: '#a0a0b0', fontSize: 12, marginBottom: 8 }}>
        <WarningOutlined style={{ marginRight: 4, color: '#e05d10' }} />
        Fields marked <Tag color="red" style={{ fontSize: 11 }}>REDACTED</Tag> are read-only and cannot be changed here.
      </p>
      {parseError && (
        <Alert type="error" message={parseError} style={{ marginBottom: 8 }} />
      )}
      <textarea
        value={raw}
        onChange={(e) => setRaw(e.target.value)}
        style={{
          width: '100%',
          height: 320,
          fontFamily: 'monospace',
          fontSize: 13,
          background: '#0f1e3a',
          color: '#e0e0e0',
          border: '1px solid #2a3a5a',
          borderRadius: 6,
          padding: 12,
          resize: 'vertical',
        }}
      />
    </Modal>
  );
}

// ─── DiffModal ────────────────────────────────────────────────────────────────

interface DiffModalProps {
  open: boolean;
  diffs: API.ConfigDiff[];
  saving: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

function DiffModal({ open, diffs, saving, onCancel, onConfirm }: DiffModalProps) {
  const columns = [
    {
      title: 'Field',
      dataIndex: 'path',
      key: 'path',
      render: (v: string) => (
        <span style={{ fontFamily: 'monospace', fontSize: 12 }}>{v}</span>
      ),
    },
    {
      title: 'Old Value',
      dataIndex: 'old',
      key: 'old',
      render: (v: unknown) => (
        <span style={{ fontFamily: 'monospace', fontSize: 12, color: '#ff7875' }}>
          {JSON.stringify(v)}
        </span>
      ),
    },
    {
      title: 'New Value',
      dataIndex: 'new',
      key: 'new',
      render: (v: unknown) => (
        <span style={{ fontFamily: 'monospace', fontSize: 12, color: '#95de64' }}>
          {JSON.stringify(v)}
        </span>
      ),
    },
  ];

  return (
    <Modal
      open={open}
      title="Confirm Configuration Changes"
      onCancel={onCancel}
      width={720}
      footer={[
        <Button key="cancel" onClick={onCancel} disabled={saving}>
          Cancel
        </Button>,
        <Button
          key="confirm"
          type="primary"
          danger
          loading={saving}
          onClick={onConfirm}
        >
          Apply {diffs.length} Change{diffs.length !== 1 ? 's' : ''}
        </Button>,
      ]}
    >
      {diffs.length === 0 ? (
        <Alert type="info" message="No changes detected." />
      ) : (
        <>
          <Alert
            type="warning"
            showIcon
            message={`${diffs.length} field(s) will be changed. A backup will be created automatically.`}
            style={{ marginBottom: 12 }}
          />
          <Table
            size="small"
            dataSource={diffs}
            columns={columns}
            rowKey="path"
            pagination={false}
            scroll={{ y: 320 }}
          />
        </>
      )}
    </Modal>
  );
}

// ─── Tab definitions ──────────────────────────────────────────────────────────

const TAB_SECTIONS: Record<string, string[]> = {
  'Server & Sessions': ['server', 'sessions'],
  'Networking': ['tcp', 'udp', 'websocket', 'http', 'grpc'],
  'Security': ['tls', 'traffic_guard'],
  'Storage & Cache': ['storage', 'cache'],
  'Admin & System': ['admin_api', 'logging', 'metrics', 'billing', 'marketplace'],
  'Plugins': ['plugins'],
};

// ─── Page ─────────────────────────────────────────────────────────────────────

export default function ConfigPage() {
  const canEdit = localStorage.getItem('draox_role') === 'admin';

  const [config, setConfig] = useState<API.ServerConfig>({});
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState('');
  const [reloading, setReloading] = useState(false);

  // Edit modal
  const [editSection, setEditSection] = useState('');
  const [editData, setEditData] = useState<Record<string, unknown>>({});
  const [editOpen, setEditOpen] = useState(false);

  // Diff modal
  const [pendingSection, setPendingSection] = useState('');
  const [pendingData, setPendingData] = useState<Record<string, unknown>>({});
  const [diffs, setDiffs] = useState<API.ConfigDiff[]>([]);
  const [diffOpen, setDiffOpen] = useState(false);
  const [saving, setSaving] = useState(false);

  const loadConfig = () => {
    setLoading(true);
    setLoadError('');
    getConfig()
      .then(setConfig)
      .catch((e) => setLoadError(String(e)))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadConfig();
  }, []);

  const handleReload = () => {
    setReloading(true);
    reloadConfig()
      .then((msg) => message.success(msg || 'Config reload triggered'))
      .catch((e) => message.error(String(e)))
      .finally(() => setReloading(false));
  };

  const handleEdit = (section: string, data: Record<string, unknown>) => {
    setEditSection(section);
    setEditData(data);
    setEditOpen(true);
  };

  const handlePreview = (section: string, parsed: Record<string, unknown>) => {
    const oldData = (config[section] as Record<string, unknown>) ?? {};
    const computed = computeDiff(oldData, parsed, section);
    setPendingSection(section);
    setPendingData(parsed);
    setDiffs(computed);
    setEditOpen(false);
    setDiffOpen(true);
  };

  const handleConfirm = () => {
    const newConfig: API.ServerConfig = { ...config, [pendingSection]: pendingData };
    setSaving(true);
    updateConfig(newConfig)
      .then((msg) => {
        message.success(msg || 'Configuration updated');
        setDiffOpen(false);
        loadConfig();
      })
      .catch((e) => message.error(String(e)))
      .finally(() => setSaving(false));
  };

  const renderSections = (sections: string[]) => {
    const present = sections.filter((s) => config[s] !== undefined);
    if (present.length === 0) {
      return (
        <div style={{ color: '#a0a0b0', textAlign: 'center', padding: 32 }}>
          No data for this tab.
        </div>
      );
    }
    return present.map((s) => (
      <SectionCard
        key={s}
        title={s}
        data={(config[s] as Record<string, unknown>) ?? {}}
        canEdit={canEdit}
        onEdit={handleEdit}
      />
    ));
  };

  // Sections not covered by any tab
  const coveredSections = new Set(Object.values(TAB_SECTIONS).flat());
  const extraSections = Object.keys(config).filter((k) => !coveredSections.has(k));

  const tabItems = Object.entries(TAB_SECTIONS).map(([label, sections]) => ({
    key: label,
    label,
    children: <div style={{ paddingTop: 16 }}>{renderSections(sections)}</div>,
  }));

  if (extraSections.length > 0) {
    tabItems.push({
      key: 'Other',
      label: 'Other',
      children: (
        <div style={{ paddingTop: 16 }}>
          {extraSections.map((s) => (
            <SectionCard
              key={s}
              title={s}
              data={(config[s] as Record<string, unknown>) ?? {}}
              canEdit={canEdit}
              onEdit={handleEdit}
            />
          ))}
        </div>
      ),
    });
  }

  return (
    <PageContainer
      title="Configuration"
      subTitle="Server configuration — read from config/default.toml"
      extra={[
        <Tooltip key="reload" title="Trigger hot-reload from disk">
          <Button
            icon={<ReloadOutlined />}
            loading={reloading}
            onClick={handleReload}
            style={{ background: '#e05d10', borderColor: '#e05d10', color: '#fff' }}
          >
            Reload Config
          </Button>
        </Tooltip>,
      ]}
    >
      {loadError && (
        <Alert
          type="error"
          message="Failed to load configuration"
          description={loadError}
          style={{ marginBottom: 16 }}
        />
      )}

      <Skeleton loading={loading} active paragraph={{ rows: 10 }}>
        <Tabs
          items={tabItems}
          type="card"
          style={{ marginTop: -8 }}
        />
      </Skeleton>

      <EditModal
        open={editOpen}
        section={editSection}
        data={editData}
        onCancel={() => setEditOpen(false)}
        onPreview={handlePreview}
      />

      <DiffModal
        open={diffOpen}
        diffs={diffs}
        saving={saving}
        onCancel={() => setDiffOpen(false)}
        onConfirm={handleConfirm}
      />
    </PageContainer>
  );
}
