import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/postcss';
import * as fs from 'fs';
import * as path from 'path';

// Custom plugin to copy built HTML to apps/tracklens-opencode
const copyToApps = () => ({
  name: 'copy-to-apps',
  closeBundle() {
    const source = path.resolve(__dirname, 'dist/index.html');
    const target = path.resolve(__dirname, '../../apps/tracklens-opencode/tracklens.html');

    if (fs.existsSync(source)) {
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.copyFileSync(source, target);
      console.log(`Copied ${source} -> ${target}`);
    }
  },
});

export default defineConfig({
  plugins: [react(), copyToApps()],
  css: {
    postcss: {
      plugins: [tailwindcss()],
    },
  },
  build: {
    outDir: 'dist',
  },
});
