import { request } from '@umijs/max';

/** List all active connections. Unwraps `{ total, connections }` envelope. */
export async function listConnections(): Promise<API.Connection[]> {
  const res = await request<any>('/api/connections', { method: 'GET' });
  if (Array.isArray(res)) return res;
  if (Array.isArray(res?.connections)) return res.connections;
  if (Array.isArray(res?.data)) return res.data;
  if (Array.isArray(res?.data?.connections)) return res.data.connections;
  return [];
}

/** Get a single connection by ID. */
export async function getConnection(id: string): Promise<API.Connection> {
  return request(`/api/connections/${id}`, { method: 'GET' });
}

/** Force-disconnect a client connection. */
export async function disconnectConnection(id: string): Promise<void> {
  return request(`/api/connections/${id}`, { method: 'DELETE' });
}

/** Retrieve aggregated connection statistics. */
export async function getConnectionStats(): Promise<API.ConnectionStats> {
  return request('/api/connections/stats', { method: 'GET' });
}
