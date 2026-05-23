// Global TypeScript namespace declarations for the Draox Admin API shapes

declare namespace API {
  /** Standard envelope wrapping all Admin API responses */
  interface ApiResponse<T = unknown> {
    success: boolean;
    data?: T;
    message?: string;
  }
}
