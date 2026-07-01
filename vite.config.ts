import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      'Inconsolata-Regular.v1.woff2': path.resolve(__dirname, 'public/Inconsolata-Regular.v1.woff2'),
      'Inconsolata-Regular.v1.woff': path.resolve(__dirname, 'public/Inconsolata-Regular.v1.woff')
    }
  },
  build: {
    assetsDir: 'aux',
    rollupOptions: {
      output: {
        entryFileNames: 'aux/[name]-[hash].js',
        chunkFileNames: 'aux/[name]-[hash].js',
        assetFileNames: (assetInfo) => {
          if (assetInfo.name) {
            if (assetInfo.name.endsWith('.css')) return 'aux/[name]-[hash].css';            
            if (assetInfo.name.endsWith('.woff2') || assetInfo.name.endsWith('.woff')) {
              return 'aux/[name].[ext]'; 
            }
          }
          return 'aux/[name]-[hash].[ext]';
        }
      }
    }
  }
});
