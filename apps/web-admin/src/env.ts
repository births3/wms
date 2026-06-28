/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL?: string;
}

declare const __WMS_WEB_ADMIN_DEV_LOGIN__: {
  enabled: boolean;
  ownerCode: string;
  username: string;
  password: string;
};
