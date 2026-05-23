// Dev proxy config — forwards /api and /ws to the Draox Admin API on port 9100
export default {
  dev: {
    '/api': {
      target: 'http://localhost:9100',
      changeOrigin: true,
    },
    '/ws': {
      target: 'ws://localhost:9100',
      ws: true,
      changeOrigin: true,
    },
  },
};
