import { request } from '@umijs/max';

/** List all dynamically registered plugin routes. Unwraps `{ total, routes }`. */
export async function listRoutes(): Promise<API.DynamicRoute[]> {
  const res = await request<any>('/api/routes', { method: 'GET' });
  if (Array.isArray(res)) return res;
  if (Array.isArray(res?.routes)) return res.routes;
  if (Array.isArray(res?.data)) return res.data;
  if (Array.isArray(res?.data?.routes)) return res.data.routes;
  return [];
}

/** Fetch routes registered by a specific plugin. Unwraps `{ plugin_id, total, routes }`. */
export async function getPluginRoutes(pluginId: string): Promise<API.DynamicRoute[]> {
  const res = await request<any>(`/api/routes/${pluginId}`, { method: 'GET' });
  if (Array.isArray(res)) return res;
  if (Array.isArray(res?.routes)) return res.routes;
  if (Array.isArray(res?.data?.routes)) return res.data.routes;
  return [];
}

/** Register a new route for a plugin. */
export async function registerRoute(
  pluginId: string,
  path: string,
  methods: string[],
): Promise<void> {
  return request(`/api/routes/${pluginId}/register`, {
    method: 'POST',
    data: { path, methods },
  });
}

/** Remove all routes registered by a plugin. */
export async function deleteRoute(pluginId: string): Promise<void> {
  return request(`/api/routes/${pluginId}`, { method: 'DELETE' });
}
