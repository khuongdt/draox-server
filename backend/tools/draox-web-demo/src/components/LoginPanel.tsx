import { useState, FormEvent } from 'react';
import { DraoxClient, MessagingPlugin } from 'draox-sdk-web';
import type { AppContext } from '../App.tsx';

interface Props {
  onConnected: (ctx: AppContext) => void;
}

interface FormValues {
  host:      string;
  port:      string;
  username:  string;
  password:  string;
  channel:   string;
}

const DEFAULTS: FormValues = {
  host:     'localhost',
  port:     '9002',
  username: 'admin',
  password: 'draox@Admin#2024',
  channel:  'general',
};

export default function LoginPanel({ onConnected }: Props) {
  const [form, setForm]       = useState<FormValues>(DEFAULTS);
  const [loading, setLoading] = useState(false);
  const [error, setError]     = useState('');

  const set = (key: keyof FormValues) => (e: React.ChangeEvent<HTMLInputElement>) =>
    setForm(prev => ({ ...prev, [key]: e.target.value }));

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      const client = new DraoxClient({
        host: form.host,
        port: Number(form.port),
        // apiUrl defaults to '' (relative) — Vite proxy forwards /api to admin port
      });

      client.on('error', (err: Error) => setError(err.message));

      await client.connect();
      await client.login(form.username, form.password);

      onConnected({
        client,
        messaging: new MessagingPlugin(client),
        username:  form.username,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setLoading(false);
    }
  };

  return (
    <div className="login-screen">
      <div className="login-card">
        <h1>💬 Draox Chat</h1>
        <p className="subtitle">Connect to your Draox server</p>

        <form onSubmit={handleSubmit}>
          <div className="form-grid">
            <div className="field">
              <label>Host</label>
              <input value={form.host} onChange={set('host')} placeholder="localhost" required />
            </div>
            <div className="field">
              <label>WS Port</label>
              <input value={form.port} onChange={set('port')} type="number" placeholder="9002" required />
            </div>
            <div className="field">
              <label>Username</label>
              <input value={form.username} onChange={set('username')} placeholder="admin" required />
            </div>
            <div className="field">
              <label>Channel</label>
              <input value={form.channel} onChange={set('channel')} placeholder="general" required />
            </div>
            <div className="field full">
              <label>Password</label>
              <input value={form.password} onChange={set('password')} type="password" required />
            </div>
          </div>

          <button className="btn btn-primary" type="submit" disabled={loading}>
            {loading ? <><span className="spinner" /> Connecting…</> : 'Connect'}
          </button>
        </form>

        {error && <div className="error-msg">{error}</div>}
      </div>
    </div>
  );
}
