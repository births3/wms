/** 统一错误转文案：Error 取 message，其余类型用调用方给的兜底文案。 */
export function errorText(error: unknown, fallback: string): string {
  return error instanceof Error && error.message ? error.message : fallback;
}
