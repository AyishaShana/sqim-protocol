import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  build: {
    chunkSizeWarningLimit: 900,
    rollupOptions: {
      output: {
        manualChunks(id) {
          const stellarPackage = id.match(/node_modules\/@stellar\/([^/]+)/);
          if (stellarPackage) return `stellar-${stellarPackage[1]}`;
          if (id.includes("node_modules/axios/")) return "stellar-transport";
          if (id.includes("node_modules/js-xdr/")) return "stellar-xdr";
          if (id.includes("lucide-react")) return "icons";
          if (id.includes("react")) return "react";
          return undefined;
        },
      },
    },
  },
  test: {
    environment: "jsdom",
  },
});
