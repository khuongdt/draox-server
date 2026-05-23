import { request } from '@umijs/max';

export async function listUsers(): Promise<API.AdminUser[]> {
  return request('/api/users', { method: 'GET' });
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
