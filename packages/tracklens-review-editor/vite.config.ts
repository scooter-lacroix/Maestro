import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { viteSingleFile } from 'vite-plugin-singlefile';

export default defineConfig({
  plugins: [react(), viteSingleFile()],
  build: {
    outDir: "dist",
    sourcemap: true,
    // Create a single-file bundle with all assets inlined
    inlineDynamicImports: true,
  },
  server: {
    port: 3000,
  },
});
