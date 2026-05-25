import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      'draox-sdk-web': path.resolve(__dirname, '../sdk-web/src/index.ts'),
    },
  },
  server: {
    port: 5173,
    open: true,
  },
});
