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

// Custom plugin to copy favicon.svg for TrackLens
const copyFavicon = () => ({
  name: 'copy-favicon',
  closeBundle() {
    // Source favicon location
    const faviconSource = path.resolve(__dirname, 'dist/favicon.svg');

    // Don't copy if it doesn't exist (e.g., during dev)
    if (!fs.existsSync(faviconSource)) {
      return;
    }

    // Copy to tracklens-editor dist (already there from build)
    // Copy to apps/tracklens-opencode
    const target = path.resolve(__dirname, '../../apps/tracklens-opencode/favicon.svg');
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.copyFileSync(faviconSource, target);
    console.log(`Copied ${faviconSource} -> ${target}`);

    // Also copy to packages/tracklens-review-editor if it exists
    const reviewEditorTarget = path.resolve(__dirname, '../../packages/tracklens-review-editor/dist/favicon.svg');
    if (fs.existsSync(path.dirname(reviewEditorTarget))) {
      fs.copyFileSync(faviconSource, reviewEditorTarget);
      console.log(`Copied ${faviconSource} -> ${reviewEditorTarget}`);
    }
  },
});

// Custom plugin to copy favicon from source to dist during build
const copyFaviconFromSource = () => ({
  name: 'copy-favicon-source',
  closeBundle() {
    const sourceFavicon = path.resolve(__dirname, '../../maestro/brand/logos/favicon.svg');
    const destFavicon = path.resolve(__dirname, 'dist/favicon.svg');

    if (fs.existsSync(sourceFavicon)) {
      fs.mkdirSync(path.dirname(destFavicon), { recursive: true });
      fs.copyFileSync(sourceFavicon, destFavicon);
      console.log(`Copied favicon: ${sourceFavicon} -> ${destFavicon}`);
    }
  },
});

export default defineConfig({
  plugins: [react(), copyFaviconFromSource(), copyToApps(), copyFavicon()],
  css: {
    postcss: {
      plugins: [tailwindcss()],
    },
  },
  build: {
    outDir: 'dist',
  },
});
