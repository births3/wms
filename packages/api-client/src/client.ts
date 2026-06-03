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
