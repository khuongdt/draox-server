import { request } from '@umijs/max';

// Clans are contributed by the io.draox.clans built-in plugin and exposed
// through the admin-api router at /api/clans.

export async function listClans(): Promise<API.Clan[]> {
  const res = await request<any>('/api/clans', { method: 'GET' });
  if (Array.isArray(res)) return res;
  if (Array.isArray(res?.data)) return res.data;
  return [];
}

export async function createClan(data: API.CreateClanRequest): Promise<API.Clan> {
  return request<API.Clan>('/api/clans', { method: 'POST', data });
}

export async function deleteClan(id: string): Promise<void> {
  return request(`/api/clans/${id}`, { method: 'DELETE' });
}

export async function freezeClan(id: string, frozen: boolean): Promise<void> {
  const path = frozen ? 'freeze' : 'unfreeze';
  return request(`/api/clans/${id}/${path}`, { method: 'POST' });
}

export async function joinClan(id: string): Promise<void> {
  return request(`/api/clans/${id}/join`, { method: 'POST' });
}

export async function leaveClan(id: string): Promise<void> {
  return request(`/api/clans/${id}/leave`, { method: 'POST' });
}

export async function listClanMembers(id: string): Promise<API.ClanMember[]> {
  const res = await request<any>(`/api/clans/${id}/members`, { method: 'GET' });
  if (Array.isArray(res)) return res;
  if (Array.isArray(res?.data)) return res.data;
  return [];
}
