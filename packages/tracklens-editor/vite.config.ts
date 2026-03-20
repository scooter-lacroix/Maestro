import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/postcss';
import * as fs from 'fs';
import * as path from 'path';

// Custom plugin to copy build artifacts and favicon assets for TrackLens
const copyTracklensAssets = () => ({
  name: 'copy-tracklens-assets',
  closeBundle() {
    const distDir = path.resolve(__dirname, 'dist');
    const indexSource = path.resolve(distDir, 'index.html');
    const sourceFavicon = path.resolve(__dirname, '../../maestro/brand/logos/favicon.svg');
    const distFavicon = path.resolve(distDir, 'favicon.svg');
    const appHtmlTarget = path.resolve(__dirname, '../../apps/tracklens-opencode/tracklens.html');
    const appFaviconTarget = path.resolve(__dirname, '../../apps/tracklens-opencode/favicon.svg');
    const reviewEditorFaviconTarget = path.resolve(
      __dirname,
      '../../packages/tracklens-review-editor/dist/favicon.svg',
    );

    if (fs.existsSync(sourceFavicon)) {
      fs.mkdirSync(path.dirname(distFavicon), { recursive: true });
      fs.copyFileSync(sourceFavicon, distFavicon);
      console.log(`Copied favicon: ${sourceFavicon} -> ${distFavicon}`);
    }

    if (fs.existsSync(indexSource)) {
      fs.mkdirSync(path.dirname(appHtmlTarget), { recursive: true });
      fs.copyFileSync(indexSource, appHtmlTarget);
      console.log(`Copied ${indexSource} -> ${appHtmlTarget}`);
    }

    if (fs.existsSync(distFavicon)) {
      fs.mkdirSync(path.dirname(appFaviconTarget), { recursive: true });
      fs.copyFileSync(distFavicon, appFaviconTarget);
      console.log(`Copied ${distFavicon} -> ${appFaviconTarget}`);

      if (fs.existsSync(path.dirname(reviewEditorFaviconTarget))) {
        fs.copyFileSync(distFavicon, reviewEditorFaviconTarget);
        console.log(`Copied ${distFavicon} -> ${reviewEditorFaviconTarget}`);
      }
    }
  },
});

export default defineConfig({
  plugins: [react(), copyTracklensAssets()],
  css: {
    postcss: {
      plugins: [tailwindcss()],
    },
  },
  build: {
    outDir: 'dist',
  },
});
