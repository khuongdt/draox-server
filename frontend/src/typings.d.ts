// Global TypeScript namespace declarations for the Draox Admin API shapes

declare namespace API {
  /** Authenticated user information returned by /api/info */
  interface CurrentUser {
    identity: string;
    role: 'admin' | 'operator' | 'viewer';
    token: string;
    expires_at: string;
  }

  /** Payload returned by the POST /api/login endpoint */
  interface LoginResult {
    token: string;
    role: 'admin' | 'operator' | 'viewer';
    identity: string;
    expires_at: string;
  }

  /** Standard envelope wrapping all Admin API responses */
  interface ApiResponse<T = unknown> {
    success: boolean;
    data?: T;
    message?: string;
  }
}
