import { request } from '@umijs/max';

/** Get traffic guard summary statistics. */
export async function getGuardStats(): Promise<API.GuardStats> {
  return request('/api/guard/stats', { method: 'GET' });
}

/** List all currently banned IP addresses. */
export async function listBans(): Promise<API.BanListResponse> {
  return request('/api/guard/bans', { method: 'GET' });
}

/** Ban an IP address with an optional reason. Returns confirmation message. */
export async function banIp(ip: string, reason?: string): Promise<string> {
  return request('/api/guard/ban', { method: 'POST', data: { ip, reason } });
}

/** Remove a ban on an IP address. Returns confirmation message. */
export async function unbanIp(ip: string): Promise<string> {
  return request('/api/guard/unban', { method: 'POST', data: { ip } });
}

/** Add an IP to the whitelist (always allowed). Returns confirmation message. */
export async function addWhitelist(ip: string): Promise<string> {
  return request('/api/guard/whitelist', { method: 'POST', data: { ip } });
}

/** Add an IP to the blacklist (always denied). Returns confirmation message. */
export async function addBlacklist(ip: string): Promise<string> {
  return request('/api/guard/blacklist', { method: 'POST', data: { ip } });
}

/** Look up reputation score for an IP (0 = clean, 100 = malicious). */
export async function getReputation(ip: string): Promise<API.ReputationResponse> {
  return request(`/api/guard/reputation/${ip}`, { method: 'GET' });
}
