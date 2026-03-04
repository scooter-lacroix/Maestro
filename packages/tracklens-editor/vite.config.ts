import { defineConfig } from 'vite';
import { viteSingleFile } from 'vite-plugin-singlefile';

export default defineConfig({
  plugins: [viteSingleFile()],
  build: {
    // Create a single-file bundle with all assets inlined
    // This allows the HTML to be served directly without external asset requests
    inlineDynamicImports: true,
  },
});
