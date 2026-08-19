import { createApiClient, putBinary } from "@wms/api-client";

import { readAccessToken } from "./auth-session";

export const apiBaseUrl = import.meta.env.VITE_API_BASE_URL ?? "";

export const api = createApiClient({
  baseUrl: apiBaseUrl,
  authToken: readAccessToken,
});

export function putApiBinary(url: string, file: File) {
  return putBinary({
    baseUrl: apiBaseUrl,
    url,
    contentType: file.type,
    body: file,
  });
}
