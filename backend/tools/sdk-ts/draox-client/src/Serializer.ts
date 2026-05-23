export interface ParsedMessage {
  type:       string;
  // response
  id?:        string;
  success?:   boolean;
  data?:      unknown;
  error?:     string;
  // event
  category?:  string;
  name?:      string;
  timestamp?: string;
}

export const Serializer = {
  serialize(obj: unknown): string {
    return JSON.stringify(obj);
  },

  parse(json: string): ParsedMessage | null {
    let node: Record<string, unknown>;
    try { node = JSON.parse(json) as Record<string, unknown>; }
    catch { return null; }

    const type = node['type'] as string | undefined;
    if (!type) return null;

    switch (type) {
      case 'response':
        return {
          type:    'response',
          id:      node['id'] as string,
          success: node['success'] as boolean,
          data:    node['data'],
          error:   node['error'] as string | undefined,
        };
      case 'event':
        return {
          type:      'event',
          category:  node['category'] as string,
          name:      node['name'] as string,
          data:      node['data'],
          timestamp: node['timestamp'] as string,
        };
      case 'pong':
        return { type: 'pong' };
      default:
        return { type };
    }
  },
};
