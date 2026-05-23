// Route definitions for Draox Admin dashboard
// Note: UmiJS lazy-loads all route components by default with dynamic imports.
// Chart-heavy pages (Metrics, Marketplace/Detail, Dashboard) are naturally code-split.
export default [
  { path: '/login', layout: false, component: './Login' },
  { path: '/', redirect: '/dashboard' },
  {
    path: '/dashboard',
    name: 'Dashboard',
    icon: 'DashboardOutlined',
    component: './Dashboard',
  },
  {
    path: '/connections',
    name: 'Connections',
    icon: 'ApiOutlined',
    routes: [
      { path: '/connections', component: './Connections', hideInMenu: true },
      { path: '/connections/:id', component: './Connections/Detail', hideInMenu: true },
    ],
  },
  {
    path: '/sessions',
    name: 'Sessions',
    icon: 'TeamOutlined',
    routes: [
      { path: '/sessions', component: './Sessions', hideInMenu: true },
      { path: '/sessions/:id', component: './Sessions/Detail', hideInMenu: true },
    ],
  },
  {
    path: '/plugins',
    name: 'Plugins',
    icon: 'AppstoreOutlined',
    routes: [
      { path: '/plugins', component: './Plugins', hideInMenu: true },
      { path: '/plugins/:id', component: './Plugins/Detail', hideInMenu: true },
    ],
  },
  {
    name: 'Traffic Guard',
    icon: 'SafetyOutlined',
    path: '/traffic-guard',
    routes: [
      { path: '/traffic-guard', name: 'Overview', component: './TrafficGuard' },
      { path: '/traffic-guard/reputation', name: 'IP Reputation', component: './TrafficGuard/Reputation' },
    ],
  },
  {
    path: '/metrics',
    name: 'Metrics',
    icon: 'LineChartOutlined',
    component: './Metrics',
  },
  {
    name: 'Marketplace',
    icon: 'ShopOutlined',
    path: '/marketplace',
    routes: [
      { path: '/marketplace', name: 'Browse', component: './Marketplace', hideInMenu: true },
      { path: '/marketplace/publish', name: 'Publish', component: './Marketplace/Publish' },
      { path: '/marketplace/:id', component: './Marketplace/Detail', hideInMenu: true },
    ],
  },
  {
    name: 'Billing',
    icon: 'DollarOutlined',
    path: '/billing',
    access: 'isAdmin',
    routes: [
      { path: '/billing/plans', name: 'Plans', component: './Billing/Plans' },
      { path: '/billing/usage', name: 'Usage', component: './Billing/Usage' },
    ],
  },
  {
    path: '/cache',
    name: 'Cache',
    icon: 'DatabaseOutlined',
    component: './Cache',
  },
  {
    path: '/config',
    name: 'Config',
    icon: 'SettingOutlined',
    component: './Config',
    access: 'isAdmin',
  },
  {
    name: 'Audit Log',
    icon: 'FileSearchOutlined',
    path: '/audit',
    routes: [
      { path: '/audit', component: './Audit', hideInMenu: true },
      { path: '/audit/:id', component: './Audit/Detail', hideInMenu: true },
    ],
  },
  {
    path: '/routes',
    name: 'Routes',
    icon: 'NodeIndexOutlined',
    component: './Routes',
  },
  {
    path: '/events',
    name: 'Event Stream',
    icon: 'ThunderboltOutlined',
    component: './EventStream',
  },
  {
    path: '/users',
    name: 'Users',
    icon: 'UserOutlined',
    component: './Users',
    access: 'isAdmin',
  },
];
