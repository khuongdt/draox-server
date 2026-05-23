import { Modal, Typography } from 'antd';
import { WarningOutlined, ExclamationCircleOutlined } from '@ant-design/icons';
import { useState } from 'react';

const { Text } = Typography;

interface ConfirmActionModalProps {
  title: string;
  description: string;
  onConfirm: () => Promise<void>;
  visible: boolean;
  onCancel: () => void;
  type?: 'danger' | 'warning';
  confirmText?: string;
}

const ConfirmActionModal: React.FC<ConfirmActionModalProps> = ({
  title,
  description,
  onConfirm,
  visible,
  onCancel,
  type = 'warning',
  confirmText = 'Confirm',
}) => {
  const [loading, setLoading] = useState(false);

  const isDanger = type === 'danger';
  const borderColor = isDanger ? '#d32f2f' : '#f5a623';
  const IconComponent = isDanger ? ExclamationCircleOutlined : WarningOutlined;
  const iconColor = isDanger ? '#d32f2f' : '#f5a623';

  const handleOk = async () => {
    setLoading(true);
    try {
      await onConfirm();
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      open={visible}
      title={
        <span>
          <IconComponent style={{ color: iconColor, marginRight: 8 }} />
          <span style={{ color: '#e0e0e0' }}>{title}</span>
        </span>
      }
      onCancel={onCancel}
      onOk={handleOk}
      okText={confirmText}
      confirmLoading={loading}
      okButtonProps={{
        danger: isDanger,
        style: isDanger ? {} : { background: '#f5a623', borderColor: '#f5a623', color: '#000' },
      }}
      styles={{
        content: {
          background: '#16213e',
          border: `1px solid ${borderColor}`,
          borderRadius: 8,
        },
        header: { background: '#16213e', borderBottom: '1px solid #2a2a4a' },
        footer: { background: '#16213e', borderTop: '1px solid #2a2a4a' },
        mask: { background: 'rgba(0,0,0,0.6)' },
      }}
    >
      <Text style={{ color: '#e0e0e0' }}>{description}</Text>
    </Modal>
  );
};

export default ConfirmActionModal;
