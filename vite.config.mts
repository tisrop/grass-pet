import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    strictPort: true,
  },
  build: {
    outDir: 'dist-vue',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        pet: fileURLToPath(new URL('./index.html', import.meta.url)),
        dashboard: fileURLToPath(new URL('./dashboard.html', import.meta.url)),
        reminder: fileURLToPath(new URL('./reminder.html', import.meta.url)),
        contextMenu: fileURLToPath(new URL('./context-menu.html', import.meta.url)),
      },
    },
  },
});
