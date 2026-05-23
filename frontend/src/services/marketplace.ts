import { request } from '@umijs/max';

interface SearchParams {
  q?: string;
  category?: string;
}

/** Search marketplace plugins by keyword and/or category. */
export async function searchPlugins(params?: SearchParams): Promise<API.MarketplacePlugin[]> {
  return request('/api/marketplace/search', { method: 'GET', params });
}

/** Fetch editorially featured plugins. */
export async function getFeatured(): Promise<API.MarketplacePlugin[]> {
  return request('/api/marketplace/featured', { method: 'GET' });
}

/** Fetch most-downloaded plugins. */
export async function getPopular(): Promise<API.MarketplacePlugin[]> {
  return request('/api/marketplace/popular', { method: 'GET' });
}

/** Fetch list of available plugin categories. */
export async function getCategories(): Promise<string[]> {
  return request('/api/marketplace/categories', { method: 'GET' });
}

/** Fetch detail for a single marketplace plugin. */
export async function getPlugin(id: string): Promise<API.MarketplacePlugin> {
  return request(`/api/marketplace/plugins/${id}`, { method: 'GET' });
}

/** Fetch version history for a marketplace plugin. */
export async function getVersions(id: string): Promise<API.PluginVersion[]> {
  return request(`/api/marketplace/plugins/${id}/versions`, { method: 'GET' });
}

/** Fetch user reviews for a marketplace plugin. */
export async function getReviews(id: string): Promise<API.PluginReview[]> {
  return request(`/api/marketplace/plugins/${id}/reviews`, { method: 'GET' });
}

/** Submit a review for a marketplace plugin. */
export async function postReview(id: string, rating: number, comment: string): Promise<void> {
  return request(`/api/marketplace/plugins/${id}/reviews`, {
    method: 'POST',
    data: { rating, comment },
  });
}

/** Fetch download and rating analytics for a plugin. */
export async function getAnalytics(id: string): Promise<API.PluginAnalytics> {
  return request(`/api/marketplace/plugins/${id}/analytics`, { method: 'GET' });
}

/** Publish a new plugin to the marketplace (multipart form with .dxp + metadata). */
export async function publishPlugin(data: FormData): Promise<void> {
  return request('/api/marketplace/publish', {
    method: 'POST',
    data,
    requestType: 'form',
  });
}
