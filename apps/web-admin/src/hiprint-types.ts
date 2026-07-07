/// <reference types="vite/client" />

declare module "hiprint" {
  export function disAutoConnect(): void;
  export const defaultElementTypeProvider: new () => unknown;
  export const hiprint: {
    init(options?: { providers?: unknown[] }): void;
    PrintElementTypeManager?: {
      buildByHtml(elements: unknown): void;
    };
    PrintTemplate: new (options?: {
      template?: unknown;
      settingContainer?: string | HTMLElement;
      paginationContainer?: string | HTMLElement;
    }) => {
      design(target: string | HTMLElement): void;
      getJson(): unknown;
      getHtml(data?: unknown): { get(index: number): HTMLElement | undefined; length: number };
      print(data?: unknown): void;
    };
  };
}

declare module "jquery" {
  interface JQueryLike {
    find(selector: string): JQueryLike;
  }
  const jquery: (target: unknown) => JQueryLike;
  export default jquery;
}
