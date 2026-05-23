import { request } from '@umijs/max';

/** List all active sessions. */
export async function listSessions(): Promise<API.Session[]> {
  return request('/api/sessions', { method: 'GET' });
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
