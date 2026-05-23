import { request } from '@umijs/max';

/** Fetch cache hit/miss statistics and memory usage. */
export async function getCacheStats(): Promise<API.CacheStats> {
  return request('/api/cache/stats', { method: 'GET' });
}

/** Check if the cache backend is reachable and responding. */
export async function getCacheHealth(): Promise<API.CacheHealth> {
  return request('/api/cache/health', { method: 'GET' });
}

/** Evict all entries from the cache. Returns confirmation message. */
export async function flushCache(): Promise<string> {
  return request('/api/cache/flush', { method: 'POST' });
}
