// Access control definitions — consumed by the UmiJS access plugin
export default function access(initialState: { currentUser?: API.CurrentUser }) {
  const role = initialState?.currentUser?.role ?? 'viewer';

  return {
    // Full admin access
    isAdmin: role === 'admin',

    // Admin or operator level — can perform operational actions
    isOperator: role === 'admin' || role === 'operator',

    // Every authenticated user can view dashboards
    isViewer: true,

    // Forcibly disconnect a client connection
    canDisconnect: role === 'admin' || role === 'operator',

    // Activate / deactivate / reload plugins
    canPluginLifecycle: role === 'admin' || role === 'operator',

    // Manage traffic-guard rules (block/unblock IPs, adjust rate limits)
    canGuardActions: role === 'admin' || role === 'operator',

    // Trigger a live config reload without restarting the server
    canConfigReload: role === 'admin',

    // Access billing plans and usage data
    canBillingManage: role === 'admin',

    // Flush Redis or in-memory cache
    canCacheFlush: role === 'admin' || role === 'operator',

    // Publish or update plugins on the marketplace
    canPublishPlugin: role === 'admin' || role === 'operator',

    // Create, edit, or delete routing rules
    canRouteManage: role === 'admin' || role === 'operator',

    // Create, edit, or delete admin users
    canUserManage: role === 'admin',
  };
}
