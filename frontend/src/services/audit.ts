import { request } from '@umijs/max';

interface AuditQueryParams {
  page?: number;
  size?: number;
  action?: string;
  severity?: string;
}

/** Fetch paginated audit log entries with optional filters. */
export async function getAuditLogs(params?: AuditQueryParams): Promise<API.AuditEntry[]> {
  return request('/api/audit', { method: 'GET', params });
}

/** Fetch a single audit log entry by ID. */
export async function getAuditEntry(id: string): Promise<API.AuditEntry> {
  return request(`/api/audit/${id}`, { method: 'GET' });
}
