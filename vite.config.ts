import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { resolve } from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  server: {
    watch: {
      ignored: ['**/.venv/**', '**/build/**', '**/dist/burgonet-sidecar/**'],
    },
  },
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        flash: resolve(__dirname, 'flash.html'),
      },
    },
  },
})
