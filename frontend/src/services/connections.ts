import { request } from '@umijs/max';

/** List all active connections. */
export async function listConnections(): Promise<API.Connection[]> {
  return request('/api/connections', { method: 'GET' });
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
