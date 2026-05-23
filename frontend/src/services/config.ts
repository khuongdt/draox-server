import { request } from '@umijs/max';

/** Fetch the current server configuration (all sections). */
export async function getConfig(): Promise<API.ServerConfig> {
  return request('/api/config', { method: 'GET' });
}

/** Trigger a hot-reload of the configuration from disk. Returns status message. */
export async function reloadConfig(): Promise<string> {
  return request('/api/config/reload', { method: 'POST' });
}
