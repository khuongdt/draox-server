import { history, RequestConfig, RunTimeLayoutConfig } from '@umijs/max';
import { message } from 'antd';
import WsHeaderIndicator from '@/components/WsHeaderIndicator';
import ErrorBoundary from '@/components/ErrorBoundary';
import defaultSettings from '../config/defaultSettings';

// Storage key used for persisting the JWT token
const TOKEN_KEY = 'draox_token';

// Fetch initial application state — called once on app bootstrap
export async function getInitialState(): Promise<{
  currentUser?: API.CurrentUser;
  settings?: typeof defaultSettings;
}> {
  const token = localStorage.getItem(TOKEN_KEY);

  // No token means the user must log in first
  if (!token) {
    history.push('/login');
    return { settings: defaultSettings };
  }

  try {
    // Validate the stored token and retrieve user identity + role from the server
    const res = await fetch('/api/auth/me', {
      headers: { Authorization: `Bearer ${token}` },
    });

    if (!res.ok) {
      // Token is invalid or expired — clear it and redirect to login
      localStorage.removeItem(TOKEN_KEY);
      localStorage.removeItem('draox_role');
      history.push('/login');
      return { settings: defaultSettings };
    }

    const json: API.ApiResponse<{ username: string; role: string }> = await res.json();

    if (json.success && json.data) {
      return {
        currentUser: {
          token,
          username: json.data.username,
          role: json.data.role,
        },
        settings: defaultSettings,
      };
    }
  } catch {
    // Network error or JSON parse failure — fall through to redirect
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem('draox_role');
    history.push('/login');
  }

  return { settings: defaultSettings };
}

// ProLayout runtime configuration
export const layout: RunTimeLayoutConfig = ({ initialState }) => {
  return {
    ...defaultSettings,
    // Right-side header: WS stream status indicators + logout
    rightContentRender: () => (
      <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
        <WsHeaderIndicator />
        <span
          style={{ cursor: 'pointer', color: '#a0a0b0', fontSize: 14 }}
          onClick={() => {
            localStorage.removeItem(TOKEN_KEY);
            history.push('/login');
          }}
        >
          Logout
        </span>
      </div>
    ),
    // Watermark using the current user identity
    waterMarkProps: initialState?.currentUser
      ? { content: initialState.currentUser.username }
      : undefined,
    // Redirect unauthenticated access to /login
    onPageChange: () => {
      const token = localStorage.getItem(TOKEN_KEY);
      const { location } = history;
      if (!token && location.pathname !== '/login') {
        history.push('/login');
      }
    },
    // Wrap every page with ErrorBoundary so a single page crash doesn't kill the shell
    childrenRender: (children) => (
      <ErrorBoundary>{children}</ErrorBoundary>
    ),
    ...initialState?.settings,
  };
};

// Global request config — token injection and response envelope unwrapping
export const request: RequestConfig = {
  timeout: 30_000,

  // Add Bearer token to every outgoing request
  requestInterceptors: [
    (config) => {
      const token = localStorage.getItem(TOKEN_KEY);
      if (token) {
        config.headers = Object.assign({}, config.headers, {
          Authorization: `Bearer ${token}`,
        });
      }
      return config;
    },
  ],

  // Unwrap the ApiResponse envelope { success, data, message }
  responseInterceptors: [
    (response) => {
      const data = response.data as API.ApiResponse;

      if (data && typeof data === 'object' && 'success' in data) {
        if (!data.success) {
          // Surface server-side error message to the user
          const errMsg = data.message ?? 'Request failed';
          message.error(errMsg);
          return Promise.reject(new Error(errMsg));
        }
        // Return the inner payload so callers receive data.data directly
        response.data = data.data;
      }

      return response;
    },
  ],

  // Centralised error handling for HTTP-level failures
  errorConfig: {
    errorHandler: (error: unknown) => {
      const err = error as {
        response?: { status?: number; data?: { error?: string; message?: string } };
        message?: string;
      };
      const status = err?.response?.status;
      const serverMsg = err?.response?.data?.error ?? err?.response?.data?.message;

      if (status === 401) {
        // Session expired — clear credentials and redirect to login
        localStorage.removeItem(TOKEN_KEY);
        history.push('/login');
        message.warning('Session expired. Please log in again.');
        return;
      }

      if (status === 429) {
        message.error('Rate limit exceeded. Please slow down your requests.');
        return;
      }

      if (serverMsg) {
        message.error(serverMsg);
        return;
      }

      // Generic fallback error
      message.error('An unexpected error occurred. Please try again.');
    },
  },
};
