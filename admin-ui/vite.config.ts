import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";

const adminUiOutDir = process.env.MADLIBRARY_ADMIN_UI_OUT_DIR ?? "dist";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: "/admin/",
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url))
    }
  },
  server: {
    strictPort: true,
    proxy: {
      "/api": "http://127.0.0.1:3789",
      "/health": "http://127.0.0.1:3789"
    }
  },
  preview: {
    strictPort: true
  },
  build: {
    outDir: adminUiOutDir,
    emptyOutDir: true
  }
});
