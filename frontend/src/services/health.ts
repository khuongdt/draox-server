import { request } from '@umijs/max';

/** Fetch lightweight health status. */
export async function getHealth(): Promise<API.HealthResponse> {
  return request('/api/health', { method: 'GET' });
}

/** Fetch detailed component-level health. */
export async function getDetailedHealth(): Promise<API.DetailedHealth> {
  return request('/api/health/detailed', { method: 'GET' });
}

/** Fetch general server metadata and version info. */
export async function getInfo(): Promise<API.ServerInfo> {
  return request('/api/info', { method: 'GET' });
}
