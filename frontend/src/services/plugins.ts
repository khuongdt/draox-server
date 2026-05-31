import { request } from '@umijs/max';

/** List all installed plugins (builtin + WASM). Unwraps `{ total, plugins }`. */
export async function listPlugins(): Promise<API.Plugin[]> {
  const res = await request<any>('/api/plugins', { method: 'GET' });
  if (Array.isArray(res)) return res;
  if (Array.isArray(res?.plugins)) return res.plugins;
  if (Array.isArray(res?.data)) return res.data;
  if (Array.isArray(res?.data?.plugins)) return res.data.plugins;
  return [];
}

/** Get metadata for a single plugin by ID. */
export async function getPlugin(id: string): Promise<API.Plugin> {
  return request(`/api/plugins/${id}`, { method: 'GET' });
}

/** Activate a plugin (transition to ActiveDisabled state). */
export async function activatePlugin(id: string): Promise<void> {
  return request(`/api/plugins/${id}/activate`, { method: 'POST' });
}

/** Deactivate a plugin (transition back to Installed state). */
export async function deactivatePlugin(id: string): Promise<void> {
  return request(`/api/plugins/${id}/deactivate`, { method: 'POST' });
}

/** Enable a plugin so it receives events and handles requests. */
export async function enablePlugin(id: string): Promise<void> {
  return request(`/api/plugins/${id}/enable`, { method: 'POST' });
}

/** Disable a plugin while keeping it activated. */
export async function disablePlugin(id: string): Promise<void> {
  return request(`/api/plugins/${id}/disable`, { method: 'POST' });
}

/** Restart a plugin (disable + re-enable). */
export async function restartPlugin(id: string): Promise<void> {
  return request(`/api/plugins/${id}/restart`, { method: 'POST' });
}

/** Check runtime health of a specific plugin. */
export async function getPluginHealth(id: string): Promise<API.PluginHealth> {
  return request(`/api/plugins/${id}/health`, { method: 'GET' });
}
