import { request } from '@umijs/max';

/** Fetch the latest server metrics snapshot. */
export async function getMetrics(): Promise<API.MetricsSnapshot> {
  return request('/api/metrics', { method: 'GET' });
}

/** Fetch aggregated activity statistics. */
export async function getActivity(): Promise<API.ActivityMetrics> {
  return request('/api/metrics/activity', { method: 'GET' });
}
