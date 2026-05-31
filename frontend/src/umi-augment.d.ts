// Module augmentation for `@umijs/max`.
//
// At runtime the umi build pipeline rewires `import … from '@umijs/max'`
// to `src/.umi/exports.ts` (auto-generated, marked `// @ts-nocheck`),
// which re-exports the hooks contributed by every umi plugin (access,
// model, request, renderer-react, etc.). The published `@umijs/max`
// .d.ts only forwards `umi/dist/index.d.ts`, which is the *build* surface
// — none of the runtime hooks (`useRequest`, `useAccess`, `history`, …)
// are typed there.
//
// This file fills the gap so editor / `tsc --noEmit` see the same surface
// the runtime exposes. We deliberately stay loose on inner shapes (the
// caller already declares concrete types where needed); the goal is to
// unblock strict-mode typechecking without re-typing every plugin.

import type { ReactNode, ReactElement, ComponentType } from 'react';

declare module '@umijs/max' {
  // ── @umijs/plugin-access ──────────────────────────────────────────────
  export const Access: ComponentType<{
    accessible: boolean | string;
    fallback?: ReactNode;
    children?: ReactNode;
  }>;
  export function useAccess(): Record<string, boolean>;

  // ── @umijs/plugin-model ───────────────────────────────────────────────
  // The model namespace is freely-shaped per-app (each app registers its
  // own models). Default to `any` so destructuring, property access, and
  // chained method calls (e.g. `snapshots.filter(s => …)`) typecheck under
  // strict mode without forcing every call-site to annotate. Callers that
  // want stronger guarantees can pass an explicit type argument.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export function useModel<T = any>(namespace: string): T;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export function useModel<T = any, S = unknown>(
    namespace: string,
    selector: (model: T) => S,
  ): S;
  export const Provider: ComponentType<{ children?: ReactNode }>;

  // ── @umijs/plugin-request ─────────────────────────────────────────────
  export interface UseRequestResult<T> {
    data?: T;
    error?: Error;
    loading: boolean;
    refresh: () => void;
    refreshAsync: () => Promise<T>;
    run: (...args: unknown[]) => Promise<T>;
    runAsync: (...args: unknown[]) => Promise<T>;
    mutate: (data?: T | ((oldData?: T) => T | undefined)) => void;
    cancel: () => void;
    params: unknown[];
  }

  export interface UseRequestOptions<T> {
    manual?: boolean;
    ready?: boolean;
    defaultParams?: unknown[];
    refreshDeps?: unknown[];
    pollingInterval?: number;
    pollingWhenHidden?: boolean;
    pollingErrorRetryCount?: number;
    debounceWait?: number;
    throttleWait?: number;
    cacheKey?: string;
    cacheTime?: number;
    staleTime?: number;
    refreshOnWindowFocus?: boolean;
    focusTimespan?: number;
    onSuccess?: (data: T, params: unknown[]) => void;
    onError?: (error: Error, params: unknown[]) => void;
    onBefore?: (params: unknown[]) => void;
    onFinally?: (params: unknown[], data?: T, error?: Error) => void;
    formatResult?: (res: unknown) => T;
    [key: string]: unknown;
  }

  export function useRequest<T = unknown>(
    service: ((...args: any[]) => Promise<T>) | string,
    options?: UseRequestOptions<T>,
  ): UseRequestResult<T>;

  export function request<T = unknown>(
    url: string,
    options?: Record<string, unknown>,
  ): Promise<T>;

  // axios-like request/response shapes — kept loose because individual
  // interceptors freely mutate `config.headers`, etc.
  export interface RequestConfigShape {
    url?: string;
    method?: string;
    headers?: Record<string, string>;
    params?: Record<string, unknown>;
    data?: unknown;
    [key: string]: unknown;
  }
  export interface RequestResponseShape {
    data: unknown;
    status: number;
    statusText: string;
    headers: Record<string, string>;
    config: RequestConfigShape;
    [key: string]: unknown;
  }

  export interface RequestConfig {
    timeout?: number;
    errorConfig?: {
      errorThrower?: (res: unknown) => void;
      errorHandler?: (error: unknown, opts?: unknown) => void;
    };
    requestInterceptors?: Array<
      | ((config: RequestConfigShape) => RequestConfigShape | Promise<RequestConfigShape>)
      | { fulfilled?: (config: RequestConfigShape) => RequestConfigShape; rejected?: (error: unknown) => unknown }
    >;
    responseInterceptors?: Array<
      | ((response: RequestResponseShape) => RequestResponseShape | Promise<RequestResponseShape>)
      | { fulfilled?: (response: RequestResponseShape) => RequestResponseShape; rejected?: (error: unknown) => unknown }
    >;
    [key: string]: unknown;
  }

  // ── @umijs/renderer-react ─────────────────────────────────────────────
  export function useParams<T extends Record<string, string | undefined> = Record<string, string | undefined>>(): T;
  export function useLocation<T = unknown>(): { pathname: string; search: string; hash: string; state: T; key: string };
  export function useNavigate(): (
    to: string | number,
    options?: { replace?: boolean; state?: unknown },
  ) => void;
  export function useSearchParams(): [URLSearchParams, (params: URLSearchParams | Record<string, string>) => void];
  export const Link: ComponentType<{ to: string; replace?: boolean; state?: unknown; children?: ReactNode; className?: string; style?: React.CSSProperties }>;
  export const Navigate: ComponentType<{ to: string; replace?: boolean; state?: unknown }>;
  export const Outlet: ComponentType;

  // ── @@/core/history ───────────────────────────────────────────────────
  export const history: {
    push: (path: string, state?: unknown) => void;
    replace: (path: string, state?: unknown) => void;
    go: (n: number) => void;
    back: () => void;
    forward: () => void;
    location: { pathname: string; search: string; hash: string; state: unknown; key: string };
    listen: (callback: (update: unknown) => void) => () => void;
  };

  // ── @umijs/plugin-layout ──────────────────────────────────────────────
  // The callback receives umi's initial-state wrapper. We type the
  // `initialState` key as the shape `getInitialState()` returns in this
  // app — currentUser + settings — which lets `({ initialState })` destructures
  // and `initialState?.currentUser?.username` access typecheck.
  export type RunTimeLayoutConfig = (params: {
    initialState?: {
      currentUser?: {
        token?: string;
        username?: string;
        role?: string;
      };
      settings?: Record<string, unknown>;
    };
    loading?: boolean;
    setInitialState?: (state: unknown) => void;
  }) => Record<string, unknown>;
}
