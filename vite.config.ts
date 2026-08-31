import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  build: {
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL('./index.html', import.meta.url)),
        hotzone: fileURLToPath(new URL('./hotzone.html', import.meta.url)),
        panel: fileURLToPath(new URL('./panel.html', import.meta.url)),
        pinned: fileURLToPath(new URL('./pinned.html', import.meta.url)),
        reminder: fileURLToPath(new URL('./reminder.html', import.meta.url)),
      },
    },
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**', '**/target/**', '**/.git/**'],
    },
  },
})
