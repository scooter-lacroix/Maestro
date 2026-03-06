import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { viteSingleFile } from 'vite-plugin-singlefile';
import tailwindcss from '@tailwindcss/postcss';
import * as fs from 'fs';
import * as path from 'path';

// Custom plugin to copy built HTML to apps/tracklens-opencode
const copyToApps = () => ({
  name: 'copy-to-apps',
  closeBundle() {
    const source = path.resolve(__dirname, 'dist/index.html');
    const target = path.resolve(__dirname, '../../apps/tracklens-opencode/tracklens-review.html');

    if (fs.existsSync(source)) {
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.copyFileSync(source, target);
      console.log(`Copied ${source} -> ${target}`);
    }
  },
});

export default defineConfig({
  plugins: [react(), viteSingleFile(), copyToApps()],
  css: {
    postcss: {
      plugins: [tailwindcss()],
    },
  },
  build: {
    outDir: "dist",
    sourcemap: true,
    // Create a single-file bundle with all assets inlined
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
      },
    },
  },
  server: {
    port: 3000,
  },
});
