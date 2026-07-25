import createClient from "openapi-fetch";

import type { paths } from "./schema";

export type ApiClient = ReturnType<typeof createClient<paths>>;

export function createApiClient(opts: {
  baseUrl: string;
  authToken?: () => string | null;
}): ApiClient {
  return createClient<paths>({
    baseUrl: opts.baseUrl,
    fetch: async (input: Request) => {
      const headers = new Headers(input.headers);
      const token = opts.authToken?.();

      if (token) {
        headers.set("Authorization", `Bearer ${token}`);
      }

      return fetch(new Request(input, { headers }));
    },
  });
}

export function putBinary(opts: {
  baseUrl: string;
  url: string;
  contentType: string;
  body: Blob;
}): Promise<Response> {
  const target = /^https?:\/\//i.test(opts.url)
    ? opts.url
    : `${opts.baseUrl.replace(/\/$/, "")}${opts.url}`;
  return fetch(target, {
    method: "PUT",
    headers: { "Content-Type": opts.contentType },
    body: opts.body,
  });
}

export class JsonApiError extends Error {
  readonly status: number;
  readonly code?: string;

  constructor(message: string, status: number, code?: string) {
    super(message);
    this.name = "JsonApiError";
    this.status = status;
    this.code = code;
  }
}

/** 为独立子应用提供不依赖 WMS OpenAPI paths 的统一 JSON 请求边界。 */
export async function requestJson<T>(opts: {
  baseUrl: string;
  path: string;
  method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  authToken?: string | null;
  body?: unknown;
}): Promise<T> {
  const target = `${opts.baseUrl.replace(/\/$/, "")}${opts.path}`;
  const headers = new Headers({ Accept: "application/json" });
  if (opts.authToken) {
    headers.set("Authorization", `Bearer ${opts.authToken}`);
  }
  if (opts.body !== undefined) {
    headers.set("Content-Type", "application/json");
  }
  const response = await fetch(target, {
    method: opts.method ?? "GET",
    headers,
    body: opts.body === undefined ? undefined : JSON.stringify(opts.body),
  });
  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as
      | { code?: string; message?: string }
      | null;
    throw new JsonApiError(
      payload?.message ?? `请求失败（HTTP ${response.status}）`,
      response.status,
      payload?.code,
    );
  }
  return (await response.json()) as T;
}
