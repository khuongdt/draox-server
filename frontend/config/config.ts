import { defineConfig } from '@umijs/max';
import routes from './routes';
import proxy from './proxy';

// UmiJS 4 / Ant Design Pro 6 main configuration
export default defineConfig({
  // Base application title shown in browser tab
  title: 'Draox Admin',

  // Enable all Umi Max plugins
  antd: {
    // Enable Ant Design 5 dark algorithm via cssVar
    dark: true,
    compact: false,
    configProvider: {
      theme: {
        cssVar: true,
        token: {
          colorPrimary: '#e05d10',
          colorBgBase: '#1a1a2e',
          colorBgContainer: '#16213e',
          colorBgElevated: '#0f3460',
          colorBorder: '#2a2a4a',
          colorText: '#e0e0e0',
          colorTextSecondary: '#a0a0b0',
          colorTextTertiary: '#707090',
          borderRadius: 6,
          fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
        },
      },
    },
  },

  // Access plugin for RBAC
  access: {},

  // Model plugin for global state management
  model: {},

  // Initial state plugin — provides getInitialState to app.tsx
  initialState: {},

  // Request plugin wrapping axios
  request: {},

  // Layout plugin using ProLayout
  layout: {
    title: 'Draox Admin',
    locale: true,
  },

  // Locale plugin: default English, also supports Vietnamese
  locale: {
    default: 'vi-VN',
    antd: true,
    title: false,
    baseNavigator: true,
    baseSeparator: '-',
  },

  // Routes definition
  routes,

  // Dev proxy to Admin API on port 9100
  proxy: proxy.dev,

  // Enable hash-based routing for static deployment compatibility
  hash: true,

  // Code splitting strategy
  codeSplitting: {
    jsStrategy: 'granularChunks',
  },

  // Less variables for global style
  lessLoader: {
    modifyVars: {
      'primary-color': '#e05d10',
    },
    javascriptEnabled: true,
  },
  esbuildMinifyIIFE: true
});
