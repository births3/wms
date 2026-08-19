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

/** 仅 dev server（vite serve）为 true：控制演示性表单预填，生产构建一律为空值。 */
declare const __WMS_WEB_ADMIN_DEV_PREFILL__: boolean;
