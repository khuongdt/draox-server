import { request } from '@umijs/max';

/** Authenticate user and receive a JWT token. */
export async function login(username: string, password: string): Promise<API.LoginResult> {
  return request('/api/auth/login', {
    method: 'POST',
    data: { username, password },
  });
}

/** Clear local authentication data (token + role). */
export function logout(): void {
  localStorage.removeItem('draox_token');
  localStorage.removeItem('draox_role');
}

/** Return the current user from token + /api/info, or null if unauthenticated. */
export async function getIdentity(): Promise<API.CurrentUser | null> {
  const token = localStorage.getItem('draox_token');
  if (!token) return null;
  try {
    const info = await request<API.ServerInfo>('/api/info', { method: 'GET' });
    const role = localStorage.getItem('draox_role') ?? 'viewer';
    return { token, role, username: 'admin', avatar: undefined };
  } catch {
    return null;
  }
}
