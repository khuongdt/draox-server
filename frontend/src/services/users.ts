import { request } from '@umijs/max';

export async function listUsers(): Promise<API.AdminUser[]> {
  const res = await request<any>('/api/users', { method: 'GET' });
  if (Array.isArray(res)) return res;           // interceptor unwrapped correctly
  if (Array.isArray(res?.data)) return res.data; // AxiosResponse or ApiResponse wrapper
  return [];
}

export async function createUser(data: API.CreateUserRequest): Promise<void> {
  return request('/api/users', { method: 'POST', data });
}

export async function updateUser(username: string, data: API.UpdateUserRequest): Promise<void> {
  return request(`/api/users/${username}`, { method: 'PUT', data });
}

export async function deleteUser(username: string): Promise<void> {
  return request(`/api/users/${username}`, { method: 'DELETE' });
}

export async function banUser(username: string): Promise<void> {
  return request(`/api/users/${username}/ban`, { method: 'POST' });
}

export async function unbanUser(username: string): Promise<void> {
  return request(`/api/users/${username}/unban`, { method: 'POST' });
}
