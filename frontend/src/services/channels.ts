import { request } from '@umijs/max';

// Channels are contributed by the io.draox.messaging built-in plugin and
// exposed through the admin-api router at /api/channels.

export async function listChannels(): Promise<API.Channel[]> {
  const res = await request<any>('/api/channels', { method: 'GET' });
  if (Array.isArray(res)) return res;
  if (Array.isArray(res?.data)) return res.data;
  return [];
}

export async function createChannel(data: API.CreateChannelRequest): Promise<API.Channel> {
  return request<API.Channel>('/api/channels', { method: 'POST', data });
}

export async function deleteChannel(id: string): Promise<void> {
  return request(`/api/channels/${id}`, { method: 'DELETE' });
}

export async function freezeChannel(id: string, frozen: boolean): Promise<void> {
  const path = frozen ? 'freeze' : 'unfreeze';
  return request(`/api/channels/${id}/${path}`, { method: 'POST' });
}

export async function subscribeChannel(id: string): Promise<void> {
  return request(`/api/channels/${id}/subscribe`, { method: 'POST' });
}

export async function unsubscribeChannel(id: string): Promise<void> {
  return request(`/api/channels/${id}/unsubscribe`, { method: 'POST' });
}
