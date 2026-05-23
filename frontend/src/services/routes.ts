import { request } from '@umijs/max';

/** List all dynamically registered plugin routes. */
export async function listRoutes(): Promise<API.DynamicRoute[]> {
  return request('/api/routes', { method: 'GET' });
}

/** Fetch routes registered by a specific plugin. */
export async function getPluginRoutes(pluginId: string): Promise<API.DynamicRoute[]> {
  return request(`/api/routes/${pluginId}`, { method: 'GET' });
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
