import { request } from '@umijs/max';

/** Fetch all available billing plans. */
export async function getPlans(): Promise<API.BillingPlan[]> {
  return request('/api/billing/plans', { method: 'GET' });
}

/** Fetch current usage stats for a specific client. */
export async function getUsage(clientId: string): Promise<API.UsageInfo> {
  return request(`/api/billing/usage/${clientId}`, { method: 'GET' });
}

/** Assign a billing plan to a client. */
export async function assignPlan(clientId: string, planId: string): Promise<void> {
  return request(`/api/billing/plan/${clientId}`, {
    method: 'PUT',
    data: { plan_id: planId },
  });
}
