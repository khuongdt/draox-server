import { request } from '@umijs/max';

/** List all active sessions. Unwraps `{ total, sessions }` or `{ data: { sessions } }` envelope. */
export async function listSessions(): Promise<API.Session[]> {
  const res = await request<any>('/api/sessions', { method: 'GET' });
  if (Array.isArray(res)) return res;
  if (Array.isArray(res?.sessions)) return res.sessions;
  if (Array.isArray(res?.data)) return res.data;
  if (Array.isArray(res?.data?.sessions)) return res.data.sessions;
  return [];
}

/** Get a single session by ID. */
export async function getSession(id: string): Promise<API.Session> {
  return request(`/api/sessions/${id}`, { method: 'GET' });
}

/** Destroy a session and close all its connections. */
export async function destroySession(id: string): Promise<void> {
  return request(`/api/sessions/${id}`, { method: 'DELETE' });
}

/** Gracefully drain a session (finish in-flight requests before closing). */
export async function drainSession(id: string): Promise<void> {
  return request(`/api/sessions/${id}/drain`, { method: 'POST' });
}

/** Retrieve bandwidth and timing metrics for a session. */
export async function getSessionMetrics(id: string): Promise<API.SessionMetrics> {
  return request(`/api/sessions/${id}/metrics`, { method: 'GET' });
}
