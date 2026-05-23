import { request } from '@umijs/max';

export async function getConfig(): Promise<API.ServerConfig> {
  const res = await request<any>('/api/config', { method: 'GET' });
  if (res && typeof res === 'object' && 'data' in res && res.data) return res.data;
  return res ?? {};
}

export async function updateConfig(data: API.ServerConfig): Promise<string> {
  const res = await request<any>('/api/config', { method: 'PUT', data });
  return res?.message ?? res?.data?.message ?? String(res);
}

export async function reloadConfig(): Promise<string> {
  const res = await request<any>('/api/config/reload', { method: 'POST' });
  return res?.message ?? res?.data?.message ?? String(res);
}
