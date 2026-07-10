import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

import { devLoginDefaults, webAdminDevMock } from "./dev-mocks/web-admin-dev-mock";

const e2eApiUrl = process.env.WMS_WEB_ADMIN_E2E_API_URL?.trim();
const devApiProxyUrl = e2eApiUrl || process.env.VITE_API_BASE_URL?.trim();

export default defineConfig(({ command }) => {
  const devLoginEnabled = command === "serve" && process.env.WMS_WEB_ADMIN_DEV_LOGIN !== "0";

  return {
    define: {
      __WMS_WEB_ADMIN_DEV_LOGIN__: JSON.stringify(devLoginDefaults(devLoginEnabled)),
    },
    plugins: [react(), webAdminDevMock()],
    resolve: {
      dedupe: ["jquery"],
      alias: {
        "@": path.resolve(__dirname, "./src"),
      },
    },
    server: {
      host: "0.0.0.0",
      port: 9002,
      strictPort: true,
      proxy: devApiProxyUrl
        ? {
            "/api": {
              target: devApiProxyUrl,
              changeOrigin: true,
            },
            "/api-docs": {
              target: devApiProxyUrl,
              changeOrigin: true,
            },
            "/openapi.json": {
              target: devApiProxyUrl,
              changeOrigin: true,
            },
            "/redoc": {
              target: devApiProxyUrl,
              changeOrigin: true,
            },
          }
        : undefined,
    },
  };
});
